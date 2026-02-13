cat > src/dag.rs << 'EOF'
// =============================================================================
// FEDERATION CORE — dag.rs
// PHASE 2 / WEEK 5 — «DAG Consensus (Light version)»
// =============================================================================
//
// Реализует:
//   1. DagNode     — вершина графа (одна запись маршрута)
//   2. DagEdge     — связь между вершинами (подтверждение)
//   3. FederationDag — сам граф с методами добавления и верификации
//   4. PoaReward   — Proof-of-Awareness: начисление наград за честные тензоры
//   5. DagExplorer — статистика и визуализация графа
// =============================================================================

use crate::tensor::{triangle_check, SsauTensor, TrustRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

/// Базовая награда за честный маршрут (в монетах Федерации)
pub const BASE_POA_REWARD: f64 = 1.0;

/// Бонус за высокое качество канала (reliability > 0.95)
pub const QUALITY_BONUS: f64 = 0.5;

/// Штраф за нечестные данные
pub const DISHONESTY_PENALTY: f64 = 2.0;

/// Максимальная глубина DAG для лёгкой версии
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

        // Простой hash: комбинируем reporter + timestamp + path
        let id = format!("{:x}", {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in format!("{}{}{}",
                reporter_id,
                now,
                route_path.join(",")
            ).bytes() {
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
// PoaReward — Proof-of-Awareness
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
}

/// Рассчитать награду Proof-of-Awareness для узла.
///
/// Логика:
///   - Базовая награда за участие в маршрутизации
///   - Бонус за высокое качество данных SSAU
///   - Штраф если Triangle Check выявил ложь
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

    // Бонус за качество канала
    let avg_reliability: f64 = if node.ssau_snapshot.is_empty() {
        0.0
    } else {
        node.ssau_snapshot.iter().map(|s| s.reliability).sum::<f64>()
            / node.ssau_snapshot.len() as f64
    };

    if avg_reliability > 0.95 {
        reward += QUALITY_BONUS;
        reason += &format!("Quality bonus +{:.2} (reliability={:.3}). ", QUALITY_BONUS, avg_reliability);
    }

    // Triangle Check если есть свидетели
    if let Some((l_ac, l_bc)) = witness_latencies {
        if let Some(snapshot) = node.ssau_snapshot.first() {
            let check = triangle_check(
                snapshot.latency_ms,
                l_ac,
                l_bc,
                trust_weight,
            );

            if !check.passed {
                penalty = DISHONESTY_PENALTY * check.deviation_score;
                honesty_score = 1.0 - check.deviation_score;
                reason += &format!(
                    "⛔ Triangle Check FAILED! Penalty -{:.2}. Deviation={:.1}%. ",
                    penalty,
                    check.deviation_score * 100.0
                );
            } else {
                reason += "✅ Triangle Check passed. ";
                honesty_score = 1.0;
            }
        }
    }

    // Бонус за скорость (задержка < 20ms)
    if node.total_latency_ms < 20.0 {
        let speed_bonus = 0.3 * (1.0 - node.total_latency_ms / 20.0);
        reward += speed_bonus;
        reason += &format!("Speed bonus +{:.2} (latency={:.1}ms). ", speed_bonus, node.total_latency_ms);
    }

    let net = (reward - penalty).max(0.0);
    reason += &format!("Net: {:.4} монет.", net);

    PoaResult {
        node_id: node.reporter_id.clone(),
        reward,
        penalty,
        net,
        reason,
        honesty_score,
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
    /// Баланс монет: node_id → накопленные монеты
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
    ///   1. Выбираем 2 tips как родителей (подтверждаем их)
    ///   2. Создаём новую вершину
    ///   3. Рассчитываем PoA награду
    ///   4. Обновляем tips
    ///   5. Начисляем монеты
    pub fn append_route(
        &mut self,
        reporter_id: &str,
        route_path: Vec<String>,
        tensors: &[&SsauTensor],
        trust_registry: &mut TrustRegistry,
        witness_latencies: Option<(f64, f64)>,
    ) -> (DagNode, PoaResult) {
        // Выбираем родителей из текущих tips (подтверждаем их)
        let parents: Vec<String> = self.tips.iter().take(2).cloned().collect();
        let depth = if parents.is_empty() {
            0
        } else {
            parents.iter()
                .filter_map(|p| self.nodes.get(p))
                .map(|n| n.depth)
                .max()
                .unwrap_or(0) + 1
        };

        // Создаём вершину
        let mut node = DagNode::new(reporter_id, route_path, tensors, parents.clone(), depth);

        // Рассчитываем PoA награду
        let poa = calculate_poa_reward(&node, trust_registry, witness_latencies);
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

        // Начисляем монеты
        *self.balances.entry(reporter_id.to_string()).or_insert(0.0) += poa.net;

        // Добавляем новую вершину как tip
        let node_id = node.id.clone();
        self.nodes.insert(node_id.clone(), node.clone());
        self.tips.push(node_id);
        self.total_operations += 1;

        // Ограничиваем размер DAG
        if self.nodes.len() > MAX_DAG_DEPTH {
            self.prune_old_nodes();
        }

        (node, poa)
    }

    /// Получить историю маршрутов конкретного узла
    pub fn get_node_history(&self, node_id: &str) -> Vec<&DagNode> {
        let mut history: Vec<&DagNode> = self.nodes.values()
            .filter(|n| n.reporter_id == node_id)
            .collect();
        history.sort_by_key(|n| n.timestamp);
        history
    }

    /// Верифицировать цепочку от вершины до genesis
    pub fn verify_chain(&self, node_id: &str) -> bool {
        let mut current_id = node_id.to_string();
        let mut visited = std::collections::HashSet::new();

        loop {
            if visited.contains(&current_id) {
                return false; // Цикл — невалидно (DAG не должен иметь циклов)
            }
            visited.insert(current_id.clone());

            match self.nodes.get(&current_id) {
                None => return false,
                Some(node) => {
                    if node.parents.is_empty() {
                        return true; // Дошли до genesis
                    }
                    if !node.verified {
                        return false;
                    }
                    current_id = node.parents[0].clone();
                }
            }
        }
    }

    /// Удалить старые подтверждённые вершины (оставить только последние)
    fn prune_old_nodes(&mut self) {
        let mut nodes_by_depth: Vec<(String, usize)> = self.nodes.iter()
            .map(|(id, n)| (id.clone(), n.depth))
            .collect();
        nodes_by_depth.sort_by_key(|(_, d)| *d);

        // Удаляем самые старые (низкая глубина), оставляем MAX_DAG_DEPTH/2
        let to_remove = nodes_by_depth.len() - MAX_DAG_DEPTH / 2;
        for (id, _) in nodes_by_depth.iter().take(to_remove) {
            if !self.tips.contains(id) {
                self.nodes.remove(id);
            }
        }
    }

    /// Статистика DAG
    pub fn stats(&self) -> DagStats {
        let total_rewards: f64 = self.balances.values().sum();
        let verified_count = self.nodes.values().filter(|n| n.verified).count();
        let avg_honesty = if self.nodes.is_empty() { 1.0 } else {
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
            richest_node: self.balances.iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(id, bal)| format!("{}: {:.4} монет", id, bal))
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
        write!(f,
            "╔══════════════════════════════════════════════════╗\n\
             ║  DAG CONSENSUS STATUS                            ║\n\
             ╠══════════════════════════════════════════════════╣\n\
             ║  Вершин:      {:>6}  Рёбер:    {:>6}          ║\n\
             ║  Tips:        {:>6}  Операций: {:>6}          ║\n\
             ║  Верифицировано: {:>6}/{:<6}                   ║\n\
             ║  Avg честность:  {:.4}                         ║\n\
             ║  Всего наград:   {:.4} монет                   ║\n\
             ║  Богатейший: {}  ║\n\
             ╚══════════════════════════════════════════════════╝",
            self.total_nodes, self.total_edges,
            self.tips_count, self.total_operations,
            self.verified_nodes, self.total_nodes,
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

        // Вторая запись
        let t3 = make_tensor("A", "D", 5.0, 0.99);
        let (node2, poa2) = dag.append_route(
            "node_BETA",
            vec!["A".into(), "D".into()],
            &[&t3],
            &mut trust,
            Some((3.0, 4.0)), // свидетели: L_AC=3ms, L_BC=4ms → L_AD должен быть в [1, 7]
        );
        println!("✅ Node 2:  id={} depth={} reward={:.4}", &node2.id[..8], node2.depth, poa2.net);
        println!("   Причина: {}", poa2.reason);

        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.tips.len(), 1); // node2 подтвердил node1, остался только node2 как tip

        let stats = dag.stats();
        println!("\n{}", stats);
    }

    #[test]
    fn test_poa_dishonest_node() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();

        // Лживый узел: заявляет 1ms, свидетели видят 20+25ms
        let t_lie = make_tensor("X", "Y", 1.0, 0.99);
        let (node, poa) = dag.append_route(
            "lying_node",
            vec!["X".into(), "Y".into()],
            &[&t_lie],
            &mut trust,
            Some((20.0, 25.0)), // Triangle Check провалится
        );

        assert!(poa.penalty > 0.0, "Должен быть штраф за ложь");
        assert!(node.honesty_score < 1.0, "Честность должна снизиться");
        println!("✅ PoA для лжеца: reward={:.4} penalty={:.4} net={:.4}", poa.reward, poa.penalty, poa.net);
        println!("   {}", poa.reason);
    }

    #[test]
    fn test_dag_chain_verification() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();
        let t = make_tensor("A", "B", 10.0, 0.99);

        let (n1, _) = dag.append_route("node_A", vec!["A".into(), "B".into()], &[&t], &mut trust, None);
        let (n2, _) = dag.append_route("node_A", vec!["A".into(), "B".into()], &[&t], &mut trust, None);

        println!("✅ Chain: n1.depth={} n2.depth={}", n1.depth, n2.depth);
        assert!(n2.depth > n1.depth, "Глубина должна расти");
    }

    #[test]
    fn test_balances_accumulate() {
        let mut dag = FederationDag::new();
        let mut trust = TrustRegistry::new();
        let t = make_tensor("A", "B", 10.0, 0.99);

        // 5 честных маршрутов от одного узла
        for _ in 0..5 {
            dag.append_route("rich_node", vec!["A".into(), "B".into()], &[&t], &mut trust, None);
        }

        let balance = dag.balances.get("rich_node").cloned().unwrap_or(0.0);
        assert!(balance > 0.0, "Баланс должен накапливаться");
        println!("✅ После 5 маршрутов баланс rich_node = {:.4} монет", balance);

        let stats = dag.stats();
        println!("   Всего наград выдано: {:.4}", stats.total_rewards_issued);
    }
}
EOF
