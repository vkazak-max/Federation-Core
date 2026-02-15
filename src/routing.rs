use crate::tensor::{shannon_entropy, SsauTensor, TrustRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ENTROPY_SWITCH_THRESHOLD: f64 = 0.4;
pub const SWITCH_PROBABILITY_DELTA: f64 = 0.15;
pub const ENTROPY_IMPROVEMENT_DELTA: f64 = 0.08; // насколько энтропия должна стать лучше, чтобы оправдать switch

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPriorities {
    pub latency: f64,
    pub anonymity: f64,
    pub cost: f64,
    pub reliability: f64,
}

impl UserPriorities {
    pub fn speed_first() -> Self {
        UserPriorities { latency: 0.7, anonymity: 0.1, cost: 0.1, reliability: 0.1 }
    }
    pub fn anonymity_first() -> Self {
        UserPriorities { latency: 0.1, anonymity: 0.7, cost: 0.1, reliability: 0.1 }
    }
    pub fn balanced() -> Self {
        UserPriorities { latency: 0.3, anonymity: 0.2, cost: 0.2, reliability: 0.3 }
    }
    pub fn normalize(mut self) -> Self {
        let total = self.latency + self.anonymity + self.cost + self.reliability;
        if total > 0.0 {
            self.latency /= total;
            self.anonymity /= total;
            self.cost /= total;
            self.reliability /= total;
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteCandidate {
    pub path: Vec<String>,

    /// Внутренние данные (не сериализуем)
    #[serde(skip)]
    pub tensors: Vec<SsauTensor>,

    #[serde(skip)]
    pub min_trust: f64,

    pub raw_score: f64,
    pub softmax_probability: f64,

    pub total_latency_ms: f64,
    pub bottleneck_bandwidth_mbps: f64,
    pub health_score: f64,
    pub entropy: f64,
    pub total_cost: f64,
}

impl RouteCandidate {
    pub fn from_tensors(tensors: Vec<SsauTensor>, path: Vec<String>, min_trust: f64) -> Self {
        let total_latency: f64 = tensors.iter().map(|t| t.latency.mean).sum();
        let min_bw = tensors.iter().map(|t| t.bandwidth).fold(f64::INFINITY, f64::min);
        let total_cost: f64 = tensors.iter().map(|t| t.energy_cost).sum();
        let tensor_refs: Vec<&SsauTensor> = tensors.iter().collect();
        let health = shannon_entropy(&tensor_refs);

        RouteCandidate {
            path,
            tensors,
            min_trust,
            raw_score: 0.0,
            softmax_probability: 0.0,
            total_latency_ms: total_latency,
            bottleneck_bandwidth_mbps: if min_bw.is_infinite() { 0.0 } else { min_bw },
            health_score: health.stability_score,
            entropy: health.entropy,
            total_cost,
        }
    }
}

pub struct ScoringContext {
    pub max_latency_ms: f64,
    pub max_bandwidth_mbps: f64,
    pub max_cost: f64,
    pub max_hops: usize,
}

impl ScoringContext {
    pub fn from_candidates(candidates: &[RouteCandidate]) -> Self {
        let max_latency = candidates.iter().map(|c| c.total_latency_ms).fold(0.0_f64, f64::max);
        let max_bandwidth = candidates.iter().map(|c| c.bottleneck_bandwidth_mbps).fold(0.0_f64, f64::max);
        let max_cost = candidates.iter().map(|c| c.total_cost).fold(0.0_f64, f64::max);
        let max_hops = candidates.iter().map(|c| c.path.len()).fold(0_usize, usize::max);

        ScoringContext {
            max_latency_ms: max_latency.max(1.0),
            max_bandwidth_mbps: max_bandwidth.max(1.0),
            max_cost: max_cost.max(1.0),
            max_hops: max_hops.max(2),
        }
    }
}

/// MVP-анонимность: чем больше hop-ов, тем лучше.
/// Но если min_trust низкий — анонимность режется (узлы подозрительные).
fn anonymity_score(candidate: &RouteCandidate, ctx: &ScoringContext) -> f64 {
    // hops = nodes-1 (но берём длину path как proxy)
    let hops = candidate.path.len().saturating_sub(1).max(1) as f64;
    let max_hops = ctx.max_hops.saturating_sub(1).max(1) as f64;

    // 0..1
    let hop_score = (hops / max_hops).clamp(0.0, 1.0);

    // trust penalty: если min_trust < 0.5, режем сильнее
    let trust_penalty = if candidate.min_trust >= 0.8 {
        1.0
    } else if candidate.min_trust >= 0.5 {
        0.85
    } else if candidate.min_trust >= 0.2 {
        0.6
    } else {
        0.3
    };

    (hop_score * trust_penalty).clamp(0.0, 1.0)
}

pub fn score_route(candidate: &RouteCandidate, priorities: &UserPriorities, ctx: &ScoringContext) -> f64 {
    // Нормализация метрик
    let latency_score = 1.0 - (candidate.total_latency_ms / ctx.max_latency_ms).min(1.0);
    let bandwidth_score = (candidate.bottleneck_bandwidth_mbps / ctx.max_bandwidth_mbps).min(1.0);
    let reliability_score = candidate.health_score; // 0..1 (1=стабильно)
    let cost_score = 1.0 - (candidate.total_cost / ctx.max_cost).min(1.0);
    let anon_score = anonymity_score(candidate, ctx);

    // min_trust как общий множитель “качества маршрута”
    // (маршрут через подозрительные узлы хуже по всем приоритетам)
    let trust_factor = candidate.min_trust.clamp(0.0, 1.0);

    // Итоговый score
    let base =
        priorities.latency * latency_score
        + priorities.anonymity * anon_score
        + priorities.cost * cost_score
        + priorities.reliability * reliability_score
        + 0.1 * bandwidth_score;

    // trust_factor мягко режет результат
    (base * (0.6 + 0.4 * trust_factor)).clamp(0.0, 10.0)
}

pub fn softmax(scores: &[f64]) -> Vec<f64> {
    if scores.is_empty() { return vec![]; }
    let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exp_scores: Vec<f64> = scores.iter().map(|&s| (s - max_score).exp()).collect();
    let sum_exp: f64 = exp_scores.iter().sum();

    if sum_exp == 0.0 {
        return vec![1.0 / scores.len() as f64; scores.len()];
    }
    exp_scores.iter().map(|&e| e / sum_exp).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub chosen_route: Option<RouteCandidate>,
    pub all_candidates: Vec<RouteCandidate>,
    pub decision_reason: String,
    pub should_switch: bool,
    pub chosen_entropy: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RouterStats {
    pub cached_routes: usize,
    pub active_routes: usize,
    pub unstable_routes: usize,
    pub avg_entropy: f64,
}

pub struct AiRouter {
    route_cache: HashMap<String, RoutingDecision>,
    /// destination -> entropy выбранного маршрута
    pub active_entropy: HashMap<String, f64>,
    /// destination -> last probability (для сравнения)
    pub active_probability: HashMap<String, f64>,
}

impl AiRouter {
    pub fn new() -> Self {
        AiRouter {
            route_cache: HashMap::new(),
            active_entropy: HashMap::new(),
            active_probability: HashMap::new(),
        }
    }

    pub fn select_route(
        &mut self,
        destination: &str,
        mut candidates: Vec<RouteCandidate>,
        priorities: &UserPriorities,
    ) -> RoutingDecision {
        if candidates.is_empty() {
            return RoutingDecision {
                chosen_route: None,
                all_candidates: vec![],
                decision_reason: "No routes available".to_string(),
                should_switch: false,
                chosen_entropy: f64::INFINITY,
            };
        }

        let priorities = priorities.clone().normalize();
        let ctx = ScoringContext::from_candidates(&candidates);

        let raw_scores: Vec<f64> = candidates
            .iter()
            .map(|c| score_route(c, &priorities, &ctx))
            .collect();

        let probabilities = softmax(&raw_scores);

        for (i, candidate) in candidates.iter_mut().enumerate() {
            candidate.raw_score = raw_scores[i];
            candidate.softmax_probability = probabilities[i];
        }

        candidates.sort_by(|a, b| b.softmax_probability.partial_cmp(&a.softmax_probability).unwrap());
        let best = candidates[0].clone();

        let chosen_entropy = best.entropy;
        let chosen_prob = best.softmax_probability;

        let should_switch = self.check_should_switch(destination, &best);

        let decision_reason = format!(
            "Route {:?} selected (P={:.3}, latency={:.1}ms, entropy={:.4}, health={:.3}, min_trust={:.2}). {}",
            best.path,
            chosen_prob,
            best.total_latency_ms,
            best.entropy,
            best.health_score,
            best.min_trust,
            if should_switch { "⚠️ SWITCHING!" } else { "✅ Stable." }
        );

        let decision = RoutingDecision {
            chosen_route: Some(best.clone()),
            all_candidates: candidates,
            decision_reason,
            should_switch,
            chosen_entropy,
        };

        self.route_cache.insert(destination.to_string(), decision.clone());
        self.active_entropy.insert(destination.to_string(), chosen_entropy);
        self.active_probability.insert(destination.to_string(), chosen_prob);

        decision
    }

    fn check_should_switch(&self, destination: &str, best: &RouteCandidate) -> bool {
        let current_entropy = self.active_entropy.get(destination).cloned().unwrap_or(0.0);
        let current_prob = self.active_probability.get(destination).cloned().unwrap_or(0.0);

        // 1) если текущий маршрут “плох” по энтропии — надо менять
        if current_entropy > ENTROPY_SWITCH_THRESHOLD {
            log::warn!(
                "⚠️ Entropy threshold exceeded for [{}]: current={:.4}",
                destination,
                current_entropy
            );
            return true;
        }

        // 2) если новый маршрут существенно вероятнее — меняем
        if (best.softmax_probability - current_prob) > SWITCH_PROBABILITY_DELTA {
            log::info!(
                "🔄 Better route found for [{}]: delta_P={:.3}",
                destination,
                best.softmax_probability - current_prob
            );
            return true;
        }

        // 3) если новая энтропия заметно лучше (меньше) — меняем
        if (current_entropy - best.entropy) > ENTROPY_IMPROVEMENT_DELTA {
            log::info!(
                "🔄 Entropy improvement for [{}]: {:.4} -> {:.4}",
                destination,
                current_entropy,
                best.entropy
            );
            return true;
        }

        false
    }

    pub fn audit_active_routes(&self) -> Vec<String> {
        self.active_entropy
            .iter()
            .filter(|(_, &e)| e > ENTROPY_SWITCH_THRESHOLD)
            .map(|(d, _)| d.clone())
            .collect()
    }

    pub fn stats(&self) -> RouterStats {
        let avg_entropy = if self.active_entropy.is_empty() {
            0.0
        } else {
            self.active_entropy.values().sum::<f64>() / self.active_entropy.len() as f64
        };

        RouterStats {
            cached_routes: self.route_cache.len(),
            active_routes: self.active_entropy.len(),
            unstable_routes: self.audit_active_routes().len(),
            avg_entropy,
        }
    }
}

impl Default for AiRouter {
    fn default() -> Self { Self::new() }
}

// -----------------------------------------------------------------------------
// Candidate builder (DFS paths)
// -----------------------------------------------------------------------------

pub fn build_route_candidates(
    ssau_table: &HashMap<String, SsauTensor>,
    source: &str,
    destination: &str,
    trust_registry: &TrustRegistry,
    max_hops: usize,
) -> Vec<RouteCandidate> {
    let mut candidates = vec![];
    let mut visited = std::collections::HashSet::new();
    let mut path_nodes = vec![source.to_string()];
    let mut path_tensors = vec![];
    let mut path_min_trust = 1.0_f64;

    dfs_paths(
        ssau_table,
        source,
        destination,
        &mut visited,
        &mut path_nodes,
        &mut path_tensors,
        &mut candidates,
        trust_registry,
        max_hops,
        &mut path_min_trust,
    );

    candidates
}

fn dfs_paths(
    ssau_table: &HashMap<String, SsauTensor>,
    current: &str,
    destination: &str,
    visited: &mut std::collections::HashSet<String>,
    path_nodes: &mut Vec<String>,
    path_tensors: &mut Vec<SsauTensor>,
    candidates: &mut Vec<RouteCandidate>,
    trust_registry: &TrustRegistry,
    max_hops: usize,
    path_min_trust: &mut f64,
) {
    if path_nodes.len() > max_hops + 1 { return; }

    if current == destination && !path_tensors.is_empty() {
        candidates.push(RouteCandidate::from_tensors(
            path_tensors.clone(),
            path_nodes.clone(),
            *path_min_trust,
        ));
        return;
    }

    visited.insert(current.to_string());

    for (_, tensor) in ssau_table {
        if tensor.from_node == current && !visited.contains(&tensor.to_node) {
            let node_trust = trust_registry.get_trust(&tensor.to_node);

            // Жёсткое отсечение “карантина”
            if node_trust < 0.2 {
                continue;
            }

            // push
            path_nodes.push(tensor.to_node.clone());
            path_tensors.push(tensor.clone());

            // обновляем min_trust по пути
            let prev_min = *path_min_trust;
            *path_min_trust = (*path_min_trust).min(node_trust);

            dfs_paths(
                ssau_table,
                &tensor.to_node,
                destination,
                visited,
                path_nodes,
                path_tensors,
                candidates,
                trust_registry,
                max_hops,
                path_min_trust,
            );

            // pop
            *path_min_trust = prev_min;
            path_nodes.pop();
            path_tensors.pop();
        }
    }

    visited.remove(current);
}
