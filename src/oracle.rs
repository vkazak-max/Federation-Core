use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_ORACLE_CONFIRMATIONS: usize = 3;
pub const MAX_PRICE_DEVIATION: f64 = 0.05;
pub const ORACLE_TTL_SECS: u64 = 300;
pub const MAX_RESPONSE_SIZE: usize = 65536;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OracleRequestType {
    HttpGet { url: String, json_path: String },
    PriceFeed { asset: String, currency: String },
    DnsLookup { domain: String, record_type: String },
    Sensor { sensor_id: String, data_type: String },
    CensorshipProbe { target: String, region: String },
    NetworkLatency { target_ip: String, expected_ms: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRequest {
    pub id: String,
    pub requester_id: String,
    pub request_type: OracleRequestType,
    pub created_at: i64,
    pub expires_at: i64,
    pub min_confirmations: usize,
    pub commitment: String,
    pub is_private: bool,
}

impl OracleRequest {
    pub fn new(requester_id: &str, request_type: OracleRequestType, is_private: bool) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}", requester_id, now).bytes() {
            h ^= b as u64; h = h.wrapping_mul(0x100000001b3);
        }
        let id = format!("oracle_{:x}", h & 0xffffffff);
        let commitment = format!("{:x}", h.wrapping_mul(0xdeadbeef) & 0xffffffffffff);
        Self {
            id, requester_id: requester_id.to_string(),
            request_type, created_at: now,
            expires_at: now + ORACLE_TTL_SECS as i64 * 1000,
            min_confirmations: MIN_ORACLE_CONFIRMATIONS,
            commitment, is_private,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleResponse {
    pub request_id: String,
    pub responder_id: String,
    pub raw_value: String,
    pub numeric_value: Option<f64>,
    pub timestamp: i64,
    pub latency_ms: u64,
    pub source_hash: String,
    pub signature: String,
}

impl OracleResponse {
    pub fn new(request_id: &str, responder_id: &str, raw_value: &str, numeric_value: Option<f64>, latency_ms: u64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}", raw_value, now).bytes() {
            h ^= b as u64; h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            request_id: request_id.to_string(),
            responder_id: responder_id.to_string(),
            raw_value: raw_value.to_string(),
            numeric_value, timestamp: now, latency_ms,
            source_hash: format!("{:x}", h),
            signature: format!("sig_{}_{:x}", responder_id, h & 0xffff),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkContainer {
    pub container_id: String,
    pub request_id: String,
    pub commitment: String,
    pub consensus_value: String,
    pub numeric_consensus: Option<f64>,
    pub confirmations: usize,
    pub deviation_score: f64,
    pub is_valid: bool,
    pub validity_proof: String,
    pub created_at: i64,
    pub responders: Vec<String>,
}

impl ZkContainer {
    pub fn build(request_id: &str, responses: &[OracleResponse]) -> Option<Self> {
        if responses.len() < MIN_ORACLE_CONFIRMATIONS { return None; }
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;

        // Числовой консенсус — медиана
        let mut nums: Vec<f64> = responses.iter()
            .filter_map(|r| r.numeric_value).collect();
        let numeric_consensus = if !nums.is_empty() {
            nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
            Some(nums[nums.len() / 2])
        } else { None };

        // Текстовый консенсус — большинство
        let mut counts: HashMap<String, usize> = HashMap::new();
        for r in responses { *counts.entry(r.raw_value.clone()).or_insert(0) += 1; }
        let consensus_value = counts.iter()
            .max_by_key(|(_, c)| *c).map(|(v, _)| v.clone())
            .unwrap_or_default();

        // Deviation score
        let deviation_score = if let Some(median) = numeric_consensus {
            if median == 0.0 { 0.0 } else {
                let max_dev = nums.iter()
                    .map(|v| (v - median).abs() / median).fold(0.0_f64, f64::max);
                max_dev
            }
        } else { 0.0 };

        let is_valid = deviation_score <= MAX_PRICE_DEVIATION;

        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}", consensus_value, now).bytes() {
            h ^= b as u64; h = h.wrapping_mul(0x100000001b3);
        }

        Some(ZkContainer {
            container_id: format!("zkc_{:x}", h & 0xffffffff),
            request_id: request_id.to_string(),
            commitment: format!("{:x}", h.wrapping_mul(0xcafe) & 0xffffffffffff),
            consensus_value,
            numeric_consensus,
            confirmations: responses.len(),
            deviation_score,
            is_valid,
            validity_proof: format!("zkproof_{:x}", h & 0xffffff),
            created_at: now,
            responders: responses.iter().map(|r| r.responder_id.clone()).collect(),
        })
    }

    pub fn verify(&self) -> bool { self.is_valid && self.confirmations >= MIN_ORACLE_CONFIRMATIONS }
}

pub struct OracleConsensus {
    pub pending: HashMap<String, Vec<OracleResponse>>,
    pub completed: HashMap<String, ZkContainer>,
    pub total_requests: u64,
    pub total_completed: u64,
    pub total_failed: u64,
}

impl OracleConsensus {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(), completed: HashMap::new(),
            total_requests: 0, total_completed: 0, total_failed: 0,
        }
    }

    pub fn submit_response(&mut self, response: OracleResponse) -> Option<ZkContainer> {
        let req_id = response.request_id.clone();
        self.pending.entry(req_id.clone()).or_default().push(response);
        let responses = &self.pending[&req_id];
        if responses.len() >= MIN_ORACLE_CONFIRMATIONS {
            if let Some(container) = ZkContainer::build(&req_id, responses) {
                self.completed.insert(req_id.clone(), container.clone());
                self.pending.remove(&req_id);
                self.total_completed += 1;
                return Some(container);
            }
        }
        None
    }

    pub fn get_result(&self, request_id: &str) -> Option<&ZkContainer> {
        self.completed.get(request_id)
    }
}

impl Default for OracleConsensus { fn default() -> Self { Self::new() } }

pub struct OracleRegistry {
    pub requests: HashMap<String, OracleRequest>,
    pub consensus: OracleConsensus,
    pub node_id: String,
    pub subscriptions: Vec<String>,
}

impl OracleRegistry {
    pub fn new(node_id: &str) -> Self {
        Self {
            requests: HashMap::new(),
            consensus: OracleConsensus::new(),
            node_id: node_id.to_string(),
            subscriptions: vec![],
        }
    }

    pub fn request(&mut self, req_type: OracleRequestType, is_private: bool) -> OracleRequest {
        let req = OracleRequest::new(&self.node_id, req_type, is_private);
        self.requests.insert(req.id.clone(), req.clone());
        self.consensus.total_requests += 1;
        req
    }

    pub fn submit_response(&mut self, response: OracleResponse) -> Option<ZkContainer> {
        self.consensus.submit_response(response)
    }

    pub fn simulate_fetch(req_type: &OracleRequestType, responder_id: &str, request_id: &str) -> OracleResponse {
        let (raw, numeric, latency) = match req_type {
            OracleRequestType::PriceFeed { asset, .. } => {
                let base = match asset.as_str() {
                    "BTC" => 67500.0, "ETH" => 3200.0, "FED" => 1.0, _ => 100.0,
                };
                let mut h: u64 = 0xcbf29ce484222325;
                for b in responder_id.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
                let noise = ((h % 100) as f64 - 50.0) / 10000.0;
                let price = base * (1.0 + noise);
                (format!("{:.2}", price), Some(price), 45 + h % 30)
            }
            OracleRequestType::CensorshipProbe { target: _, region } => {
                let blocked = region == "CN" || region == "RU";
                (if blocked { "BLOCKED" } else { "ACCESSIBLE" }.to_string(), None, 120)
            }
            OracleRequestType::DnsLookup { domain: _, .. } => {
                (format!("192.168.{}.{}", 1, 100), None, 30)
            }
            OracleRequestType::NetworkLatency { expected_ms, .. } => {
                let mut h: u64 = 0xcbf29ce484222325;
                for b in responder_id.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
                let latency = expected_ms + (h % 20) as f64 - 10.0;
                (format!("{:.1}ms", latency), Some(latency), (latency as u64).max(1))
            }
            _ => ("OK".to_string(), None, 50),
        };
        OracleResponse::new(request_id, responder_id, &raw, numeric, latency)
    }

    pub fn stats(&self) -> OracleStats {
        OracleStats {
            total_requests: self.consensus.total_requests,
            completed: self.consensus.total_completed,
            pending: self.consensus.pending.len(),
            valid_containers: self.consensus.completed.values().filter(|c| c.is_valid).count(),
            invalid_containers: self.consensus.completed.values().filter(|c| !c.is_valid).count(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OracleStats {
    pub total_requests: u64,
    pub completed: u64,
    pub pending: usize,
    pub valid_containers: usize,
    pub invalid_containers: usize,
}

impl std::fmt::Display for OracleStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  PRIVATE ORACLES — REGISTRY STATUS           ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Запросов:    {:>4}  Выполнено: {:>4}         ║\n\
             ║  В процессе:  {:>4}                           ║\n\
             ║  ZK-контейнеров: {:>4} валидных               ║\n\
             ║  Невалидных:     {:>4}                        ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_requests, self.completed,
            self.pending, self.valid_containers, self.invalid_containers,
        )
    }
}
