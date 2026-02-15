// =============================================================================
// FEDERATION CORE — dag.rs
// PHASE 2 / WEEK 5 — «DAG Consensus (Light version)»
// =============================================================================
//
// Реализует:
//   1. DagNode        — вершина графа (одна запись маршрута)
//   2. DagEdge        — связь между вершинами (подтверждение)
//   3. FederationDag  — граф с методами добавления и верификации
//   4. PoaReward      — Proof-of-Awareness: награды за честные тензоры
//   5. DagExplorer    — статистика графа
//
// Важно (MVP):
//   - In-memory DAG
//   - Pruning безопасный: не удаляем вершины, на которые кто-то ссылается как parent
//   - TrustRegistry реально обновляется через TriangleCheckResult
// =============================================================================

use crate::tensor::{triangle_check, SsauTensor, TrustRegistry, TriangleCheckResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

/// Базовая награда за честный маршрут (в "credits" Федерации)
pub const BASE_POA_REWARD: f64 = 1.0;

/// Бонус за высокое качество канала (reliability > 0.95)
pub const QUALITY_BONUS: f64 = 0.5;

/// Штраф за нечестные данные
pub const DISHONESTY_PENALTY: f64 = 2.0;

/// Максимальное число вершин DAG для лёгкой версии (in-memory)
/// (название оставлено ради совместимости)
pub const MAX_DAG_DEPTH: usize = 1000;

// -----------------------------------------------------------------------------
// DagNode — вершина графа
// -----------------------------------------------------------------------------

/// Одна запись в DAG — факт маршрутизации пакета через узел
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagNode {
    /// Уникальный ID вершины (hash от содержимого)
    pub id: String,
    /// Узел который создал эту запись
    pub reporter_id: String,
    /// Маршрут который был использован
    pub route_path: Vec<String>,
    /// Снимок SSAU тензоров на момент маршрутизации
    pub ssau_snapshot: Vec<SsauSnapshot>,
    /// Суммарная задержка маршрута (мс)
    pub total_latency_ms: f64,
    /// Время создания (Unix ms)
    pub timestamp: i64,
    /// ID родительских вершин (предыдущие записи)
    pub parents: Vec<String>,
    /// Глубина в графе
    pub depth: usize,
    /// Статус верификации
    pub verified: bool,
    /// Начисленная награда PoA
    pub poa_reward: f64,
    /// Оценка честности (0.0 = ложь, 1.0 = правда)
    pub honesty_score: f64,
}

/// Снимок одного тензора для записи в DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsauSnapshot {
    pub from_node: String,
    pub to_node: String,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    pub reliability: f64,
}

impl From<&SsauTensor> for SsauSnapshot {
    fn from(t: &SsauTensor) -> Self {
        SsauSnapshot {
            from_node: t.from_node.clone(),
            to_node: t.to_node.clone(),
            latency_ms: t.latency.mean,
            bandwidth_mbps: t.bandwidth,
            reliability: t.reliability,
        }
    }
}

impl DagNode {
    /// Создать новую вершину DAG
    pub fn new(
        reporter_id: &str,
        route_path: Vec<String>,
        tensors: &[&SsauTensor],
        parents: Vec<String>,
        depth: usize,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let total_latency: f64 = tensors.iter().map(|t| t.latency.mean).sum();
        let snapshots: Vec<SsauSnapshot> = tensors.iter().map(|t| SsauSnapshot::from(*t)).collect();

        // MVP hash (FNV-1a 64): reporter + timestamp + path + latency
        // В будущем заменить на SHA-256 от сериализованного header.
        let id = format!("{:x}", {
            let mut h: u64 = 0xcbf29ce484222325;
            let input = format!(
                "{}|{}|{}|{:.4}",
                reporter_id,
                now,
                route_path.join(","),
                total_latency
            );
            for b in input.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        });

        DagNode {
            id,
            reporter_id: reporter_id.to_string(),
            route_path,
            ssau_snapshot: snapshots,
            total_latency_ms: total_latency,
            timestamp: now,
            parents,
            depth,
            verified: false,
            poa_reward: 0.0,
            honesty_score: 1.0,
        }
    }
}

// -----------------------------------------------------------------------------
// DagEdge — связь между вершинами
// -----------------------------------------------------------------------------

/// Направленное ребро в DAG (подтверждение от одной вершины к другой)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagEdge {
    pub from_id: String,
    pub to_id: String,
    /// Узел который подтвердил связь
    pub confirmed_by: String,
    pub timestamp: i64,
}

// -----------------------------------------------------------------------------
// PoA — Proof-of-Awareness
// -----------------------------------------------------------------------------

/// Результат расчёта награды PoA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoaResult {
    pub node_id: String,
    pub reward: f64,
    pub penalty: f64,
    pub net: f64,
    pub reason: String,
    pub honesty_score: f64,
    /// Если был triangle-check — сохраняем результат (чтобы обновить trust_registry снаружи)
    pub triangle: Option<TriangleCheckResult>,
}

/// Рассчитать награду Proof-of-Awareness для узла.
///
/// Логика:
///   - Базовая награда за участие
///   - Бонус за высокую надёжность
///   - Triangle Check (если есть свидетели) → штраф/снижение честности
///   - Множитель от текущего Trust Weight
pub fn calculate_poa_reward(
    node: &DagNode,
    trust_registry: &TrustRegistry,
    witness_latencies: Option<(f64, f64)>, // (L_AC, L_BC) от соседей-свидетелей
) -> PoaResult {
    let trust_weight = trust_registry.get_trust(&node.reporter_id);
    let mut reward = BASE_POA_REWARD * trust_weight;
    let mut penalty = 0.0;
    let mut honesty_score = 1.0;
    let mut reason = String::new();
    let mut triangle_result: Option<TriangleCheckResult> = None;

    // Бонус за качество канала
    let avg_reliability: f64 = if node.ssau_snapshot.is_empty() {
        0.0
    } else {
        node.ssau_snapshot.iter().map(|s| s.reliability).sum::<f64>()
            / node.ssau_snapshot.len() as f64
    };

    if avg_reliability > 0.95 {
        reward += QUALITY_BONUS;
        reason += &format!(
            "Quality bonus +{:.2} (reliability={:.3}). ",
            QUALITY_BONUS, avg_reliability
        );
    }

    // Triangle Check если есть свидетели
    if let Some((l_ac, l_bc)) = witness_latencies {
        if let Some(snapshot) = node.ssau_snapshot.first() {
            let check = triangle_check(
                snapshot.latency_ms, // L_AB
                l_ac,                // L_AC
                l_bc,                // L_BC
                trust_weight,
            );

            if !check.passed {
                penalty = DISHONESTY_PENALTY * check.deviation_score;
                honesty_score = (1.0 - check.deviation_score).clamp(0.0, 1.0);
                reason += &format!(
                    "⛔ Triangle Check FAILED! Penalty -{:.2}. Deviation={:.1}%. ",
                    penalty,
                    check.deviation_score * 100.0
                );
            } else {
                reason += "✅ Triangle Check passed. ";
                honesty_score = 1.0;
            }

            triangle_result = Some(check);
        }
    }

    // Бонус за скорость (суммарная задержка < 20ms)
    if node.total_latency_ms < 20.0 {
        let speed_bonus = 0.3 * (1.0 - node.total_latency_ms / 20.0);
        reward += speed_bonus;
        reason += &format!(
            "Speed bonus +{:.2} (latency={:.1}ms). ",
            speed_bonus, node.total_latency_ms
        );
    }

    let net = (reward - penalty).max(0.0);
    reason += &format!("Net: {:.4} credits.", net);

    PoaResult {
        node_id: node.reporter_id.clone(),
        reward,
        penalty,
        net,
        reason,
        honesty_score,
        triangle: triangle_result,
    }
}

// -----------------------------------------------------------------------------
// FederationDag — главная структура графа
// -----------------------------------------------------------------------------

/// Асинхронный DAG Федерации.
/// В Phase 2 (light version) — in-memory.
/// В Phase 3 — персистентное хранилище.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FederationDag {
    /// Все вершины: id → DagNode
    pub nodes: HashMap<String, DagNode>,
    /// Все рёбра
    pub edges: Vec<DagEdge>,
    /// Баланс credits: node_id → накопленные credits
    pub balances: HashMap<String, f64>,
    /// Tips — вершины без исходящих рёбер (кончики DAG)
    pub tips: Vec<String>,
    /// Счётчик операций
    pub total_operations: u64,
}

impl FederationDag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавить новую запись маршрута в DAG.
    ///
    /// Алгоритм:
    ///   1. Выбираем до 2 tips как родителей (подтверждаем их)
    ///   2. Создаём новую вершину
    ///   3. Рассчитываем PoA награду
    ///   4. Обновляем trust_registry (если был triangle-check)
    ///   5. Обновляем tips
    ///   6. Начисляем credits
    pub fn append_route(
        &mut self,
        reporter_id: &str,
        route_path: Vec<String>,
        tensors: &[&SsauTensor],
        trust_registry: &mut TrustRegistry,
        witness_latencies: Option<(f64, f64)>,
    ) -> (DagNode, PoaResult) {
        // Выбираем родителей из текущих tips
        let parents: Vec<String> = self.tips.iter().take(2).cloned().collect();
        let depth = if parents.is_empty() {
            0
        } else {
            parents
                .iter()
                .filter_map(|p| self.nodes.get(p))
                .map(|n| n.depth)
                .max()
                .unwrap_or(0)
                + 1
        };

        // Создаём вершину
        let mut node = DagNode::new(reporter_id, route_path, tensors, parents.clone(), depth);

        // Рассчитываем PoA
        let poa = calculate_poa_reward(&node, trust_registry, witness_latencies);

        // Применяем trust update (важно!)
        if let Some(ref tri) = poa.triangle {
            trust_registry.record_check(reporter_id, tri);
        }

        // Проставляем поля вершины
        node.poa_reward = poa.net;
        node.honesty_score = poa.honesty_score;
        node.verified = poa.honesty_score > 0.5;

        // Добавляем рёбра от родителей к новой вершине
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        for parent_id in &parents {
            self.edges.push(DagEdge {
                from_id: parent_id.clone(),
                to_id: node.id.clone(),
                confirmed_by: reporter_id.to_string(),
                timestamp: now,
            });
            // Убираем подтверждённый tip
            self.tips.retain(|t| t != parent_id);
        }

        // Начисляем credits
        *self.balances.entry(reporter_id.to_string()).or_insert(0.0) += poa.net;

        // Добавляем новую вершину как tip
        let node_id = node.id.clone();
        self.nodes.insert(node_id.clone(), node.clone());
        self.tips.push(node_id);
        self.total_operations += 1;

        // Ограничиваем размер DAG (safe prune)
        if self.nodes.len() > MAX_DAG_DEPTH {
            self.prune_old_nodes();
        }

        (node, poa)
    }

    /// Получить историю маршрутов конкретного узла
    pub fn get_node_history(&self, node_id: &str) -> Vec<&DagNode> {
        let mut history: Vec<&DagNode> = self
            .nodes
            .values()
            .filter(|n| n.reporter_id == node_id)
            .collect();
        history.sort_by_key(|n| n.timestamp);
        history
    }

    /// Верифицировать цепочку от вершины до genesis.
    /// Проверяем, что:
    ///   - нет циклов
    ///   - каждая посещённая вершина verified
    ///   - все parent-ссылки существуют и валидны
    pub fn verify_chain(&self, start_id: &str) -> bool {
        let mut stack = vec![start_id.to_string()];
        let mut visited = HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                return false; // цикл
            }
            visited.insert(current_id.clone());

            let node = match self.nodes.get(&current_id) {
                Some(n) => n,
                None => return false,
            };

            if !node.verified {
                return false;
            }

            // genesis — ок
            if node.parents.is_empty() {
                continue;
            }

            for p in &node.parents {
                stack.push(p.clone());
            }
        }

        true
    }

    /// Safe prune: удаляем только те вершины, которые:
    ///   - НЕ tips
    ///   - НЕ используются как parent в других вершинах
    /// Сохраняем “скелет” DAG, чтобы verify_chain не ломался.
    fn prune_old_nodes(&mut self) {
        if self.nodes.len() <= MAX_DAG_DEPTH / 2 {
            return;
        }

        // Собираем множество всех parent-ссылок
        let mut referenced: HashSet<String> = HashSet::new();
        for n in self.nodes.values() {
            for p in &n.parents {
                referenced.insert(p.clone());
            }
        }

        // Сортируем кандидатов по depth (старые сначала)
        let mut nodes_by_depth: Vec<(String, usize)> = self
            .nodes
            .iter()
            .map(|(id, n)| (id.clone(), n.depth))
            .collect();
        nodes_by_depth.sort_by_key(|(_, d)| *d);

        // Удаляем понемногу, пока не ужмём размер
        let target = MAX_DAG_DEPTH / 2;
        for (id, _) in nodes_by_depth {
            if self.nodes.len() <= target {
                break;
            }
            if self.tips.contains(&id) {
                continue;
            }
            if referenced.contains(&id) {
                continue; // нельзя — кто-то ссылается как parent
            }
            self.nodes.remove(&id);
        }
    }

    /// Статистика DAG
    pub fn stats(&self) -> DagStats {
        let total_rewards: f64 = self.balances.values().sum();
        let verified_count = self.nodes.values().filter(|n| n.verified).count();
        let avg_honesty = if self.nodes.is_empty() {
            1.0
        } else {
            self.nodes.values().map(|n| n.honesty_score).sum::<f64>() / self.nodes.len() as f64
        };

        DagStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            tips_count: self.tips.len(),
            total_operations: self.total_operations,
            total_rewards_issued: total_rewards,
            verified_nodes: verified_count,
            avg_honesty_score: avg_honesty,
            richest_node: self
                .balances
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(id, bal)| format!("{}: {:.4} credits", id, bal))
                .unwrap_or_else(|| "нет данных".to_string()),
        }
    }
}

/// Статистика DAG для отображения
#[derive(Debug, Serialize, Deserialize)]
pub struct DagStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub tips_count: usize,
    pub total_operations: u64,
    pub total_rewards_issued: f64,
    pub verified_nodes: usize,
    pub avg_honesty_score: f64,
    pub richest_node: String,
}

impl std::fmt::Display for DagStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "╔══════════════════════════════════════════════════╗\n\
             ║  DAG LEDGER STATUS                               ║\n\
             ╠══════════════════════════════════════════════════╣\n\
             ║  Вершин:      {:>6}  Рёбер:    {:>6}          ║\n\
             ║  Tips:        {:>6}  Операций: {:>6}          ║\n\
             ║  Верифицировано: {:>6}/{:<6}                   ║\n\
             ║  Avg честность:  {:.4}                         ║\n\
             ║  Всего наград:   {:.4} credits                 ║\n\
             ║  Богатейший: {}  ║\n\
             ╚══════════════════════════════════════════════════╝",
            self.total_nodes,
            self.total_edges,
            self.tips_count,
            self.total_operations,
            self.verified_nodes,
            self.total_nodes,
            self.avg_honesty_score,
            self.total_rewards_issued,
            self.richest_node,
        )
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::SsauTensor;

    fn make_tensor(from: &str, to: &str, latency: f64, reliability: f64) -> SsauTensor {
        let mut t = SsauTensor::new(from, to, latency, 1000.0);
        t.reliability = reliability;
        t
    }

    #[test]
    fn test_dag_append_and_grow() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();

        let t1 = make_tensor("A", "B", 10.0, 0.99);
        let t2 = make_tensor("B", "C", 15.0, 0.97);

        // Genesis запись
        let (node1, poa1) = dag.append_route(
            "node_ALPHA",
            vec!["A".into(), "B".into(), "C".into()],
            &[&t1, &t2],
            &mut trust,
            None,
        );
        println!("✅ Genesis: id={} depth={} reward={:.4}", &node1.id[..8], node1.depth, poa1.net);
        println!("   Причина: {}", poa1.reason);

        // Вторая запись (со свидетелями)
        let t3 = make_tensor("A", "D", 5.0, 0.99);
        let (node2, poa2) = dag.append_route(
            "node_BETA",
            vec!["A".into(), "D".into()],
            &[&t3],
            &mut trust,
            Some((3.0, 4.0)), // свидетели
        );
        println!("✅ Node 2:  id={} depth={} reward={:.4}", &node2.id[..8], node2.depth, poa2.net);
        println!("   Причина: {}", poa2.reason);

        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.tips.len(), 1);

        let stats = dag.stats();
        println!("\n{}", stats);
    }

    #[test]
    fn test_poa_dishonest_node_updates_trust() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();

        // Лживый узел: заявляет 1ms, свидетели видят 20+25ms
        let t_lie = make_tensor("X", "Y", 1.0, 0.99);
        let (_node, poa) = dag.append_route(
            "lying_node",
            vec!["X".into(), "Y".into()],
            &[&t_lie],
            &mut trust,
            Some((20.0, 25.0)),
        );

        assert!(poa.penalty > 0.0, "Должен быть штраф за ложь");

        let w = trust.get_trust("lying_node");
        assert!(w < 1.0, "Trust должен уменьшиться после провала triangle-check");

        println!("✅ PoA лжеца: net={:.4} trust={:.4}", poa.net, w);
        println!("   {}", poa.reason);
    }

    #[test]
    fn test_dag_chain_verification_all_parents() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();
        let t = make_tensor("A", "B", 10.0, 0.99);

        let (n1, _) = dag.append_route("node_A", vec!["A".into(), "B".into()], &[&t], &mut trust, None);
        let (n2, _) = dag.append_route("node_A", vec!["A".into(), "B".into()], &[&t], &mut trust, None);

        assert!(dag.verify_chain(&n2.id));
        assert!(n2.depth > n1.depth);
    }
}
