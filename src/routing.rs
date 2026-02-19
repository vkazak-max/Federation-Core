use crate::tensor::{shannon_entropy, SsauTensor, TrustRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ENTROPY_SWITCH_THRESHOLD: f64 = 0.4;
pub const SWITCH_PROBABILITY_DELTA: f64 = 0.15;

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
    #[serde(skip)]
    pub tensors: Vec<SsauTensor>,
    pub raw_score: f64,
    pub softmax_probability: f64,
    pub total_latency_ms: f64,
    pub bottleneck_bandwidth_mbps: f64,
    pub health_score: f64,
    pub entropy: f64,
    pub total_cost: f64,
}

impl RouteCandidate {
    pub fn from_tensors(tensors: Vec<SsauTensor>, path: Vec<String>) -> Self {
        let total_latency: f64 = tensors.iter().map(|t| t.latency.mean).sum();
        let min_bw = tensors.iter().map(|t| t.bandwidth).fold(f64::INFINITY, f64::min);
        let total_cost: f64 = tensors.iter().map(|t| t.energy_cost).sum();
        let tensor_refs: Vec<&SsauTensor> = tensors.iter().collect();
        let health = shannon_entropy(&tensor_refs);
        RouteCandidate {
            path,
            tensors,
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
}

impl ScoringContext {
    pub fn from_candidates(candidates: &[RouteCandidate]) -> Self {
        let max_latency = candidates.iter().map(|c| c.total_latency_ms).fold(0.0_f64, f64::max);
        let max_bandwidth = candidates.iter().map(|c| c.bottleneck_bandwidth_mbps).fold(0.0_f64, f64::max);
        let max_cost = candidates.iter().map(|c| c.total_cost).fold(0.0_f64, f64::max);
        ScoringContext {
            max_latency_ms: max_latency.max(1.0),
            max_bandwidth_mbps: max_bandwidth.max(1.0),
            max_cost: max_cost.max(1.0),
        }
    }
}

pub fn score_route(candidate: &RouteCandidate, priorities: &UserPriorities, context: &ScoringContext) -> f64 {
    let latency_score = 1.0 - (candidate.total_latency_ms / context.max_latency_ms).min(1.0);
    let bandwidth_score = (candidate.bottleneck_bandwidth_mbps / context.max_bandwidth_mbps).min(1.0);
    let reliability_score = candidate.health_score;
    let anonymity_score = candidate.tensors.iter().map(|t| t.reliability).fold(1.0_f64, f64::min);
    let cost_score = 1.0 - (candidate.total_cost / context.max_cost).min(1.0);
    priorities.latency * latency_score
        + priorities.anonymity * anonymity_score
        + priorities.cost * cost_score
        + priorities.reliability * reliability_score
        + 0.1 * bandwidth_score
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
    pub active_entropy: HashMap<String, f64>,
}

impl AiRouter {
    pub fn new() -> Self {
        AiRouter { route_cache: HashMap::new(), active_entropy: HashMap::new() }
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
        let context = ScoringContext::from_candidates(&candidates);
        let raw_scores: Vec<f64> = candidates.iter().map(|c| score_route(c, &priorities, &context)).collect();
        let probabilities = softmax(&raw_scores);
        for (i, candidate) in candidates.iter_mut().enumerate() {
            candidate.raw_score = raw_scores[i];
            candidate.softmax_probability = probabilities[i];
        }
        candidates.sort_by(|a, b| b.softmax_probability.partial_cmp(&a.softmax_probability).unwrap());
        let best = candidates[0].clone();
        let chosen_entropy = best.entropy;
        let should_switch = self.check_should_switch(destination, &best, &candidates);
        let decision_reason = format!(
            "Route {:?} selected (P={:.3}, latency={:.1}ms, entropy={:.4}, health={:.3}). {}",
            best.path, best.softmax_probability, best.total_latency_ms, best.entropy, best.health_score,
            if should_switch { "⚠️ SWITCHING!" } else { "✅ Stable." }
        );
        let decision = RoutingDecision {
            chosen_route: Some(best),
            all_candidates: candidates,
            decision_reason,
            should_switch,
            chosen_entropy,
        };
        self.route_cache.insert(destination.to_string(), decision.clone());
        self.active_entropy.insert(destination.to_string(), chosen_entropy);
        decision
    }

    fn check_should_switch(&self, destination: &str, best: &RouteCandidate, _all: &[RouteCandidate]) -> bool {
        let current_entropy = self.active_entropy.get(destination).cloned().unwrap_or(0.0);
        if current_entropy > ENTROPY_SWITCH_THRESHOLD {
            log::warn!("⚠️ Entropy threshold exceeded for [{}]: {:.4}", destination, current_entropy);
            return true;
        }
        if let Some(cached) = self.route_cache.get(destination) {
            if let Some(ref current_route) = cached.chosen_route {
                if current_route.path != best.path {
                    let delta = best.softmax_probability - current_route.softmax_probability;
                    if delta > SWITCH_PROBABILITY_DELTA {
                        log::info!("🔄 Better route found for [{}]: delta_P={:.3}", destination, delta);
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn audit_active_routes(&self) -> Vec<String> {
        self.active_entropy.iter()
            .filter(|(_, &e)| e > ENTROPY_SWITCH_THRESHOLD)
            .map(|(d, _)| d.clone())
            .collect()
    }

    pub fn stats(&self) -> RouterStats {
        let avg_entropy = if self.active_entropy.is_empty() { 0.0 }
            else { self.active_entropy.values().sum::<f64>() / self.active_entropy.len() as f64 };
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
    dfs_paths(ssau_table, source, destination, &mut visited,
        &mut path_nodes, &mut path_tensors, &mut candidates, trust_registry, max_hops);
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
) {
    if path_nodes.len() > max_hops + 1 { return; }
    if current == destination && !path_tensors.is_empty() {
        candidates.push(RouteCandidate::from_tensors(path_tensors.clone(), path_nodes.clone()));
        return;
    }
    visited.insert(current.to_string());
    for tensor in ssau_table.values() {
        if tensor.from_node == current && !visited.contains(&tensor.to_node) {
            if trust_registry.get_trust(&tensor.to_node) < 0.2 { continue; }
            path_nodes.push(tensor.to_node.clone());
            path_tensors.push(tensor.clone());
            dfs_paths(ssau_table, &tensor.to_node.clone(), destination,
                visited, path_nodes, path_tensors, candidates, trust_registry, max_hops);
            path_nodes.pop();
            path_tensors.pop();
        }
    }
    visited.remove(current);
}
