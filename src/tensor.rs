cat > src/tensor.rs << 'EOF'
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SSAU_DIMENSIONS: usize = 5;
pub const TRUST_DECAY_ALPHA: f64 = 0.1;
pub const TRIANGLE_TOLERANCE: f64 = 0.05;

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
        const MAX_SAMPLES: usize = 50;
        self.samples.push(sample_ms);
        if self.samples.len() > MAX_SAMPLES {
            self.samples.remove(0);
        }
        self.recalculate();
    }

    fn recalculate(&mut self) {
        if self.samples.is_empty() { return; }
        let n = self.samples.len() as f64;
        self.mean = self.samples.iter().sum::<f64>() / n;
        let variance = self.samples.iter().map(|x| (x - self.mean).powi(2)).sum::<f64>() / n;
        self.std_dev = variance.sqrt();
    }
}

impl SsauTensor {
    pub fn new(from_node: &str, to_node: &str, latency_ms: f64, bandwidth_mbps: f64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        Self {
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            latency: LatencyDistribution::new(latency_ms, 0.0),
            jitter: 0.0,
            bandwidth: bandwidth_mbps,
            reliability: 1.0,
            energy_cost: 1.0,
            updated_at: now,
            version: 1,
        }
    }

    pub fn to_vector(&self) -> [f64; SSAU_DIMENSIONS] {
        [self.latency.mean, self.jitter, self.bandwidth, self.reliability, self.energy_cost]
    }

    pub fn update_measurement(&mut self, latency_ms: f64, bandwidth_mbps: f64) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let old_mean = self.latency.mean;
        self.latency.add_sample(latency_ms);
        self.jitter = (self.latency.mean - old_mean).abs();
        self.bandwidth = bandwidth_mbps;
        self.updated_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        self.version += 1;
    }

    pub fn is_fresh(&self, max_age_ms: i64) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        (now - self.updated_at) < max_age_ms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteHealthScore {
    pub entropy: f64,
    pub stability_score: f64,
    pub is_unstable: bool,
    pub total_latency_ms: f64,
    pub min_bandwidth_mbps: f64,
}

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
    let mut entropy = 0.0_f64;
    let mut total_latency = 0.0_f64;
    let mut min_bandwidth = f64::INFINITY;
    for tensor in tensors {
        let p = tensor.reliability.clamp(1e-10, 1.0 - 1e-10);
        entropy += -(p * p.log2());
        total_latency += tensor.latency.mean;
        if tensor.bandwidth < min_bandwidth { min_bandwidth = tensor.bandwidth; }
    }
    let max_entropy = tensors.len() as f64 * 1.0;
    let stability_score = if max_entropy > 0.0 {
        (1.0 - entropy / max_entropy).clamp(0.0, 1.0)
    } else { 1.0 };
    RouteHealthScore {
        entropy,
        stability_score,
        is_unstable: entropy > 0.5 * tensors.len() as f64,
        total_latency_ms: total_latency,
        min_bandwidth_mbps: if min_bandwidth.is_infinite() { 0.0 } else { min_bandwidth },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangleCheckResult {
    pub passed: bool,
    pub deviation_score: f64,
    pub violation: Option<String>,
    pub new_trust_weight: f64,
}

pub fn triangle_check(l_ab: f64, l_ac: f64, l_bc: f64, current_trust_weight: f64) -> TriangleCheckResult {
    let lower_bound = (l_ac - l_bc).abs();
    let upper_bound = l_ac + l_bc;
    let lower_with_tolerance = lower_bound * (1.0 - TRIANGLE_TOLERANCE);
    let upper_with_tolerance = upper_bound * (1.0 + TRIANGLE_TOLERANCE);
    let passed = l_ab >= lower_with_tolerance && l_ab <= upper_with_tolerance;
    let deviation_score = if passed { 0.0 }
        else if l_ab < lower_with_tolerance { (lower_with_tolerance - l_ab) / lower_with_tolerance.max(1.0) }
        else { (l_ab - upper_with_tolerance) / upper_with_tolerance.max(1.0) };
    let new_trust_weight = if passed {
        (current_trust_weight * (1.0 + TRUST_DECAY_ALPHA * 0.1)).min(1.0)
    } else {
        current_trust_weight * (-TRUST_DECAY_ALPHA * deviation_score).exp()
    };
    let violation = if !passed {
        Some(format!("Triangle violation: L_AB={:.2}ms outside [{:.2}, {:.2}]ms. Deviation: {:.1}%",
            l_ab, lower_with_tolerance, upper_with_tolerance, deviation_score * 100.0))
    } else { None };
    TriangleCheckResult { passed, deviation_score, violation, new_trust_weight }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustRegistry {
    scores: HashMap<String, f64>,
    check_counts: HashMap<String, u64>,
}

impl TrustRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn get_trust(&self, node_id: &str) -> f64 {
        *self.scores.get(node_id).unwrap_or(&1.0)
    }

    pub fn record_check(&mut self, node_id: &str, result: &TriangleCheckResult) {
        self.scores.insert(node_id.to_string(), result.new_trust_weight);
        *self.check_counts.entry(node_id.to_string()).or_insert(0) += 1;
    }

    pub fn penalize_unreachable(&mut self, node_id: &str) {
        let current = self.get_trust(node_id);
        let penalized = current * (-TRUST_DECAY_ALPHA * 2.0).exp();
        self.scores.insert(node_id.to_string(), penalized);
    }

    pub fn get_quarantined_nodes(&self, threshold: f64) -> Vec<String> {
        self.scores.iter().filter(|(_, &w)| w < threshold).map(|(id, _)| id.clone()).collect()
    }

    pub fn stats(&self) -> String {
        let total = self.scores.len();
        let high_trust = self.scores.values().filter(|&&w| w > 0.8).count();
        let quarantined = self.scores.values().filter(|&&w| w < 0.2).count();
        format!("Nodes: {}. High trust (>0.8): {}. Quarantined (<0.2): {}.", total, high_trust, quarantined)
    }
}
EOF
