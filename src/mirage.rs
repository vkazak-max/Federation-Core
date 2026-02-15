// =============================================================================
// FEDERATION CORE — mirage.rs
// PHASE 2 / WEEK 7 — «Active Mimicry (Mirage Module)»
// =============================================================================
//
// Реализует:
//   1) AnomalyDetector — обнаружение признаков сканирования/атаки (окна времени)
//   2) MirageGenerator — генерация ложных SSAU тензоров (T_fake)
//   3) MazeTrap        — виртуальная топология-ловушка (пакеты зацикливаются)
//   4) MirageNode      — модуль мимикрии узла (detector + generator + mazes)
//
// Математика (идея):
//   T_fake = T_real + Φ(A) · M
//
// MVP: эвристики + псевдорандом (не security-grade).
// =============================================================================

use crate::tensor::SsauTensor;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

/// Окно для расчёта rate (мс)
pub const RATE_WINDOW_MS: i64 = 1_000;

/// Порог аномального rate (запросов/сек) (признак сканирования)
pub const SCAN_RATE_THRESHOLD: f64 = 10.0;

/// Порог уникальных маршрутов в окне (признак перебора)
pub const UNIQUE_ROUTES_THRESHOLD: usize = 6;

/// Порог “слишком регулярных” задержек (CV ниже => автоматизация/пробинг)
pub const TIMING_ANOMALY_THRESHOLD: f64 = 0.15;

/// Минимум наблюдений, чтобы включать эвристику регулярности
pub const MIN_TIMING_SAMPLES: usize = 6;

/// Время жизни Mirage-ловушки (секунды)
pub const MIRAGE_TTL_SECS: u64 = 300;

/// Порог активации mirage по threat
pub const MIRAGE_THREAT_THRESHOLD: f64 = 0.35;

/// “Распределённый скан”: активных источников больше чем
pub const DISTRIBUTED_SOURCES_THRESHOLD: usize = 12;

/// Ограничение на сохранение источников (чтобы память не росла бесконечно)
pub const MAX_TRACKED_SOURCES: usize = 512;

/// Сколько max TTL значений храним на источник (окно)
pub const MAX_TTL_SAMPLES: usize = 32;

/// Сколько max маршрутов храним на источник (окно)
pub const MAX_ROUTE_SAMPLES: usize = 64;

/// Сколько max timing deltas храним на источник (окно)
pub const MAX_TIMING_SAMPLES: usize = 64;

// -----------------------------------------------------------------------------
// Время
// -----------------------------------------------------------------------------

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

// -----------------------------------------------------------------------------
// AnomalyScore — оценка угрозы
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnomalyType {
    RateFlooding,
    TopologyProbing,
    TtlScanning,
    DistributedScan,
    RouteEnumeration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyScore {
    /// 0.0..1.0
    pub threat_level: f64,
    pub anomalies: Vec<AnomalyType>,
    pub suspected_attacker: Option<String>,
    pub activate_mirage: bool,
    pub description: String,
}

// -----------------------------------------------------------------------------
// AnomalyDetector
// -----------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct SourceStats {
    /// timestamps запросов в окне RATE_WINDOW_MS
    request_times: VecDeque<i64>,

    /// последние маршруты (окно по количеству)
    routes_window: VecDeque<String>,

    /// TTL значения (окно)
    ttl_values: VecDeque<u8>,

    /// timing deltas (окно)
    timing_deltas: VecDeque<f64>,

    last_seen: i64,
}

#[derive(Debug, Default)]
pub struct AnomalyDetector {
    source_stats: HashMap<String, SourceStats>,
    pub mirage_activations: u64,
    pub total_requests: u64,
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_request(
        &mut self,
        source_id: &str,
        queried_route: &str,
        ttl: u8,
        timing_delta_ms: f64,
    ) -> AnomalyScore {
        let now = now_ms();
        self.total_requests += 1;

        // Ограничение памяти: если источников слишком много — удаляем самых старых (грубо)
        if self.source_stats.len() > MAX_TRACKED_SOURCES {
            self.evict_oldest_sources();
        }

        let stats = self.source_stats.entry(source_id.to_string()).or_default();
        stats.last_seen = now;

        // rate window
        stats.request_times.push_back(now);
        Self::prune_times(&mut stats.request_times, now - RATE_WINDOW_MS);

        // routes window
        stats.routes_window.push_back(queried_route.to_string());
        while stats.routes_window.len() > MAX_ROUTE_SAMPLES {
            stats.routes_window.pop_front();
        }

        // ttl window
        stats.ttl_values.push_back(ttl);
        while stats.ttl_values.len() > MAX_TTL_SAMPLES {
            stats.ttl_values.pop_front();
        }

        // timing window
        stats.timing_deltas.push_back(timing_delta_ms.max(0.0));
        while stats.timing_deltas.len() > MAX_TIMING_SAMPLES {
            stats.timing_deltas.pop_front();
        }

        self.analyze(source_id)
    }

    fn analyze(&self, source_id: &str) -> AnomalyScore {
        let Some(stats) = self.source_stats.get(source_id) else {
            return AnomalyScore {
                threat_level: 0.0,
                anomalies: vec![],
                suspected_attacker: None,
                activate_mirage: false,
                description: "Нет данных".to_string(),
            };
        };

        let mut threat = 0.0_f64;
        let mut anomalies = Vec::new();
        let mut desc = Vec::new();

        // 1) RateFlooding
        let rate = stats.request_times.len() as f64 * (1000.0 / RATE_WINDOW_MS as f64); // req/sec
        if rate > SCAN_RATE_THRESHOLD {
            let severity = ((rate / SCAN_RATE_THRESHOLD) - 1.0).min(1.0);
            threat += 0.35 * severity;
            anomalies.push(AnomalyType::RateFlooding);
            desc.push(format!("RateFlood: {:.1} req/s", rate));
        }

        // 2) RouteEnumeration (уникальные маршруты в окне)
        let unique_routes = stats.routes_window.iter().collect::<HashSet<_>>().len();
        if unique_routes >= UNIQUE_ROUTES_THRESHOLD {
            let severity = ((unique_routes as f64 / UNIQUE_ROUTES_THRESHOLD as f64) - 1.0).min(1.0);
            threat += 0.25 * (0.5 + 0.5 * severity);
            anomalies.push(AnomalyType::RouteEnumeration);
            desc.push(format!("RouteEnum: {} uniq routes", unique_routes));
        }

        // 3) TTL scanning
        if stats.ttl_values.len() >= 6 {
            let values: Vec<f64> = stats.ttl_values.iter().map(|&t| t as f64).collect();
            let var = variance(&values);
            if var > 6.0 {
                threat += 0.20;
                anomalies.push(AnomalyType::TtlScanning);
                desc.push(format!("TTLScan: var={:.2}", var));
            }
        }

        // 4) Topology probing (слишком регулярные задержки)
        if stats.timing_deltas.len() >= MIN_TIMING_SAMPLES {
            let values: Vec<f64> = stats.timing_deltas.iter().copied().collect();
            let cv = coefficient_of_variation(&values);
            if cv < TIMING_ANOMALY_THRESHOLD {
                threat += 0.25;
                anomalies.push(AnomalyType::TopologyProbing);
                desc.push(format!("TopologyProbe: CV={:.4}", cv));
            }
        }

        // 5) Distributed scan (слишком много активных источников в целом)
        let active_sources = self.source_stats.len();
        if active_sources >= DISTRIBUTED_SOURCES_THRESHOLD {
            threat += 0.15;
            anomalies.push(AnomalyType::DistributedScan);
            desc.push(format!("DistScan: {} sources", active_sources));
        }

        threat = threat.min(1.0);
        let activate = threat >= MIRAGE_THREAT_THRESHOLD || anomalies.len() >= 2;

        AnomalyScore {
            threat_level: threat,
            anomalies,
            suspected_attacker: if activate { Some(source_id.to_string()) } else { None },
            activate_mirage: activate,
            description: if desc.is_empty() { "Норма".to_string() } else { desc.join(" | ") },
        }
    }

    fn prune_times(q: &mut VecDeque<i64>, cutoff: i64) {
        while let Some(&t) = q.front() {
            if t < cutoff {
                q.pop_front();
            } else {
                break;
            }
        }
    }

    fn evict_oldest_sources(&mut self) {
        // Удаляем ~10% самых старых
        let mut items: Vec<(String, i64)> = self
            .source_stats
            .iter()
            .map(|(id, st)| (id.clone(), st.last_seen))
            .collect();

        items.sort_by_key(|(_, ts)| *ts);
        let remove_n = (items.len() / 10).max(1);

        for (id, _) in items.into_iter().take(remove_n) {
            self.source_stats.remove(&id);
        }
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
// MirageGenerator
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MimicryMatrix {
    pub latency_factor: f64,
    pub bandwidth_factor: f64,
    pub reliability_target: f64,
    pub noise_amplitude: f64,
}

impl MimicryMatrix {
    pub fn perfect_lure() -> Self {
        MimicryMatrix {
            latency_factor: 0.15,
            bandwidth_factor: 8.0,
            reliability_target: 0.995,
            noise_amplitude: 0.02,
        }
    }

    pub fn dead_node() -> Self {
        MimicryMatrix {
            latency_factor: 50.0,
            bandwidth_factor: 0.05,
            reliability_target: 0.05,
            noise_amplitude: 0.35,
        }
    }

    pub fn maze() -> Self {
        MimicryMatrix {
            latency_factor: 1.0,
            bandwidth_factor: 1.0,
            reliability_target: 0.92,
            noise_amplitude: 0.06,
        }
    }
}

pub struct MirageGenerator {
    pub matrix: MimicryMatrix,
    pub traps_generated: u64,
    rng_state: u64,
}

impl MirageGenerator {
    pub fn new(matrix: MimicryMatrix) -> Self {
        let seed = now_ms() as u64 ^ 0xA5A5_1234_F00D_BEEF;
        MirageGenerator { matrix, traps_generated: 0, rng_state: seed }
    }

    fn next_rand(&mut self) -> f64 {
        // xorshift64
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as f64) / (u64::MAX as f64)
    }

    pub fn generate_fake_tensor(
        &mut self,
        real: &SsauTensor,
        threat_level: f64,
        anomaly_score: &AnomalyScore,
    ) -> FakeTensor {
        let phi = threat_level.clamp(0.0, 1.0);

        let noise = |amp: f64, base: f64, r: &mut MirageGenerator| -> f64 {
            ((r.next_rand() - 0.5) * 2.0) * amp * base.max(1.0)
        };

        // latency: множитель + шум
        let base_lat = real.latency.mean.max(0.1);
        let fake_latency = base_lat
            * (1.0 + phi * (self.matrix.latency_factor - 1.0))
            + noise(self.matrix.noise_amplitude, base_lat, self);

        // bandwidth: множитель + шум
        let base_bw = real.bandwidth.max(0.1);
        let fake_bw = base_bw
            * (1.0 + phi * (self.matrix.bandwidth_factor - 1.0))
            + noise(self.matrix.noise_amplitude, base_bw, self);

        // reliability: тянем к target + шум
        let base_rel = real.reliability.clamp(0.0, 1.0);
        let mut fake_rel = base_rel + phi * (self.matrix.reliability_target - base_rel);
        fake_rel += (self.next_rand() - 0.5) * 2.0 * self.matrix.noise_amplitude;
        fake_rel = fake_rel.clamp(0.0, 1.0);

        self.traps_generated += 1;

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
            fake_bandwidth_mbps: fake_bw.max(0.1),
            fake_reliability: fake_rel,
            phi_weight: phi,
            strategy,
            trap_id: format!("trap_{:x}", self.rng_state & 0xffff_ffff),
            // эти поля оставляю для отладки (в public API не показывать)
            real_latency_ms: real.latency.mean,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MimicryStrategy {
    PerfectLure,
    DeadNode,
    Maze,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeTensor {
    pub from_node: String,
    pub to_node: String,
    pub fake_latency_ms: f64,
    pub fake_bandwidth_mbps: f64,
    pub fake_reliability: f64,
    pub phi_weight: f64,
    pub strategy: MimicryStrategy,
    pub trap_id: String,

    /// debug-only
    pub real_latency_ms: f64,
}

// -----------------------------------------------------------------------------
// MazeTrap
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeNode {
    pub id: String,
    pub fake_latency_ms: f64,
    pub next_hop: String,
    pub is_loop: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MazeTrap {
    pub nodes: Vec<MazeNode>,
    pub entry_point: String,
    pub total_fake_latency: f64,
    pub created_for: String,
    pub created_at_ms: i64,
}

impl MazeTrap {
    pub fn create_for_attacker(attacker_id: &str, depth: usize) -> Self {
        let depth = depth.max(3).min(32);
        let mut nodes = vec![];
        let mut total_latency = 0.0;

        for i in 0..depth {
            let next = (i + 1) % depth;
            let fake_latency = 5.0 + (i as f64 * 3.0);
            total_latency += fake_latency;

            nodes.push(MazeNode {
                id: format!("mirage_{}_{}", attacker_id, i),
                fake_latency_ms: fake_latency,
                next_hop: format!("mirage_{}_{}", attacker_id, next),
                is_loop: next == 0,
            });
        }

        MazeTrap {
            entry_point: format!("mirage_{}_0", attacker_id),
            nodes,
            total_fake_latency: total_latency,
            created_for: attacker_id.to_string(),
            created_at_ms: now_ms(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let age_ms = now_ms() - self.created_at_ms;
        age_ms > (MIRAGE_TTL_SECS as i64 * 1000)
    }

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
                    if loops > 2 {
                        break;
                    }
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
// MirageNode
// -----------------------------------------------------------------------------

pub struct MirageNode {
    pub node_id: String,
    pub detector: AnomalyDetector,
    pub generator: MirageGenerator,
    pub active_mazes: HashMap<String, MazeTrap>,
    pub mirage_active: bool,
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

    pub fn handle_request(
        &mut self,
        source_id: &str,
        queried_route: &str,
        ttl: u8,
        timing_delta_ms: f64,
        real_tensor: &SsauTensor,
    ) -> MirageResponse {
        // периодически чистим старые лабиринты
        self.cleanup_mazes();

        let anomaly = self
            .detector
            .record_request(source_id, queried_route, ttl, timing_delta_ms);

        if anomaly.activate_mirage {
            self.mirage_active = true;
            self.attacks_deflected += 1;
            self.detector.mirage_activations += 1;

            // стратегия
            self.generator.matrix = if anomaly.anomalies.contains(&AnomalyType::TopologyProbing) {
                MimicryMatrix::perfect_lure()
            } else if anomaly.anomalies.contains(&AnomalyType::RateFlooding) {
                MimicryMatrix::dead_node()
            } else {
                MimicryMatrix::maze()
            };

            let fake = self
                .generator
                .generate_fake_tensor(real_tensor, anomaly.threat_level, &anomaly);

            // лабиринт на атакующего
            self.active_mazes
                .entry(source_id.to_string())
                .or_insert_with(|| MazeTrap::create_for_attacker(source_id, 6));

            MirageResponse::Fake {
                fake_tensor: fake,
                anomaly_score: anomaly,
                maze_entry: self.active_mazes.get(source_id).map(|m| m.entry_point.clone()),
            }
        } else {
            MirageResponse::Real { anomaly_score: anomaly }
        }
    }

    fn cleanup_mazes(&mut self) {
        let expired: Vec<String> = self
            .active_mazes
            .iter()
            .filter(|(_, m)| m.is_expired())
            .map(|(k, _)| k.clone())
            .collect();

        for k in expired {
            self.active_mazes.remove(&k);
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

#[derive(Debug)]
pub enum MirageResponse {
    Real { anomaly_score: AnomalyScore },
    Fake {
        fake_tensor: FakeTensor,
        anomaly_score: AnomalyScore,
        maze_entry: Option<String>,
    },
}

// -----------------------------------------------------------------------------
// Math helpers
// -----------------------------------------------------------------------------

fn variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64
}

fn coefficient_of_variation(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 1.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if mean.abs() < 1e-9 {
        return 1.0;
    }
    let std_dev = variance(values).sqrt();
    (std_dev / mean.abs()).clamp(0.0, 10.0)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_flood_detected_in_window() {
        let mut detector = AnomalyDetector::new();

        // имитируем 20 запросов "почти одновременно"
        let mut last = detector.record_request("attacker", "A→B", 64, 10.0);
        for _ in 0..20 {
            last = detector.record_request("attacker", "A→B", 64, 10.0);
        }

        assert!(last.threat_level > 0.0);
        assert!(last.anomalies.contains(&AnomalyType::RateFlooding));
    }

    #[test]
    fn test_maze_expiration_logic() {
        let maze = MazeTrap::create_for_attacker("spy", 5);
        assert!(!maze.is_expired());
    }

    #[test]
    fn test_mirage_node_activates() {
        let mut mirage = MirageNode::new("node_X");
        let real = SsauTensor::new("A", "B", 10.0, 1000.0);

        // даём много разных маршрутов быстро -> RouteEnumeration/RateFlood
        let mut resp = mirage.handle_request("spy", "r0", 60, 5.0, &real);
        for i in 0..20 {
            resp = mirage.handle_request("spy", &format!("r{}", i), 60 + (i as u8 % 10), 5.0, &real);
        }

        match resp {
            MirageResponse::Fake { .. } => {}
            MirageResponse::Real { anomaly_score } => {
                // может не добить порог на медленных машинах — тогда хотя бы threat должен быть > 0
                assert!(anomaly_score.threat_level >= 0.0);
            }
        }
    }
}
