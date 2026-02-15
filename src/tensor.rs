use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Константы
// =============================================================================

pub const SSAU_DIMENSIONS: usize = 5;

/// Насколько быстро “тает” доверие при нарушениях (и насколько растёт при честности)
pub const TRUST_DECAY_ALPHA: f64 = 0.10;

/// Допуск для Triangle inequality (5%)
pub const TRIANGLE_TOLERANCE: f64 = 0.05;

/// Сколько сэмплов latency храним (скользящее окно)
pub const MAX_LATENCY_SAMPLES: usize = 50;

// =============================================================================
// Helpers
// =============================================================================

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

pub fn tensor_key(from: &str, to: &str) -> String {
    format!("{}→{}", from, to)
}

fn clamp_prob(p: f64) -> f64 {
    // чтобы log не взрывался
    p.clamp(1e-12, 1.0 - 1e-12)
}

// =============================================================================
// SSAU Tensor
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsauTensor {
    pub from_node: String,
    pub to_node: String,
    pub latency: LatencyDistribution,
    pub jitter: f64,
    pub bandwidth: f64,
    pub reliability: f64,
    pub energy_cost: f64,
    pub updated_at: i64,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub samples: Vec<f64>,
}

impl LatencyDistribution {
    pub fn new(mean: f64, std_dev: f64) -> Self {
        Self { mean, std_dev, samples: Vec::new() }
    }

    pub fn add_sample(&mut self, sample_ms: f64) {
        self.samples.push(sample_ms.max(0.0));
        if self.samples.len() > MAX_LATENCY_SAMPLES {
            self.samples.remove(0);
        }
        self.recalculate();
    }

    fn recalculate(&mut self) {
        if self.samples.is_empty() {
            return;
        }
        let n = self.samples.len() as f64;
        let mean = self.samples.iter().sum::<f64>() / n;

        let var = self
            .samples
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / n;

        self.mean = mean;
        self.std_dev = var.sqrt();
    }
}

impl SsauTensor {
    pub fn new(from_node: &str, to_node: &str, latency_ms: f64, bandwidth_mbps: f64) -> Self {
        let now = now_ms();
        Self {
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            latency: LatencyDistribution::new(latency_ms.max(0.0), 0.0),
            jitter: 0.0,
            bandwidth: bandwidth_mbps.max(0.0),
            reliability: 1.0,
            energy_cost: 1.0,
            updated_at: now,
            version: 1,
        }
    }

    pub fn to_vector(&self) -> [f64; SSAU_DIMENSIONS] {
        [
            self.latency.mean,
            self.jitter,
            self.bandwidth,
            self.reliability,
            self.energy_cost,
        ]
    }

    pub fn update_measurement(&mut self, latency_ms: f64, bandwidth_mbps: f64) {
        let old_mean = self.latency.mean;
        self.latency.add_sample(latency_ms);
        self.jitter = (self.latency.mean - old_mean).abs();
        self.bandwidth = bandwidth_mbps.max(0.0);
        self.updated_at = now_ms();
        self.version += 1;
    }

    pub fn is_fresh(&self, max_age_ms: i64) -> bool {
        (now_ms() - self.updated_at) < max_age_ms
    }
}

// =============================================================================
// Shannon entropy / Route health
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHealthScore {
    pub entropy: f64,
    pub stability_score: f64,
    pub is_unstable: bool,
    pub total_latency_ms: f64,
    pub min_bandwidth_mbps: f64,
}

/// Энтропия маршрута на основе reliability каждого ребра.
///
/// Для каждого ребра считаем энтропию Бернулли:
///   H(p) = -p log2 p - (1-p) log2 (1-p)
///
/// p = reliability (вероятность “успеха”/стабильности)
///
/// Чем выше энтропия — тем “более неопределённый” маршрут.
pub fn shannon_entropy(tensors: &[&SsauTensor]) -> RouteHealthScore {
    if tensors.is_empty() {
        return RouteHealthScore {
            entropy: f64::INFINITY,
            stability_score: 0.0,
            is_unstable: true,
            total_latency_ms: f64::INFINITY,
            min_bandwidth_mbps: 0.0,
        };
    }

    let mut entropy_sum = 0.0_f64;
    let mut total_latency = 0.0_f64;
    let mut min_bandwidth = f64::INFINITY;

    for t in tensors {
        let p = clamp_prob(t.reliability);
        let q = clamp_prob(1.0 - p);

        // H(p)
        let h = -(p * p.log2()) - (q * q.log2());
        entropy_sum += h;

        total_latency += t.latency.mean;
        if t.bandwidth < min_bandwidth {
            min_bandwidth = t.bandwidth;
        }
    }

    // Нормируем: максимум энтропии для Бернулли = 1.0 на ребро (при p=0.5)
    let max_entropy = tensors.len() as f64 * 1.0;

    // stability_score: 1 = идеально стабильно (энтропия низкая), 0 = максимальная неопределённость
    let stability_score = (1.0 - (entropy_sum / max_entropy)).clamp(0.0, 1.0);

    // критерий “нестабильно”: если стабильность ниже 0.6
    let is_unstable = stability_score < 0.60;

    RouteHealthScore {
        entropy: entropy_sum,
        stability_score,
        is_unstable,
        total_latency_ms: total_latency,
        min_bandwidth_mbps: if min_bandwidth.is_infinite() { 0.0 } else { min_bandwidth },
    }
}

// =============================================================================
// Triangle Check (lie detector)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangleCheckResult {
    pub passed: bool,
    /// 0..1 насколько “сильно” нарушили границы (0 = ок, 1 = очень плохо)
    pub deviation_score: f64,
    pub violation: Option<String>,
    pub new_trust_weight: f64,
}

/// Triangle inequality (latency):
///   |L_AC - L_BC| <= L_AB <= L_AC + L_BC
/// + допуск TRIANGLE_TOLERANCE
pub fn triangle_check(
    l_ab: f64,
    l_ac: f64,
    l_bc: f64,
    current_trust_weight: f64,
) -> TriangleCheckResult {
    let l_ab = l_ab.max(0.0);
    let l_ac = l_ac.max(0.0);
    let l_bc = l_bc.max(0.0);

    let lower = (l_ac - l_bc).abs();
    let upper = l_ac + l_bc;

    let lower_t = lower * (1.0 - TRIANGLE_TOLERANCE);
    let upper_t = upper * (1.0 + TRIANGLE_TOLERANCE);

    let passed = l_ab >= lower_t && l_ab <= upper_t;

    let deviation_score = if passed {
        0.0
    } else if l_ab < lower_t {
        // насколько “ниже” нижней границы (в процентах от диапазона)
        let denom = (upper_t - lower_t).max(1.0);
        ((lower_t - l_ab) / denom).clamp(0.0, 1.0)
    } else {
        let denom = (upper_t - lower_t).max(1.0);
        ((l_ab - upper_t) / denom).clamp(0.0, 1.0)
    };

    // Обновление trust:
    // - если passed: чуть растим
    // - если failed: экспоненциально штрафуем по deviation_score
    let mut new_trust = current_trust_weight.clamp(0.0, 1.0);
    if passed {
        new_trust = (new_trust * (1.0 + TRUST_DECAY_ALPHA * 0.10)).min(1.0);
    } else {
        // штраф: чем больше deviation, тем сильнее падение
        new_trust = new_trust * (-TRUST_DECAY_ALPHA * (0.5 + 2.0 * deviation_score)).exp();
        new_trust = new_trust.clamp(0.01, 1.0);
    }

    let violation = if !passed {
        Some(format!(
            "Triangle violation: L_AB={:.2}ms outside [{:.2}, {:.2}]ms. Deviation={:.1}%",
            l_ab,
            lower_t,
            upper_t,
            deviation_score * 100.0
        ))
    } else {
        None
    };

    TriangleCheckResult {
        passed,
        deviation_score,
        violation,
        new_trust_weight: new_trust,
    }
}

// =============================================================================
// TrustRegistry
// =============================================================================

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustRegistry {
    scores: HashMap<String, f64>,
    check_counts: HashMap<String, u64>,
}

impl TrustRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_trust(&self, node_id: &str) -> f64 {
        *self.scores.get(node_id).unwrap_or(&1.0)
    }

    pub fn record_check(&mut self, node_id: &str, result: &TriangleCheckResult) {
        self.scores.insert(node_id.to_string(), result.new_trust_weight);
        *self.check_counts.entry(node_id.to_string()).or_insert(0) += 1;
    }

    pub fn penalize_unreachable(&mut self, node_id: &str) {
        let current = self.get_trust(node_id).clamp(0.0, 1.0);
        // “недоступен” — сильнее штраф, чем обычное отклонение
        let penalized = (current * (-TRUST_DECAY_ALPHA * 2.0).exp()).clamp(0.01, 1.0);
        self.scores.insert(node_id.to_string(), penalized);
    }

    pub fn get_quarantined_nodes(&self, threshold: f64) -> Vec<String> {
        self.scores
            .iter()
            .filter(|(_, &w)| w < threshold)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn stats(&self) -> String {
        let total = self.scores.len();
        let high_trust = self.scores.values().filter(|&&w| w > 0.8).count();
        let quarantined = self.scores.values().filter(|&&w| w < 0.2).count();
        format!(
            "Nodes: {}. High trust (>0.8): {}. Quarantined (<0.2): {}.",
            total, high_trust, quarantined
        )
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_low_for_reliable() {
        let t1 = SsauTensor::new("A", "B", 10.0, 1000.0);
        let mut t2 = SsauTensor::new("B", "C", 12.0, 800.0);
        t2.reliability = 0.99;

        let refs: Vec<&SsauTensor> = vec![&t1, &t2];
        let h = shannon_entropy(&refs);

        assert!(h.stability_score > 0.8);
        assert!(!h.is_unstable);
    }

    #[test]
    fn test_triangle_check_pass() {
        // L_AC=10, L_BC=5 => L_AB must be in [5,15] (примерно)
        let r = triangle_check(12.0, 10.0, 5.0, 1.0);
        assert!(r.passed);
        assert!(r.deviation_score == 0.0);
        assert!(r.new_trust_weight <= 1.0);
    }

    #[test]
    fn test_triangle_check_fail_decreases_trust() {
        let r = triangle_check(100.0, 10.0, 5.0, 1.0);
        assert!(!r.passed);
        assert!(r.deviation_score > 0.0);
        assert!(r.new_trust_weight < 1.0);
    }

    #[test]
    fn test_latency_distribution_updates() {
        let mut ld = LatencyDistribution::new(10.0, 0.0);
        ld.add_sample(10.0);
        ld.add_sample(20.0);
        assert!(ld.mean > 10.0 && ld.mean < 20.0);
        assert!(ld.std_dev > 0.0);
    }
}
