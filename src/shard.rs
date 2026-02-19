use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_SHARD_SIZE: usize = 3;
pub const MAX_SHARD_SIZE: usize = 50;
pub const SPLIT_THRESHOLD: f64 = 0.4;
pub const MERGE_THRESHOLD: f64 = 0.7;
pub const RENDEZVOUS_TTL: u8 = 5;
pub const BRIDGE_ELECTION_QUORUM: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum ThreatLevel {
    #[default]
    Normal,
    Elevated,
    Critical,
    Fragmentation,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkEvent {
    NodeDropped { node_id: String, region: String },
    RegionBlocked { region: String, blocked_ports: Vec<u16> },
    BgpAnomaly { affected_prefixes: Vec<String>, severity: f64 },
    MassDisconnect { node_count: usize, simultaneous: bool },
    DnsBlocking { domains: Vec<String>, region: String },
    DeepPacketBlock { signature: String, region: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    pub score: f64,
    pub threat_level: ThreatLevel,
    pub should_shard: bool,
    pub reasons: Vec<String>,
    pub blocked_regions: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ShardDetector {
    pub events: Vec<NetworkEvent>,
    pub dropped_nodes: HashMap<String, usize>,
    pub blocked_regions: Vec<String>,
    pub threat_level: ThreatLevel,
    pub threat_score: f64,
}

impl ShardDetector {
    pub fn new() -> Self { Self::default() }

    pub fn record_event(&mut self, event: NetworkEvent) -> ThreatAssessment {
        match &event {
            NetworkEvent::NodeDropped { region, .. } => {
                *self.dropped_nodes.entry(region.clone()).or_insert(0) += 1;
            }
            NetworkEvent::RegionBlocked { region, .. } => {
                if !self.blocked_regions.contains(region) {
                    self.blocked_regions.push(region.clone());
                }
            }
            NetworkEvent::MassDisconnect { node_count, simultaneous } => {
                if *simultaneous && *node_count > 5 { self.threat_score += 0.3; }
            }
            _ => {}
        }
        self.events.push(event);
        self.assess()
    }

    fn assess(&mut self) -> ThreatAssessment {
        let mut score = 0.0_f64;
        let mut reasons = vec![];
        let blocked = self.blocked_regions.len();
        if blocked > 0 {
            score += 0.2 * blocked as f64;
            reasons.push(format!("{} регион(ов) заблокировано", blocked));
        }
        let total_dropped: usize = self.dropped_nodes.values().sum();
        if total_dropped > 3 {
            score += 0.1 * total_dropped as f64;
            reasons.push(format!("{} узлов отключено", total_dropped));
        }
        let bgp = self.events.iter()
            .filter(|e| matches!(e, NetworkEvent::BgpAnomaly { .. })).count();
        if bgp > 0 {
            score += 0.25 * bgp as f64;
            reasons.push(format!("{} BGP аномалий", bgp));
        }
        score += self.threat_score;
        score = score.min(1.0);
        self.threat_level = match score {
            s if s < 0.2  => ThreatLevel::Normal,
            s if s < 0.5  => ThreatLevel::Elevated,
            s if s < 0.75 => ThreatLevel::Critical,
            _             => ThreatLevel::Fragmentation,
        };
        ThreatAssessment {
            score,
            threat_level: self.threat_level.clone(),
            should_shard: score >= SPLIT_THRESHOLD,
            reasons,
            blocked_regions: self.blocked_regions.clone(),
        }
    }

    pub fn current_assessment(&mut self) -> ThreatAssessment { self.assess() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    pub id: String,
    pub nodes: Vec<String>,
    pub region_focus: String,
    pub bridge_nodes: Vec<String>,
    pub created_at: i64,
    pub generation: u32,
    pub parent_shard: Option<String>,
    pub is_isolated: bool,
    pub rendezvous_key: String,
}

impl Shard {
    pub fn new(id: &str, nodes: Vec<String>, region: &str, generation: u32) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}", id, now).bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Shard {
            id: id.to_string(), nodes,
            region_focus: region.to_string(),
            bridge_nodes: vec![],
            created_at: now, generation,
            parent_shard: None, is_isolated: false,
            rendezvous_key: format!("rdv_{:x}", h & 0xffffffffffff),
        }
    }

    pub fn size(&self) -> usize { self.nodes.len() }

    pub fn elect_bridges(&mut self) {
        let n = (self.nodes.len() / 3).max(1).min(BRIDGE_ELECTION_QUORUM);
        self.bridge_nodes = self.nodes.iter().take(n).cloned().collect();
    }

    pub fn can_merge_with(&self, other: &Shard) -> bool {
        self.size() + other.size() <= MAX_SHARD_SIZE
    }
}

pub struct ShardManager {
    pub shards: HashMap<String, Shard>,
    pub split_count: u64,
    pub merge_count: u64,
    pub generation: u32,
}

impl ShardManager {
    pub fn new() -> Self {
        ShardManager { shards: HashMap::new(), split_count: 0, merge_count: 0, generation: 0 }
    }

    pub fn split(&mut self, nodes: &[String], assessment: &ThreatAssessment) -> Vec<Shard> {
        self.generation += 1;
        let gen = self.generation;
        let shard_size = match assessment.threat_level {
            ThreatLevel::Fragmentation => MIN_SHARD_SIZE,
            ThreatLevel::Critical      => (nodes.len() / 4).max(MIN_SHARD_SIZE),
            ThreatLevel::Elevated      => (nodes.len() / 2).max(MIN_SHARD_SIZE),
            ThreatLevel::Normal        => nodes.len(),
        };
        let mut new_shards = vec![];
        for (idx, chunk) in nodes.chunks(shard_size.max(1)).enumerate() {
            let sid = format!("shard_g{}_{}", gen, idx);
            let mut shard = Shard::new(
                &sid, chunk.to_vec(),
                &assessment.blocked_regions.first().cloned().unwrap_or("GLOBAL".into()),
                gen,
            );
            shard.is_isolated = assessment.threat_level == ThreatLevel::Fragmentation;
            shard.elect_bridges();
            self.shards.insert(sid.clone(), shard.clone());
            new_shards.push(shard);
        }
        self.split_count += 1;
        new_shards
    }

    pub fn merge(&mut self, a_id: &str, b_id: &str) -> Option<Shard> {
        let a = self.shards.remove(a_id)?;
        let b = self.shards.remove(b_id)?;
        if !a.can_merge_with(&b) {
            self.shards.insert(a_id.to_string(), a);
            self.shards.insert(b_id.to_string(), b);
            return None;
        }
        let mut nodes = a.nodes.clone();
        nodes.extend(b.nodes.clone());
        nodes.dedup();
        let mid = format!("shard_merged_g{}", self.generation);
        let mut merged = Shard::new(&mid, nodes, &a.region_focus, self.generation);
        merged.parent_shard = Some(a_id.to_string());
        merged.elect_bridges();
        self.shards.insert(mid.clone(), merged.clone());
        self.merge_count += 1;
        Some(merged)
    }

    pub fn find_mergeable_pairs(&self) -> Vec<(String, String)> {
        let ids: Vec<String> = self.shards.keys().cloned().collect();
        let mut pairs = vec![];
        for i in 0..ids.len() {
            for j in (i+1)..ids.len() {
                if self.shards[&ids[i]].can_merge_with(&self.shards[&ids[j]]) {
                    pairs.push((ids[i].clone(), ids[j].clone()));
                }
            }
        }
        pairs
    }

    pub fn stats(&self) -> ShardStats {
        let sizes: Vec<usize> = self.shards.values().map(|s| s.size()).collect();
        let avg = if sizes.is_empty() { 0.0 }
            else { sizes.iter().sum::<usize>() as f64 / sizes.len() as f64 };
        ShardStats {
            total_shards: self.shards.len(),
            total_nodes: sizes.iter().sum(),
            avg_shard_size: avg,
            min_shard_size: sizes.iter().cloned().min().unwrap_or(0),
            max_shard_size: sizes.iter().cloned().max().unwrap_or(0),
            split_count: self.split_count,
            merge_count: self.merge_count,
            generation: self.generation,
        }
    }
}

impl Default for ShardManager {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardStats {
    pub total_shards: usize,
    pub total_nodes: usize,
    pub avg_shard_size: f64,
    pub min_shard_size: usize,
    pub max_shard_size: usize,
    pub split_count: u64,
    pub merge_count: u64,
    pub generation: u32,
}

impl std::fmt::Display for ShardStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  DYNAMIC SHARDING STATUS                     ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Шардов:     {:>4}  Узлов всего: {:>6}      ║\n\
             ║  Avg размер: {:>6.1}  Min: {:>3}  Max: {:>3}   ║\n\
             ║  Splits:     {:>4}  Merges:     {:>4}        ║\n\
             ║  Generation: {:>4}                           ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_shards, self.total_nodes,
            self.avg_shard_size, self.min_shard_size, self.max_shard_size,
            self.split_count, self.merge_count, self.generation,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RendezvousBeacon {
    pub shard_id: String,
    pub rendezvous_key: String,
    pub bridge_nodes: Vec<String>,
    pub generation: u32,
    pub ttl: u8,
}

pub struct RendezvousProtocol {
    pub pending_beacons: Vec<RendezvousBeacon>,
    pub matched_pairs: Vec<(String, String)>,
}

impl RendezvousProtocol {
    pub fn new() -> Self {
        RendezvousProtocol { pending_beacons: vec![], matched_pairs: vec![] }
    }

    pub fn publish_beacon(&mut self, shard: &Shard) {
        self.pending_beacons.push(RendezvousBeacon {
            shard_id: shard.id.clone(),
            rendezvous_key: shard.rendezvous_key.clone(),
            bridge_nodes: shard.bridge_nodes.clone(),
            generation: shard.generation,
            ttl: RENDEZVOUS_TTL,
        });
    }

    pub fn find_matches(&mut self) -> Vec<(String, String)> {
        let mut matches = vec![];
        for i in 0..self.pending_beacons.len() {
            for j in (i+1)..self.pending_beacons.len() {
                let a = &self.pending_beacons[i];
                let b = &self.pending_beacons[j];
                if a.generation.abs_diff(b.generation) <= 1 {
                    matches.push((a.shard_id.clone(), b.shard_id.clone()));
                }
            }
        }
        self.matched_pairs = matches.clone();
        matches
    }
}

impl Default for RendezvousProtocol {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkState {
    Unified,
    Splitting,
    Sharded,
    Merging,
}

pub struct DynamicNetwork {
    pub all_nodes: Vec<String>,
    pub detector: ShardDetector,
    pub manager: ShardManager,
    pub rendezvous: RendezvousProtocol,
    pub state: NetworkState,
}

impl DynamicNetwork {
    pub fn new(nodes: Vec<String>) -> Self {
        DynamicNetwork {
            all_nodes: nodes,
            detector: ShardDetector::new(),
            manager: ShardManager::new(),
            rendezvous: RendezvousProtocol::new(),
            state: NetworkState::Unified,
        }
    }

    pub fn handle_threat(&mut self, event: NetworkEvent) -> ShardingAction {
        let assessment = self.detector.record_event(event);
        if assessment.should_shard && self.state == NetworkState::Unified {
            self.state = NetworkState::Splitting;
            let shards = self.manager.split(&self.all_nodes, &assessment);
            self.state = NetworkState::Sharded;
            return ShardingAction::Split {
                shards_created: shards.len(), assessment, shards,
            };
        }
        ShardingAction::Monitor { assessment }
    }

    pub fn attempt_recovery(&mut self) -> ShardingAction {
        if self.state != NetworkState::Sharded { return ShardingAction::NoAction; }
        let ids: Vec<String> = self.manager.shards.keys().cloned().collect();
        for id in &ids {
            if let Some(shard) = self.manager.shards.get(id) {
                self.rendezvous.publish_beacon(shard);
            }
        }
        let matches = self.rendezvous.find_matches();
        if matches.is_empty() { return ShardingAction::NoAction; }
        self.state = NetworkState::Merging;
        let mut merged_count = 0;
        for (a_id, b_id) in &matches {
            if self.manager.shards.contains_key(a_id.as_str())
                && self.manager.shards.contains_key(b_id.as_str())
                && self.manager.merge(a_id, b_id).is_some() { merged_count += 1; }
        }
        if self.manager.shards.len() <= 1 { self.state = NetworkState::Unified; }
        ShardingAction::Merge {
            merges_performed: merged_count,
            remaining_shards: self.manager.shards.len(),
        }
    }

    pub fn status(&self) -> String {
        format!("NetworkState: {:?} | Shards: {} | Nodes: {} | Gen: {}",
            self.state, self.manager.shards.len(),
            self.all_nodes.len(), self.manager.generation)
    }
}

#[derive(Debug)]
pub enum ShardingAction {
    Split { shards_created: usize, assessment: ThreatAssessment, shards: Vec<Shard> },
    Merge { merges_performed: usize, remaining_shards: usize },
    Monitor { assessment: ThreatAssessment },
    NoAction,
}
