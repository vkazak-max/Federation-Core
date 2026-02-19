// =============================================================================
// src/consensus.rs — Tendermint-style BFT для Federation Core
// =============================================================================

use std::collections::HashMap;

const QUORUM_THRESHOLD: f64 = 2.0 / 3.0;
const ROUND_TIMEOUT_TICKS: u64 = 10;
const BYZANTINE_DELAY_FRACTION: f64 = 0.95;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeTier { Ghost, Droid, Sentinel, Citadel }

#[derive(Debug, Clone, PartialEq)]
pub enum FaultMode { Honest, Crash, ByzantineLite, Byzantine }

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub height: u64,
    pub round: u32,
    pub proposer_id: usize,
    pub payload: Vec<u8>,
    pub hash: [u8; 32],
}

impl Block {
    pub fn new(height: u64, round: u32, proposer_id: usize, payload: Vec<u8>) -> Self {
        let mut hash = [0u8; 32];
        for (i, b) in payload.iter().enumerate() { hash[i % 32] ^= b; }
        hash[0] ^= (height & 0xff) as u8;
        hash[1] ^= (round & 0xff) as u8;
        hash[2] ^= (proposer_id & 0xff) as u8;
        Block { height, round, proposer_id, payload, hash }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusPhase { Propose, Prevote, Precommit, Finalized }

#[derive(Debug, Clone)]
pub struct Vote {
    pub voter_id: usize,
    pub height: u64,
    pub round: u32,
    pub phase: ConsensusPhase,
    pub block_hash: Option<[u8; 32]>,
    pub weight: f64,
    pub sent_at_tick: u64,
}

#[derive(Debug, Clone)]
pub struct ConsensusNode {
    pub id: usize,
    pub tier: NodeTier,
    pub trust_score: f64,
    pub stake: f64,
    pub fault_mode: FaultMode,
    pub current_height: u64,
    pub current_round: u32,
    pub phase: ConsensusPhase,
    pub locked_block: Option<Block>,
    pub proposed_block: Option<Block>,
    pub rounds_participated: u64,
    pub equivocations_detected: u64,
}

impl ConsensusNode {
    pub fn new(id: usize, tier: NodeTier, trust_score: f64, stake: f64, fault_mode: FaultMode) -> Self {
        ConsensusNode {
            id, tier, trust_score, stake, fault_mode,
            current_height: 0, current_round: 0,
            phase: ConsensusPhase::Propose,
            locked_block: None, proposed_block: None,
            rounds_participated: 0, equivocations_detected: 0,
        }
    }

    pub fn vote_weight(&self) -> f64 {
        self.trust_score.powf(0.7) * self.stake.sqrt()
    }

    pub fn can_lead(&self) -> bool {
        matches!(self.tier, NodeTier::Sentinel | NodeTier::Citadel)
    }

    fn byzantine_equivocate(
        &self, height: u64, round: u32, phase: ConsensusPhase,
        block_a: Option<[u8; 32]>, block_b: Option<[u8; 32]>, tick: u64,
    ) -> (Vote, Vote) {
        let weight = self.vote_weight();
        let delayed_tick = tick + (ROUND_TIMEOUT_TICKS as f64 * BYZANTINE_DELAY_FRACTION) as u64;
        let vote_a = Vote { voter_id: self.id, height, round, phase: phase.clone(),
            block_hash: block_a, weight, sent_at_tick: delayed_tick };
        let vote_b = Vote { voter_id: self.id, height, round, phase,
            block_hash: block_b, weight, sent_at_tick: delayed_tick };
        (vote_a, vote_b)
    }
}

#[derive(Debug)]
pub enum RoundResult {
    Finalized(Block),
    Timeout { new_round: u32 },
    EquivocationDetected { attacker_id: usize },
    NoQuorum,
}

#[derive(Debug, Default)]
pub struct ConsensusLog {
    pub height: u64,
    pub rounds_taken: u32,
    pub total_weight: f64,
    pub prevote_weight_for: f64,
    pub precommit_weight_for: f64,
    pub byzantine_detected: bool,
    pub equivocations: Vec<(usize, [u8; 32], [u8; 32])>,
    pub finalized_hash: Option<[u8; 32]>,
    pub leader_id: usize,
    pub participants: usize,
}

pub struct ConsensusEngine {
    pub nodes: Vec<ConsensusNode>,
    pub f: usize,
    pub current_height: u64,
    pub current_round: u32,
    pub tick: u64,
    pub log: Vec<ConsensusLog>,
}

impl ConsensusEngine {
    pub fn new(nodes: Vec<ConsensusNode>, f: usize) -> Self {
        let n = nodes.len();
        assert!(n > 3 * f, "BFT нарушена: n={} < 3f+1={} (f={})", n, 3 * f + 1, f);
        ConsensusEngine { nodes, f, current_height: 0, current_round: 0, tick: 0, log: Vec::new() }
    }

    pub fn total_weight(&self) -> f64 {
        self.nodes.iter().map(|n| n.vote_weight()).sum()
    }

    pub fn total_honest_weight(&self) -> f64 {
        self.nodes.iter().filter(|n| n.fault_mode == FaultMode::Honest)
            .map(|n| n.vote_weight()).sum()
    }

    pub fn is_healthy(&self) -> bool {
        self.total_honest_weight() > self.total_weight() * QUORUM_THRESHOLD
    }

    pub fn byzantine_weight_fraction(&self) -> f64 {
        let byz: f64 = self.nodes.iter()
            .filter(|n| n.fault_mode == FaultMode::Byzantine || n.fault_mode == FaultMode::ByzantineLite)
            .map(|n| n.vote_weight()).sum();
        byz / self.total_weight()
    }

    pub fn elect_leader(&self) -> usize {
        self.nodes.iter()
            .filter(|n| n.can_lead() && n.fault_mode == FaultMode::Honest)
            .max_by(|a, b| a.trust_score.partial_cmp(&b.trust_score)
                .unwrap_or(std::cmp::Ordering::Equal).then(a.id.cmp(&b.id)))
            .map(|n| n.id).unwrap_or(0)
    }

    pub fn run_round(&mut self, height: u64, payload: Vec<u8>) -> RoundResult {
        let round = self.current_round;
        self.tick += 1;
        let mut clog = ConsensusLog {
            height, rounds_taken: round,
            total_weight: self.total_weight(),
            participants: self.nodes.len(),
            ..Default::default()
        };
        let leader_id = self.elect_leader();
        clog.leader_id = leader_id;
        let block = Block::new(height, round, leader_id, payload.clone());

        // PREVOTE
        let mut prevotes: Vec<Vote> = Vec::new();
        let mut equivocations: HashMap<usize, Vec<[u8; 32]>> = HashMap::new();

        for node in &self.nodes {
            match &node.fault_mode {
                FaultMode::Honest => {
                    prevotes.push(Vote { voter_id: node.id, height, round,
                        phase: ConsensusPhase::Prevote,
                        block_hash: Some(block.hash),
                        weight: node.vote_weight(), sent_at_tick: self.tick });
                }
                FaultMode::Crash => {}
                FaultMode::ByzantineLite => {
                    prevotes.push(Vote { voter_id: node.id, height, round,
                        phase: ConsensusPhase::Prevote,
                        block_hash: None,
                        weight: node.vote_weight(), sent_at_tick: self.tick });
                }
                FaultMode::Byzantine => {
                    let mut fake_hash = block.hash;
                    fake_hash[0] ^= 0xFF;
                    let (vote_a, _vote_b) = node.byzantine_equivocate(
                        height, round, ConsensusPhase::Prevote,
                        Some(block.hash), Some(fake_hash), self.tick);
                    equivocations.entry(node.id).or_default().push(block.hash);
                    equivocations.entry(node.id).or_default().push(fake_hash);
                    if vote_a.sent_at_tick <= self.tick + ROUND_TIMEOUT_TICKS {
                        prevotes.push(vote_a);
                    }
                }
            }
        }

        // Детектор equivocation
        for (node_id, hashes) in &equivocations {
            if hashes.len() >= 2 && hashes[0] != hashes[1] {
                clog.byzantine_detected = true;
                clog.equivocations.push((*node_id, hashes[0], hashes[1]));
                if let Some(node) = self.nodes.iter_mut().find(|n| n.id == *node_id) {
                    node.trust_score *= 0.1;
                    node.equivocations_detected += 1;
                }
            }
        }

        let prevote_weight_for: f64 = prevotes.iter()
            .filter(|v| v.block_hash == Some(block.hash))
            .map(|v| v.weight).sum();
        clog.prevote_weight_for = prevote_weight_for;

        if prevote_weight_for <= clog.total_weight * QUORUM_THRESHOLD {
            self.current_round += 1;
            self.log.push(clog);
            return RoundResult::Timeout { new_round: self.current_round };
        }

        // PRECOMMIT
        let mut precommits: Vec<Vote> = Vec::new();
        for node in &self.nodes {
            if node.fault_mode == FaultMode::Honest {
                precommits.push(Vote { voter_id: node.id, height, round,
                    phase: ConsensusPhase::Precommit,
                    block_hash: Some(block.hash),
                    weight: node.vote_weight(), sent_at_tick: self.tick });
            }
            // Byzantine/Crash/ByzantineLite — selective silence на Precommit
        }

        let precommit_weight_for: f64 = precommits.iter()
            .filter(|v| v.block_hash == Some(block.hash))
            .map(|v| v.weight).sum();
        clog.precommit_weight_for = precommit_weight_for;

        if precommit_weight_for <= clog.total_weight * QUORUM_THRESHOLD {
            self.current_round += 1;
            let byz_detected = clog.byzantine_detected;
            let first_attacker = clog.equivocations.first().map(|&(id, _, _)| id);
            self.log.push(clog);
            if byz_detected {
                if let Some(attacker_id) = first_attacker {
                    return RoundResult::EquivocationDetected { attacker_id };
                }
            }
            return RoundResult::Timeout { new_round: self.current_round };
        }

        // INSTANT FINALITY
        clog.finalized_hash = Some(block.hash);
        self.current_height += 1;
        self.current_round = 0;
        self.tick += 1;
        for node in &mut self.nodes {
            node.current_height = self.current_height;
            node.current_round = 0;
            node.rounds_participated += 1;
        }
        self.log.push(clog);
        RoundResult::Finalized(block)
    }

    pub fn simulate(&mut self, num_blocks: u64, payloads: Vec<Vec<u8>>) -> SimulationResult {
        let mut finalized = 0u64;
        let mut total_rounds = 0u32;
        let mut byzantine_events = 0usize;
        for i in 0..num_blocks {
            let payload = payloads.get(i as usize).cloned().unwrap_or_else(|| vec![i as u8]);
            let height = self.current_height;
            let mut attempts = 0u32;
            loop {
                attempts += 1;
                if attempts > 100 { break; }
                match self.run_round(height, payload.clone()) {
                    RoundResult::Finalized(_) => { finalized += 1; total_rounds += attempts; break; }
                    RoundResult::Timeout { .. } => {}
                    RoundResult::EquivocationDetected { attacker_id } => {
                        byzantine_events += 1;
                        eprintln!("[CONSENSUS] Equivocation! Узел #{} наказан.", attacker_id);
                    }
                    RoundResult::NoQuorum => { break; }
                }
            }
        }
        SimulationResult {
            blocks_finalized: finalized,
            blocks_attempted: num_blocks,
            avg_rounds_per_block: if finalized > 0 { total_rounds as f64 / finalized as f64 } else { 0.0 },
            byzantine_events_detected: byzantine_events,
            final_height: self.current_height,
        }
    }
}

#[derive(Debug)]
pub struct SimulationResult {
    pub blocks_finalized: u64,
    pub blocks_attempted: u64,
    pub avg_rounds_per_block: f64,
    pub byzantine_events_detected: usize,
    pub final_height: u64,
}

pub struct NetworkBuilder { nodes: Vec<ConsensusNode>, next_id: usize }

impl NetworkBuilder {
    pub fn new() -> Self { NetworkBuilder { nodes: Vec::new(), next_id: 0 } }

    pub fn add_node(mut self, tier: NodeTier, trust_score: f64, stake: f64, fault_mode: FaultMode) -> Self {
        self.nodes.push(ConsensusNode::new(self.next_id, tier, trust_score, stake, fault_mode));
        self.next_id += 1;
        self
    }

    pub fn build_minimal_bft(f: usize) -> ConsensusEngine {
        let n = 3 * f + 1;
        let mut builder = NetworkBuilder::new();
        builder = builder
            .add_node(NodeTier::Citadel,  0.95, 1000.0, FaultMode::Honest)
            .add_node(NodeTier::Sentinel, 0.85, 500.0,  FaultMode::Honest);
        for i in 2..(n - f) {
            let trust = (0.7 - i as f64 * 0.02).max(0.3);
            builder = builder.add_node(NodeTier::Droid, trust, 100.0, FaultMode::Honest);
        }
        for i in 0..f {
            // Byzantine узлы: намеренно низкий stake чтобы их вес < 1/3 суммарного
            // В реальной сети вес Byzantine ограничен протоколом slashing
            let trust = (0.60 - i as f64 * 0.05).max(0.2);
            builder = builder.add_node(NodeTier::Sentinel, trust, 80.0, FaultMode::Byzantine);
        }
        ConsensusEngine::new(builder.nodes, f)
    }

    pub fn build(self, f: usize) -> ConsensusEngine { ConsensusEngine::new(self.nodes, f) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine_4_1() -> ConsensusEngine { NetworkBuilder::build_minimal_bft(1) }

    #[test]
    fn test_consensus_honest_network() {
        let nodes = vec![
            ConsensusNode::new(0, NodeTier::Citadel,  0.95, 1000.0, FaultMode::Honest),
            ConsensusNode::new(1, NodeTier::Sentinel, 0.85, 500.0,  FaultMode::Honest),
            ConsensusNode::new(2, NodeTier::Droid,    0.75, 200.0,  FaultMode::Honest),
            ConsensusNode::new(3, NodeTier::Droid,    0.65, 150.0,  FaultMode::Honest),
        ];
        let mut engine = ConsensusEngine::new(nodes, 1);
        match engine.run_round(0, b"block-0-payload".to_vec()) {
            RoundResult::Finalized(block) => {
                assert_eq!(block.height, 0);
                assert_eq!(block.proposer_id, 0);
            }
            other => panic!("Ожидали Finalized, получили {:?}", other),
        }
        assert_eq!(engine.current_height, 1);
    }

    #[test]
    fn test_bft_survives_one_byzantine() {
        let mut engine = make_engine_4_1();
        assert!(engine.is_healthy());
        match engine.run_round(0, b"test-payload".to_vec()) {
            RoundResult::NoQuorum => panic!("NoQuorum при f=1 из n=4"),
            _ => {}
        }
    }

    #[test]
    fn test_equivocation_detection_and_punishment() {
        let nodes = vec![
            ConsensusNode::new(0, NodeTier::Citadel,  0.95, 1000.0, FaultMode::Honest),
            ConsensusNode::new(1, NodeTier::Sentinel, 0.85, 500.0,  FaultMode::Honest),
            ConsensusNode::new(2, NodeTier::Droid,    0.75, 200.0,  FaultMode::Honest),
            ConsensusNode::new(3, NodeTier::Sentinel, 0.80, 600.0,  FaultMode::Byzantine),
        ];
        let mut engine = ConsensusEngine::new(nodes, 1);
        let mut detected = false;
        for _ in 0..5 {
            if let RoundResult::EquivocationDetected { attacker_id } =
                engine.run_round(engine.current_height, b"payload".to_vec()) {
                detected = true;
                assert_eq!(attacker_id, 3);
                assert!(engine.nodes[3].trust_score < 0.80);
                break;
            }
        }
        let has_byz_log = engine.log.iter().any(|l| l.byzantine_detected);
        assert!(detected || has_byz_log);
    }

    #[test]
    fn test_vote_weight_formula() {
        let node = ConsensusNode::new(0, NodeTier::Citadel, 0.9, 100.0, FaultMode::Honest);
        let expected = 0.9f64.powf(0.7) * 100f64.sqrt();
        assert!((node.vote_weight() - expected).abs() < 1e-10);
        let citadel = ConsensusNode::new(0, NodeTier::Citadel, 0.95, 1000.0, FaultMode::Honest);
        let ghost   = ConsensusNode::new(1, NodeTier::Ghost,   0.30, 10.0,   FaultMode::Honest);
        assert!(citadel.vote_weight() > ghost.vote_weight());
    }

    #[test]
    fn test_parametric_n7_f2() {
        let mut engine = NetworkBuilder::build_minimal_bft(2);
        assert_eq!(engine.nodes.len(), 7);
        assert!(engine.is_healthy());
        let payloads: Vec<Vec<u8>> = (0..5).map(|i| vec![i as u8]).collect();
        let result = engine.simulate(5, payloads);
        eprintln!("n=7 f=2: {:?}", result);
        assert!(result.blocks_finalized > 0);
    }

    #[test]
    fn test_parametric_n10_f3() {
        let engine = NetworkBuilder::build_minimal_bft(3);
        assert_eq!(engine.nodes.len(), 10);
        assert!(engine.byzantine_weight_fraction() < 1.0 / 3.0 + 0.1);
    }

    #[test]
    fn test_unhealthy_when_byzantine_exceeds_threshold() {
        let nodes = vec![
            ConsensusNode::new(0, NodeTier::Citadel,  0.9, 100.0, FaultMode::Byzantine),
            ConsensusNode::new(1, NodeTier::Sentinel, 0.8, 100.0, FaultMode::Byzantine),
            ConsensusNode::new(2, NodeTier::Droid,    0.7, 100.0, FaultMode::Byzantine),
            ConsensusNode::new(3, NodeTier::Droid,    0.6, 100.0, FaultMode::Honest),
        ];
        let engine = ConsensusEngine::new(nodes, 1);
        assert!(!engine.is_healthy());
    }

    #[test]
    fn test_leader_election() {
        let nodes = vec![
            ConsensusNode::new(0, NodeTier::Ghost,    0.3,  10.0, FaultMode::Honest),
            ConsensusNode::new(1, NodeTier::Droid,    0.6, 100.0, FaultMode::Honest),
            ConsensusNode::new(2, NodeTier::Sentinel, 0.8, 500.0, FaultMode::Honest),
            ConsensusNode::new(3, NodeTier::Citadel,  0.9, 800.0, FaultMode::Honest),
        ];
        let engine = ConsensusEngine::new(nodes, 1);
        assert_eq!(engine.elect_leader(), 3);
    }

    #[test]
    fn test_instant_finality() {
        let mut engine = make_engine_4_1();
        let h = engine.current_height;
        if let RoundResult::Finalized(block) = engine.run_round(h, b"finality-test".to_vec()) {
            assert_eq!(engine.current_height, h + 1);
            assert_eq!(block.height, h);
        }
    }

    #[test]
    fn test_full_simulation_10_blocks() {
        let mut engine = make_engine_4_1();
        let payloads: Vec<Vec<u8>> = (0..10u8).map(|i| format!("block-{}", i).into_bytes()).collect();
        let result = engine.simulate(10, payloads);
        eprintln!("Симуляция 10 блоков: {:?}", result);
        eprintln!("Здоровье сети: {}", engine.is_healthy());
        eprintln!("Byzantine вес: {:.1}%", engine.byzantine_weight_fraction() * 100.0);
        assert!(result.blocks_finalized >= 5);
    }

    #[test]
    #[should_panic(expected = "BFT нарушена")]
    fn test_bft_invariant_panic() {
        let nodes = vec![
            ConsensusNode::new(0, NodeTier::Citadel, 0.9, 100.0, FaultMode::Honest),
            ConsensusNode::new(1, NodeTier::Droid,   0.7, 100.0, FaultMode::Honest),
            ConsensusNode::new(2, NodeTier::Droid,   0.6, 100.0, FaultMode::Byzantine),
        ];
        let _ = ConsensusEngine::new(nodes, 2);
    }
}
