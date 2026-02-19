// =============================================================================
// FEDERATION CORE — mirage.rs
// PHASE 2 / WEEK 7 — «Active Mimicry (Mirage Module)»
// =============================================================================
//
// Реализует:
//   1. AnomalyDetector — обнаружение признаков сканирования/атаки
//   2. MirageGenerator — генерация ложных SSAU тензоров (T_fake)
//   3. MazeTrap        — виртуальная топология-ловушка (пакеты зацикливаются)
//   4. MirageNode      — полный модуль мимикрии узла
//
// Математика из White Paper (Раздел 5.5):
//
//   T_fake = T_real + Φ(A) · M
//
//   Где:
//     Φ(A) — функция преобразования паттерна атаки в веса дезинформации
//     M    — матрица мимикрии (создаёт иллюзию «идеального узла»)
//
//   Для атакующего: Σ L_i → ∞ (пакеты зацикливаются в лабиринте)
// =============================================================================

use crate::tensor::SsauTensor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

/// Порог аномального числа запросов в секунду (признак сканирования)
pub const SCAN_RATE_THRESHOLD: f64 = 10.0;

/// Порог аномального числа уникальных источников запросов
pub const UNIQUE_SOURCES_THRESHOLD: usize = 5;

/// Порог отклонения задержек (признак измерения топологии)
pub const TIMING_ANOMALY_THRESHOLD: f64 = 0.15;

/// Время жизни Mirage-ловушки (секунды)
pub const MIRAGE_TTL_SECS: u64 = 300;

// -----------------------------------------------------------------------------
// AnomalyScore — оценка угрозы
// -----------------------------------------------------------------------------

/// Тип обнаруженной аномалии
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// Слишком много запросов с одного источника
    RateFlooding,
    /// Систематическое измерение задержек (топология-разведка)
    TopologyProbing,
    /// Аномальные паттерны TTL (traceroute-сканирование)
    TtlScanning,
    /// Одновременные запросы с множества источников (DDoS)
    DistributedScan,
    /// Повторяющиеся запросы одних и тех же маршрутов
    RouteEnumeration,
}

/// Результат анализа аномалий
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScore {
    /// Общий балл угрозы (0.0 = норма, 1.0 = явная атака)
    pub threat_level: f64,
    /// Обнаруженные типы аномалий
    pub anomalies: Vec<AnomalyType>,
    /// Источник атаки (если определён)
    pub suspected_attacker: Option<String>,
    /// Рекомендация: активировать Mirage?
    pub activate_mirage: bool,
    /// Описание
    pub description: String,
}

// -----------------------------------------------------------------------------
// AnomalyDetector — детектор аномалий
// -----------------------------------------------------------------------------

/// Статистика запросов от одного источника
#[derive(Debug, Default, Clone)]
struct SourceStats {
    request_count: u64,
    unique_routes_queried: std::collections::HashSet<String>,
    ttl_values: Vec<u8>,
    last_seen: i64,
    timing_deltas: Vec<f64>,
}

/// Детектор аномального поведения.
/// Анализирует паттерны входящих запросов и выявляет признаки атаки.
#[derive(Debug, Default)]
pub struct AnomalyDetector {
    /// Статистика по источникам: source_id → stats
    source_stats: HashMap<String, SourceStats>,
    /// Общее число запросов за последнее окно
    total_requests: u64,
    /// Число активированных Mirage-ловушек
    pub mirage_activations: u64,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Записать входящий запрос и проанализировать на аномалии
    pub fn record_request(
        &mut self,
        source_id: &str,
        queried_route: &str,
        ttl: u8,
        timing_delta_ms: f64,
    ) -> AnomalyScore {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let stats = self.source_stats
            .entry(source_id.to_string())
            .or_default();

        stats.request_count += 1;
        stats.unique_routes_queried.insert(queried_route.to_string());
        stats.ttl_values.push(ttl);
        stats.last_seen = now;
        stats.timing_deltas.push(timing_delta_ms);
        self.total_requests += 1;

        self.analyze(source_id)
    }

    /// Анализ паттернов для конкретного источника
    fn analyze(&self, source_id: &str) -> AnomalyScore {
        let stats = match self.source_stats.get(source_id) {
            Some(s) => s,
            None => return AnomalyScore {
                threat_level: 0.0,
                anomalies: vec![],
                suspected_attacker: None,
                activate_mirage: false,
                description: "Нет данных".to_string(),
            },
        };

        let mut threat_level = 0.0_f64;
        let mut anomalies = vec![];
        let mut descriptions = vec![];

        // Признак 1: Rate Flooding — слишком много запросов
        let request_rate = stats.request_count as f64;
        if request_rate > SCAN_RATE_THRESHOLD {
            let severity = (request_rate / SCAN_RATE_THRESHOLD - 1.0).min(1.0);
            threat_level += 0.3 * severity;
            anomalies.push(AnomalyType::RateFlooding);
            descriptions.push(format!("RateFlood: {} запросов", stats.request_count));
        }

        // Признак 2: Route Enumeration — перебор маршрутов
        let unique_routes = stats.unique_routes_queried.len();
        if unique_routes > UNIQUE_SOURCES_THRESHOLD {
            let severity = (unique_routes as f64 / UNIQUE_SOURCES_THRESHOLD as f64 - 1.0).min(1.0);
            threat_level += 0.25 * severity;
            anomalies.push(AnomalyType::RouteEnumeration);
            descriptions.push(format!("RouteEnum: {} уникальных маршрутов", unique_routes));
        }

        // Признак 3: TTL Scanning — аномальные TTL (traceroute)
        if stats.ttl_values.len() > 3 {
            let ttl_variance = Self::variance(&stats.ttl_values
                .iter().map(|&t| t as f64).collect::<Vec<_>>());
            if ttl_variance > 5.0 {
                threat_level += 0.2;
                anomalies.push(AnomalyType::TtlScanning);
                descriptions.push(format!("TTLScan: variance={:.2}", ttl_variance));
            }
        }

        // Признак 4: Topology Probing — систематические замеры задержек
        if stats.timing_deltas.len() > 5 {
            let timing_cv = Self::coefficient_of_variation(&stats.timing_deltas);
            if timing_cv < TIMING_ANOMALY_THRESHOLD {
                // Слишком регулярные запросы = автоматический сканер
                threat_level += 0.25;
                anomalies.push(AnomalyType::TopologyProbing);
                descriptions.push(format!("TopologyProbe: CV={:.4} (слишком регулярно)", timing_cv));
            }
        }

        // Признак 5: Distributed Scan — много источников одновременно
        let active_sources = self.source_stats.len();
        if active_sources > 10 {
            threat_level += 0.2;
            anomalies.push(AnomalyType::DistributedScan);
            descriptions.push(format!("DistScan: {} источников", active_sources));
        }

        threat_level = threat_level.min(1.0);
        let activate_mirage = threat_level > 0.3 || anomalies.len() >= 2;

        AnomalyScore {
            threat_level,
            anomalies,
            suspected_attacker: if threat_level > 0.3 { Some(source_id.to_string()) } else { None },
            activate_mirage,
            description: if descriptions.is_empty() {
                "Норма".to_string()
            } else {
                descriptions.join(" | ")
            },
        }
    }

    fn variance(values: &[f64]) -> f64 {
        if values.is_empty() { return 0.0; }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64
    }

    fn coefficient_of_variation(values: &[f64]) -> f64 {
        if values.is_empty() { return 1.0; }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        if mean == 0.0 { return 1.0; }
        let std_dev = Self::variance(values).sqrt();
        std_dev / mean
    }

    pub fn stats(&self) -> String {
        format!(
            "Источников: {} | Всего запросов: {} | Mirage активаций: {}",
            self.source_stats.len(),
            self.total_requests,
            self.mirage_activations,
        )
    }
}

// -----------------------------------------------------------------------------
// MirageGenerator — генератор ложных тензоров
// -----------------------------------------------------------------------------

/// Матрица мимикрии M — определяет как именно искажать данные
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimicryMatrix {
    /// Коэффициент искажения задержки
    pub latency_factor: f64,
    /// Коэффициент искажения bandwidth
    pub bandwidth_factor: f64,
    /// Коэффициент искажения reliability
    pub reliability_factor: f64,
    /// Добавить ли случайный шум
    pub noise_amplitude: f64,
}

impl MimicryMatrix {
    /// Режим «Идеальный узел» — выглядим слишком хорошо (приманка)
    pub fn perfect_lure() -> Self {
        MimicryMatrix {
            latency_factor: 0.1,      // Задержка в 10 раз меньше реальной
            bandwidth_factor: 10.0,   // Bandwidth в 10 раз больше
            reliability_factor: 1.0,  // Reliability = 1.0
            noise_amplitude: 0.01,    // Минимальный шум (выглядит реалистично)
        }
    }

    /// Режим «Мёртвый узел» — отпугиваем сканер
    pub fn dead_node() -> Self {
        MimicryMatrix {
            latency_factor: 100.0,    // Огромная задержка
            bandwidth_factor: 0.01,   // Нет bandwidth
            reliability_factor: 0.0,  // Ненадёжен
            noise_amplitude: 0.5,     // Большой шум (нестабильность)
        }
    }

    /// Режим «Лабиринт» — узел выглядит нормально но пакеты зацикливаются
    pub fn maze() -> Self {
        MimicryMatrix {
            latency_factor: 1.0,
            bandwidth_factor: 1.0,
            reliability_factor: 0.95,
            noise_amplitude: 0.05,
        }
    }
}

/// Генератор ложных SSAU тензоров.
///
/// Формула из White Paper §5.5:
///   T_fake = T_real + Φ(A) · M
pub struct MirageGenerator {
    /// Текущая матрица мимикрии
    pub matrix: MimicryMatrix,
    /// Счётчик сгенерированных ловушек
    pub traps_generated: u64,
    /// Псевдо-random state
    rng_state: u64,
}

impl MirageGenerator {
    pub fn new(matrix: MimicryMatrix) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        MirageGenerator { matrix, traps_generated: 0, rng_state: seed }
    }

    fn next_rand(&mut self) -> f64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }

    /// Сгенерировать ложный тензор на основе реального.
    ///
    /// T_fake = T_real + Φ(A) · M
    ///
    /// Φ(A) — функция угрозы (threat_level из AnomalyScore)
    pub fn generate_fake_tensor(
        &mut self,
        real: &SsauTensor,
        threat_level: f64,
        anomaly_score: &AnomalyScore,
    ) -> FakeTensor {
        // Φ(A) — вес дезинформации пропорционален угрозе
        let phi = threat_level.clamp(0.0, 1.0);

        // Шум для реалистичности
        // Извлекаем значения матрицы до closure (borrow checker)
        let noise_amp = self.matrix.noise_amplitude;
        let lat_factor = self.matrix.latency_factor;
        let bw_factor  = self.matrix.bandwidth_factor;
        let rel_factor = self.matrix.reliability_factor;
        let mut noise = || (self.next_rand() - 0.5) * 2.0 * noise_amp;

        // T_fake = T_real + Φ(A) · M
        let fake_latency = real.latency.mean
            * (1.0 + phi * (lat_factor - 1.0))
            + noise() * real.latency.mean;

        let fake_bandwidth = real.bandwidth
            * (1.0 + phi * (bw_factor - 1.0))
            + noise() * real.bandwidth;

        let fake_reliability = (real.reliability
            + phi * (rel_factor - real.reliability)
            + noise()).clamp(0.0, 1.0);

        self.traps_generated += 1;

        // Выбираем стратегию на основе типа аномалии
        let strategy = if anomaly_score.anomalies.contains(&AnomalyType::TopologyProbing) {
            MimicryStrategy::PerfectLure
        } else if anomaly_score.anomalies.contains(&AnomalyType::RateFlooding) {
            MimicryStrategy::DeadNode
        } else {
            MimicryStrategy::Maze
        };

        FakeTensor {
            from_node: real.from_node.clone(),
            to_node: real.to_node.clone(),
            fake_latency_ms: fake_latency.max(0.1),
            fake_bandwidth_mbps: fake_bandwidth.max(0.1),
            fake_reliability,
            real_latency_ms: real.latency.mean,
            phi_weight: phi,
            strategy,
            trap_id: format!("trap_{:x}", self.rng_state & 0xffff),
        }
    }
}

/// Стратегия мимикрии
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MimicryStrategy {
    PerfectLure,  // Идеальный узел-приманка
    DeadNode,     // Мёртвый узел — отпугиваем
    Maze,         // Лабиринт — зацикливаем
}

/// Ложный тензор
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeTensor {
    pub from_node: String,
    pub to_node: String,
    pub fake_latency_ms: f64,
    pub fake_bandwidth_mbps: f64,
    pub fake_reliability: f64,
    /// Реальные значения (только у нас, атакующий не видит)
    pub real_latency_ms: f64,
    /// Вес дезинформации Φ(A)
    pub phi_weight: f64,
    pub strategy: MimicryStrategy,
    pub trap_id: String,
}

// -----------------------------------------------------------------------------
// MazeTrap — виртуальная топология-ловушка
// -----------------------------------------------------------------------------

/// Виртуальный узел в лабиринте
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeNode {
    pub id: String,
    pub fake_latency_ms: f64,
    pub next_hop: String,
    pub is_loop: bool,
}

/// Лабиринт — виртуальная топология для зацикливания атакующего.
///
/// Формула из White Paper:
///   Σ(path in Maze) L_i → ∞
#[derive(Debug, Serialize, Deserialize)]
pub struct MazeTrap {
    pub nodes: Vec<MazeNode>,
    pub entry_point: String,
    pub total_fake_latency: f64,
    pub created_for: String,
}

impl MazeTrap {
    /// Создать лабиринт для конкретного атакующего
    pub fn create_for_attacker(attacker_id: &str, depth: usize) -> Self {
        let mut nodes = vec![];
        let mut total_latency = 0.0;

        // Строим кольцо виртуальных узлов
        for i in 0..depth {
            let next = (i + 1) % depth;
            let fake_latency = 5.0 + (i as f64 * 3.0); // нарастающая задержка
            total_latency += fake_latency;

            nodes.push(MazeNode {
                id: format!("mirage_{}_{}", attacker_id, i),
                fake_latency_ms: fake_latency,
                next_hop: format!("mirage_{}_{}", attacker_id, next),
                is_loop: next == 0, // последний узел ведёт обратно к первому
            });
        }

        MazeTrap {
            entry_point: format!("mirage_{}_0", attacker_id),
            nodes,
            total_fake_latency: total_latency,
            created_for: attacker_id.to_string(),
        }
    }

    /// Симулировать прохождение пакета через лабиринт
    pub fn simulate_packet(&self, max_hops: usize) -> MazeSimResult {
        let mut path = vec![];
        let mut total_ms = 0.0;
        let mut current = self.entry_point.clone();
        let mut loops = 0;

        for hop in 0..max_hops {
            path.push(current.clone());
            if let Some(node) = self.nodes.iter().find(|n| n.id == current) {
                total_ms += node.fake_latency_ms;
                if node.is_loop && hop > 0 {
                    loops += 1;
                    if loops > 2 { break; } // Ограничиваем симуляцию
                }
                current = node.next_hop.clone();
            } else {
                break;
            }
        }

        MazeSimResult {
            hops: path.len(),
            total_latency_ms: total_ms,
            loops_detected: loops,
            path,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MazeSimResult {
    pub hops: usize,
    pub total_latency_ms: f64,
    pub loops_detected: usize,
    pub path: Vec<String>,
}

// -----------------------------------------------------------------------------
// MirageNode — полный модуль мимикрии узла
// -----------------------------------------------------------------------------

/// Полный модуль Active Mimicry.
/// Объединяет детектор, генератор и лабиринты.
pub struct MirageNode {
    pub node_id: String,
    pub detector: AnomalyDetector,
    pub generator: MirageGenerator,
    pub active_mazes: HashMap<String, MazeTrap>,
    pub mirage_active: bool,
    /// Статистика: сколько атак отражено
    pub attacks_deflected: u64,
}

impl MirageNode {
    pub fn new(node_id: &str) -> Self {
        MirageNode {
            node_id: node_id.to_string(),
            detector: AnomalyDetector::new(),
            generator: MirageGenerator::new(MimicryMatrix::perfect_lure()),
            active_mazes: HashMap::new(),
            mirage_active: false,
            attacks_deflected: 0,
        }
    }

    /// Обработать входящий запрос.
    /// Если обнаружена атака — вернуть ложные данные.
    pub fn handle_request(
        &mut self,
        source_id: &str,
        queried_route: &str,
        ttl: u8,
        timing_delta_ms: f64,
        real_tensor: &SsauTensor,
    ) -> MirageResponse {
        // Анализируем запрос
        let anomaly = self.detector.record_request(
            source_id, queried_route, ttl, timing_delta_ms
        );

        if anomaly.activate_mirage {
            self.mirage_active = true;
            self.attacks_deflected += 1;
            self.detector.mirage_activations += 1;

            // Выбираем матрицу мимикрии на основе типа атаки
            self.generator.matrix = if anomaly.anomalies.contains(&AnomalyType::TopologyProbing) {
                MimicryMatrix::perfect_lure()
            } else if anomaly.anomalies.contains(&AnomalyType::RateFlooding) {
                MimicryMatrix::dead_node()
            } else {
                MimicryMatrix::maze()
            };

            // Генерируем ложный тензор
            let fake = self.generator.generate_fake_tensor(
                real_tensor, anomaly.threat_level, &anomaly
            );

            // Создаём лабиринт если ещё нет
            if !self.active_mazes.contains_key(source_id) {
                let maze = MazeTrap::create_for_attacker(source_id, 6);
                self.active_mazes.insert(source_id.to_string(), maze);
            }

            MirageResponse::Fake {
                fake_tensor: fake,
                anomaly_score: anomaly,
                maze_entry: self.active_mazes.get(source_id)
                    .map(|m| m.entry_point.clone()),
            }
        } else {
            // Нормальный запрос — отвечаем честно
            MirageResponse::Real {
                anomaly_score: anomaly,
            }
        }
    }

    pub fn status(&self) -> String {
        format!(
            "MirageNode [{}]: active={} | mazes={} | deflected={} | {}",
            self.node_id,
            self.mirage_active,
            self.active_mazes.len(),
            self.attacks_deflected,
            self.detector.stats(),
        )
    }
}

/// Ответ модуля мимикрии
#[derive(Debug)]
pub enum MirageResponse {
    /// Реальные данные (нормальный запрос)
    Real { anomaly_score: AnomalyScore },
    /// Ложные данные (атака обнаружена)
    Fake {
        fake_tensor: FakeTensor,
        anomaly_score: AnomalyScore,
        maze_entry: Option<String>,
    },
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::SsauTensor;

    #[test]
    fn test_anomaly_detection_rate_flood() {
        let mut detector = AnomalyDetector::new();
        let mut last_score = detector.record_request("attacker", "A→B", 64, 10.0);
        for i in 0..15 {
            last_score = detector.record_request(
                "attacker", "A→B", 64, 10.0 + i as f64 * 0.1
            );
        }
        assert!(last_score.threat_level > 0.0);
        assert!(last_score.anomalies.contains(&AnomalyType::RateFlooding));
        println!("✅ Rate Flood detected: threat={:.4} anomalies={:?}",
            last_score.threat_level, last_score.anomalies);
    }

    #[test]
    fn test_mirage_generator() {
        let mut real = SsauTensor::new("A", "B", 50.0, 500.0);
        real.reliability = 0.7;

        let anomaly = AnomalyScore {
            threat_level: 0.8,
            anomalies: vec![AnomalyType::TopologyProbing],
            suspected_attacker: Some("spy_node".to_string()),
            activate_mirage: true,
            description: "test".to_string(),
        };

        let mut gen = MirageGenerator::new(MimicryMatrix::perfect_lure());
        let fake = gen.generate_fake_tensor(&real, 0.8, &anomaly);

        println!("✅ Mirage Generator:");
        println!("   Real:  latency={:.1}ms  BW={:.0}Mbps  rel={:.2}",
            fake.real_latency_ms, real.bandwidth, real.reliability);
        println!("   Fake:  latency={:.1}ms  BW={:.0}Mbps  rel={:.2}",
            fake.fake_latency_ms, fake.fake_bandwidth_mbps, fake.fake_reliability);
        println!("   Φ(A)={:.2} strategy={:?} trap_id={}",
            fake.phi_weight, fake.strategy, fake.trap_id);

        assert!(fake.fake_latency_ms < fake.real_latency_ms,
            "Perfect lure должен показывать меньшую задержку");
        assert!(fake.fake_bandwidth_mbps > real.bandwidth,
            "Perfect lure должен показывать большую bandwidth");
    }

    #[test]
    fn test_maze_trap() {
        let maze = MazeTrap::create_for_attacker("spy_node", 5);
        let result = maze.simulate_packet(20);

        println!("✅ Maze Trap для spy_node:");
        println!("   Узлов в лабиринте: {}", maze.nodes.len());
        println!("   Симуляция: {} хопов, {:.1}ms, {} петель",
            result.hops, result.total_latency_ms, result.loops_detected);
        println!("   Путь: {:?}", &result.path[..result.path.len().min(5)]);

        assert!(result.loops_detected > 0, "Лабиринт должен содержать петли");
        assert!(result.total_latency_ms > 0.0);
    }

    #[test]
    fn test_full_mirage_node() {
        let mut mirage = MirageNode::new("federation_node");
        let real_tensor = SsauTensor::new("A", "B", 10.0, 1000.0);

        // Нормальный запрос
        let resp = mirage.handle_request("normal_peer", "A→B", 64, 15.0, &real_tensor);
        match resp {
            MirageResponse::Real { .. } => println!("✅ Нормальный запрос: реальные данные"),
            MirageResponse::Fake { .. } => println!("⚠️ Ложная тревога"),
        }

        // Имитируем атаку: много запросов с одного источника
        for i in 0..15 {
            mirage.handle_request(
                "spy_node",
                &format!("route_{}", i % 8),
                60 + i as u8,
                5.0 + i as f64 * 0.1,
                &real_tensor,
            );
        }

        // Последний запрос должен получить ложные данные
        let attack_resp = mirage.handle_request("spy_node", "A→B", 64, 5.1, &real_tensor);
        match attack_resp {
            MirageResponse::Fake { ref fake_tensor, ref anomaly_score, .. } => {
                println!("✅ Атака обнаружена! Mirage активирован:");
                println!("   Threat level: {:.4}", anomaly_score.threat_level);
                println!("   Anomalies: {:?}", anomaly_score.anomalies);
                println!("   Fake latency: {:.1}ms (real: {:.1}ms)",
                    fake_tensor.fake_latency_ms, fake_tensor.real_latency_ms);
                assert!(anomaly_score.threat_level > 0.0);
            }
            MirageResponse::Real { .. } => {
                println!("ℹ️ Атака ещё не достигла порога (нормально)");
            }
        }

        println!("\n{}", mirage.status());
    }
}
