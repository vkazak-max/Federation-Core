use crate::adaptive_censor::{AdaptiveStrategy, PatternDetector, BlockStrategy, BypassTactic, RegionCensor};
use std::collections::HashMap;

pub const WAR2_NODES: usize = 100;
pub const LEARNING_WINDOW: usize = 5;

#[derive(Debug)]
pub struct AdaptiveSuperCensor {
    pub name: String,
    pub strategy: AdaptiveStrategy,
    pub detector: PatternDetector,
    pub region_censors: HashMap<String, RegionCensor>,
    pub total_checks: u64,
    pub successful_blocks: u64,
    pub failed_blocks: u64,
    pub cpu_load: f64,
    pub exhausted: bool,
    pub confused: bool,
    tick: u64,
}

impl AdaptiveSuperCensor {
    pub fn new(name: &str) -> Self {
        AdaptiveSuperCensor {
            name: name.to_string(),
            strategy: AdaptiveStrategy::new(BlockStrategy::DpiFingerprint { effectiveness: 0.70 }),
            detector: PatternDetector::new(),
            region_censors: HashMap::new(),
            total_checks: 0,
            successful_blocks: 0,
            failed_blocks: 0,
            cpu_load: 0.15,
            exhausted: false,
            confused: false,
            tick: 0,
        }
    }
    pub fn check_packet(&mut self, packet: &War2Packet) -> bool {
        self.total_checks += 1;
        self.tick += 1;
        if self.exhausted { self.failed_blocks += 1; return false; }
        if self.confused {
            self.cpu_load = (self.cpu_load + 0.10).min(1.0);
            let blocked = self.tick.is_multiple_of(3);
            if blocked { self.successful_blocks += 1; } else { self.failed_blocks += 1; }
            return blocked;
        }
        let block_chance = match (&self.strategy.current, &packet.tactic) {
            (BlockStrategy::DpiFingerprint { effectiveness }, BypassTactic::Mirage) => effectiveness * 0.9,
            (BlockStrategy::TargetedBlock { .. }, BypassTactic::Onion) => 0.85,
            (BlockStrategy::PhysicalPressure { rate }, BypassTactic::Mesh) => rate * 0.95,
            (BlockStrategy::DpiFingerprint { effectiveness }, _) => effectiveness * 0.6,
            _ => 0.5,
        };
        let mask_penalty = match packet.mask.as_str() { "VideoStream" => 0.30, "TlsHandshake" => 0.25, _ => 0.0 };
        let final_chance = (block_chance - mask_penalty).max(0.0);
        let blocked = (self.tick * 7 + packet.id) % 100 < (final_chance * 100.0) as u64;
        self.cpu_load = (self.cpu_load + 0.01).min(1.0);
        if self.cpu_load > 0.85 { self.confused = true; }
        if self.cpu_load > 0.95 { self.exhausted = true; }
        if blocked { self.successful_blocks += 1; } 
        else { self.failed_blocks += 1; self.detector.observe(packet.tactic.clone(), 0.9); }
        if self.tick.is_multiple_of(LEARNING_WINDOW as u64) {
            let bypass_rate = self.failed_blocks as f64 / self.total_checks.max(1) as f64;
            self.strategy.update_effectiveness(bypass_rate);
            self.strategy.adapt(&self.detector);
        }
        blocked
    }
    pub fn receive_aiki_reflection(&mut self, intensity: f64) {
        self.cpu_load = (self.cpu_load + intensity * 0.4).min(1.0);
        if self.cpu_load > 0.75 { self.confused = true; }
    }
    pub fn block_rate(&self) -> f64 {
        if self.total_checks == 0 { return 0.0; }
        self.successful_blocks as f64 / self.total_checks as f64
    }
    pub fn current_strategy_name(&self) -> String {
        match &self.strategy.current {
            BlockStrategy::DpiFingerprint { .. } => "DpiFingerprint",
            BlockStrategy::PhysicalPressure { .. } => "PhysicalPressure",
            _ => "Other",
        }.to_string()
    }
    pub fn status(&self) -> &str {
        if self.exhausted { "EXHAUSTED" }
        else if self.confused { "CONFUSED" }
        else if self.cpu_load > 0.7 { "OVERLOADED" }
        else { "ACTIVE" }
    }
}
#[derive(Debug, Clone)]
pub struct War2Packet {
    pub id: u64,
    pub node_id: String,
    pub tactic: BypassTactic,
    pub mask: String,
    pub mutation_level: f64,
    pub is_decoy: bool,
}

impl War2Packet {
    pub fn new(id: u64, node_id: &str, tactic: BypassTactic) -> Self {
        War2Packet { id, node_id: node_id.to_string(), tactic, mask: "raw".to_string(), mutation_level: 0.0, is_decoy: false }
    }
    pub fn with_mask(mut self, mask: &str) -> Self { self.mask = mask.to_string(); self }
}

#[derive(Debug, Clone)]
pub struct War2Node {
    pub id: String,
    pub packets_sent: u64,
    pub packets_delivered: u64,
    pub packets_blocked: u64,
    pub current_tactic: BypassTactic,
    pub congestion_level: f64,
}

impl War2Node {
    pub fn new(id: &str) -> Self {
        War2Node { id: id.to_string(), packets_sent: 0, packets_delivered: 0, packets_blocked: 0, 
                   current_tactic: BypassTactic::Mirage, congestion_level: 0.0 }
    }
    pub fn adapt_tactic(&mut self) {
        self.current_tactic = if self.congestion_level > 0.8 { BypassTactic::Strike }
        else if self.congestion_level > 0.6 { BypassTactic::Mesh }
        else { BypassTactic::Mirage };
    }
}
pub struct War2Simulator {
    pub nodes: Vec<War2Node>,
    pub censor: AdaptiveSuperCensor,
    pub tick: usize,
}

impl War2Simulator {
    pub fn new() -> Self {
        let nodes = (0..WAR2_NODES).map(|i| War2Node::new(&format!("node_{}", i))).collect();
        War2Simulator { nodes, censor: AdaptiveSuperCensor::new("War2Censor"), tick: 0 }
    }
    pub fn run_phase(&mut self, name: &str, ticks: usize, packets_per_node: u32) -> War2PhaseResult {
        for _ in 0..ticks {
            self.tick += 1;
            for node_idx in 0..self.nodes.len() {
                for _ in 0..packets_per_node {
                    let packet = War2Packet::new(self.tick as u64 * 1000 + node_idx as u64,
                        &self.nodes[node_idx].id, self.nodes[node_idx].current_tactic.clone()).with_mask("TlsHandshake");
                    self.nodes[node_idx].packets_sent += 1;
                    if self.censor.check_packet(&packet) {
                        self.nodes[node_idx].packets_blocked += 1;
                        self.nodes[node_idx].congestion_level = (self.nodes[node_idx].congestion_level + 0.1).min(1.0);
                    } else {
                        self.nodes[node_idx].packets_delivered += 1;
                        self.nodes[node_idx].congestion_level = (self.nodes[node_idx].congestion_level - 0.02).max(0.0);
                    }
                }
                self.nodes[node_idx].adapt_tactic();
            }
        }
        let total_delivered: u64 = self.nodes.iter().map(|n| n.packets_delivered).sum();
        let total_sent: u64 = self.nodes.iter().map(|n| n.packets_sent).sum();
        War2PhaseResult {
            phase_name: name.to_string(),
            delivery_rate: if total_sent > 0 { total_delivered as f64 / total_sent as f64 } else { 0.0 },
            censor_block_rate: self.censor.block_rate(),
            censor_strategy: self.censor.current_strategy_name(),
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct War2PhaseResult {
    pub phase_name: String,
    pub delivery_rate: f64,
    pub censor_block_rate: f64,
    pub censor_strategy: String,
    pub censor_cpu: f64,
    pub censor_status: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_adaptive_supercensor_basic() {
        let mut censor = AdaptiveSuperCensor::new("test");
        let packet = War2Packet::new(1, "node_1", BypassTactic::Mirage);
        let _ = censor.check_packet(&packet);
        assert_eq!(censor.total_checks, 1);
    }
    #[test]
    fn test_war2_simulation() {
        let mut sim = War2Simulator::new();
        assert_eq!(sim.nodes.len(), WAR2_NODES);
        let result = sim.run_phase("Test", 5, 3);
        assert!(result.delivery_rate >= 0.0 && result.delivery_rate <= 1.0);
    }
    #[test]
    fn test_node_adapts_tactic() {
        let mut node = War2Node::new("test");
        node.congestion_level = 0.9;
        node.adapt_tactic();
        assert_eq!(node.current_tactic, BypassTactic::Strike);
    }
}
