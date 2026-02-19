use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_SWARM_MEMORY: usize = 10000;
pub const GOSSIP_FAN_OUT: usize = 3;
pub const EXPERIENCE_TTL: u8 = 7;
pub const MIN_TRUST_FOR_EXPERIENCE: f64 = 0.5;
pub const CONFIRMATION_THRESHOLD: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExperienceType {
    CensorshipBypass { region: String, method: String, success_rate: f64 },
    AttackPattern { attacker_fingerprint: String, attack_type: String, severity: f64 },
    OptimalRoute { destination_region: String, via_nodes: Vec<String>, avg_latency_ms: f64, reliability: f64 },
    BadActor { node_id: String, violation_type: String, evidence_hash: String },
    NetworkAnomaly { affected_region: String, anomaly_type: String, impact_score: f64 },
    DpiBypass { technique: String, tested_in_regions: Vec<String>, effectiveness: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub origin_node: String,
    pub origin_region: String,
    pub experience_type: ExperienceType,
    pub created_at: i64,
    pub ttl: u8,
    pub confirmations: usize,
    pub confirmed_by: Vec<String>,
    pub utility_score: f64,
    pub signature: String,
}

impl Experience {
    pub fn new(origin_node: &str, origin_region: &str, experience_type: ExperienceType) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}{}", origin_node, now, origin_region).bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let id = format!("exp_{:x}", h);
        let utility_score = match &experience_type {
            ExperienceType::CensorshipBypass { success_rate, .. } => *success_rate,
            ExperienceType::AttackPattern { severity, .. } => *severity,
            ExperienceType::OptimalRoute { reliability, .. } => *reliability,
            ExperienceType::BadActor { .. } => 0.9,
            ExperienceType::NetworkAnomaly { impact_score, .. } => *impact_score,
            ExperienceType::DpiBypass { effectiveness, .. } => *effectiveness,
        };
        Experience {
            id, origin_node: origin_node.to_string(),
            origin_region: origin_region.to_string(),
            experience_type, created_at: now,
            ttl: EXPERIENCE_TTL, confirmations: 1,
            confirmed_by: vec![origin_node.to_string()],
            utility_score, signature: format!("sig_{}", origin_node),
        }
    }

    pub fn confirm(&mut self, confirming_node: &str) {
        if !self.confirmed_by.contains(&confirming_node.to_string()) {
            self.confirmed_by.push(confirming_node.to_string());
            self.confirmations += 1;
            self.utility_score = (self.utility_score * 0.9
                + 0.1 * (self.confirmations as f64 / 10.0).min(1.0)).min(1.0);
        }
    }

    pub fn is_verified(&self) -> bool { self.confirmations >= CONFIRMATION_THRESHOLD }

    pub fn propagate(&mut self) -> bool {
        if self.ttl > 0 { self.ttl -= 1; true } else { false }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SwarmMemory {
    experiences: HashMap<String, Experience>,
    type_index: HashMap<String, Vec<String>>,
    region_index: HashMap<String, Vec<String>>,
    pub blacklist: Vec<String>,
    pub total_received: u64,
    pub total_applied: u64,
}

impl SwarmMemory {
    pub fn new() -> Self { Self::default() }

    pub fn absorb(&mut self, exp: Experience) -> bool {
        if let Some(existing) = self.experiences.get_mut(&exp.id) {
            for confirmer in &exp.confirmed_by { existing.confirm(confirmer); }
            return false;
        }
        if let ExperienceType::BadActor { ref node_id, .. } = exp.experience_type {
            if !self.blacklist.contains(node_id) { self.blacklist.push(node_id.clone()); }
        }
        let type_key = match &exp.experience_type {
            ExperienceType::CensorshipBypass { .. } => "censorship_bypass",
            ExperienceType::AttackPattern { .. }    => "attack_pattern",
            ExperienceType::OptimalRoute { .. }     => "optimal_route",
            ExperienceType::BadActor { .. }         => "bad_actor",
            ExperienceType::NetworkAnomaly { .. }   => "network_anomaly",
            ExperienceType::DpiBypass { .. }        => "dpi_bypass",
        }.to_string();
        self.type_index.entry(type_key).or_default().push(exp.id.clone());
        self.region_index.entry(exp.origin_region.clone()).or_default().push(exp.id.clone());
        self.total_received += 1;
        let id = exp.id.clone();
        self.experiences.insert(id, exp);
        if self.experiences.len() > MAX_SWARM_MEMORY { self.prune(); }
        true
    }

    pub fn get_by_type(&self, type_key: &str) -> Vec<&Experience> {
        self.type_index.get(type_key)
            .map(|ids| ids.iter().filter_map(|id| self.experiences.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn get_by_region(&self, region: &str) -> Vec<&Experience> {
        self.region_index.get(region)
            .map(|ids| ids.iter().filter_map(|id| self.experiences.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn censorship_bypasses_for(&self, region: &str) -> Vec<&Experience> {
        self.get_by_type("censorship_bypass").into_iter().filter(|exp| {
            if let ExperienceType::CensorshipBypass { region: ref r, .. } = exp.experience_type {
                r.contains(region)
            } else { false }
        }).collect()
    }

    fn prune(&mut self) {
        let mut scores: Vec<(String, f64)> = self.experiences.iter()
            .map(|(id, exp)| (id.clone(), exp.utility_score)).collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let to_remove = self.experiences.len() - MAX_SWARM_MEMORY / 2;
        for (id, _) in scores.iter().take(to_remove) { self.experiences.remove(id); }
    }

    pub fn stats(&self) -> SwarmStats {
        SwarmStats {
            total_experiences: self.experiences.len(),
            verified_experiences: self.experiences.values().filter(|e| e.is_verified()).count(),
            blacklisted_nodes: self.blacklist.len(),
            regions_covered: self.region_index.len(),
            total_received: self.total_received,
            censorship_bypasses: self.get_by_type("censorship_bypass").len(),
            attack_patterns: self.get_by_type("attack_pattern").len(),
            optimal_routes: self.get_by_type("optimal_route").len(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwarmStats {
    pub total_experiences: usize,
    pub verified_experiences: usize,
    pub blacklisted_nodes: usize,
    pub regions_covered: usize,
    pub total_received: u64,
    pub censorship_bypasses: usize,
    pub attack_patterns: usize,
    pub optimal_routes: usize,
}

impl std::fmt::Display for SwarmStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  SWARM INTELLIGENCE — COLLECTIVE MEMORY      ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Всего опытов:    {:>6} (верифиц: {:>6})  ║\n\
             ║  Регионов:        {:>6}                      ║\n\
             ║  Обходы цензуры:  {:>6}                      ║\n\
             ║  Паттерны атак:   {:>6}                      ║\n\
             ║  Маршруты:        {:>6}                      ║\n\
             ║  Чёрный список:   {:>6} узлов               ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_experiences, self.verified_experiences,
            self.regions_covered, self.censorship_bypasses,
            self.attack_patterns, self.optimal_routes,
            self.blacklisted_nodes,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub experience: Experience,
    pub forwarded_by: String,
    pub visited: Vec<String>,
}

pub struct GossipProtocol {
    pub node_id: String,
    pub region: String,
    pending_gossip: Vec<GossipMessage>,
    seen_ids: std::collections::HashSet<String>,
    pub messages_forwarded: u64,
    pub messages_received: u64,
}

impl GossipProtocol {
    pub fn new(node_id: &str, region: &str) -> Self {
        GossipProtocol {
            node_id: node_id.to_string(), region: region.to_string(),
            pending_gossip: vec![],
            seen_ids: std::collections::HashSet::new(),
            messages_forwarded: 0, messages_received: 0,
        }
    }

    pub fn receive(&mut self, mut msg: GossipMessage, memory: &mut SwarmMemory) -> bool {
        self.messages_received += 1;
        if self.seen_ids.contains(&msg.experience.id) { return false; }
        if msg.experience.ttl == 0 { return false; }
        msg.experience.confirm(&self.node_id);
        let is_new = memory.absorb(msg.experience.clone());
        self.seen_ids.insert(msg.experience.id.clone());
        if is_new && msg.experience.ttl > 0 {
            let mut forward = msg.clone();
            forward.forwarded_by = self.node_id.clone();
            forward.visited.push(self.node_id.clone());
            forward.experience.propagate();
            self.pending_gossip.push(forward);
            return true;
        }
        false
    }

    pub fn originate(&mut self, exp_type: ExperienceType, memory: &mut SwarmMemory) -> GossipMessage {
        let exp = Experience::new(&self.node_id, &self.region, exp_type);
        let msg = GossipMessage {
            experience: exp.clone(),
            forwarded_by: self.node_id.clone(),
            visited: vec![self.node_id.clone()],
        };
        memory.absorb(exp);
        self.seen_ids.insert(msg.experience.id.clone());
        msg
    }

    pub fn drain_pending(&mut self) -> Vec<GossipMessage> {
        self.messages_forwarded += GOSSIP_FAN_OUT as u64;
        self.pending_gossip.drain(..).collect()
    }
}

pub struct SwarmLearner {
    pub memory: SwarmMemory,
    pub gossip: GossipProtocol,
    route_bonuses: HashMap<String, f64>,
}

impl SwarmLearner {
    pub fn new(node_id: &str, region: &str) -> Self {
        SwarmLearner {
            memory: SwarmMemory::new(),
            gossip: GossipProtocol::new(node_id, region),
            route_bonuses: HashMap::new(),
        }
    }

    pub fn learn_and_apply(&mut self, msg: GossipMessage) {
        let is_new = self.gossip.receive(msg.clone(), &mut self.memory);
        if is_new { self.apply_experience(&msg.experience); }
    }

    fn apply_experience(&mut self, exp: &Experience) {
        if let ExperienceType::OptimalRoute { via_nodes, reliability, avg_latency_ms, .. } = &exp.experience_type {
            if let Some(first) = via_nodes.first() {
                let bonus = reliability * (1.0 - avg_latency_ms / 1000.0).max(0.0);
                *self.route_bonuses.entry(first.clone()).or_insert(0.0) += bonus * 0.1;
            }
        }
        self.memory.total_applied += 1;
    }

    pub fn get_route_bonus(&self, node_id: &str) -> f64 {
        *self.route_bonuses.get(node_id).unwrap_or(&0.0)
    }

    pub fn is_blacklisted(&self, node_id: &str) -> bool {
        self.memory.blacklist.contains(&node_id.to_string())
    }

    pub fn simulate_network_gossip(&mut self, experiences: Vec<(ExperienceType, &str, &str)>) {
        for (exp_type, origin_node, origin_region) in experiences {
            let exp = Experience::new(origin_node, origin_region, exp_type);
            let msg = GossipMessage {
                experience: exp,
                forwarded_by: origin_node.to_string(),
                visited: vec![origin_node.to_string()],
            };
            self.learn_and_apply(msg);
        }
    }
}
