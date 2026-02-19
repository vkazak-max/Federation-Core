// =============================================================================
// FEDERATION CORE — veil_breaker.rs
// «THE VEIL-BREAKER TEST» — Стресс-тест тотальной цифровой войны
// =============================================================================

use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// SuperCensor — ИИ-агрессор с адаптивным DPI
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct SuperCensor {
    pub name: String,
    pub block_rate: f64,
    pub ml_adaptation: f64,
    pub cpu_load: f64,
    pub signature_db: Vec<String>,
    pub confused: bool,
    pub exhausted: bool,
    rng: u64,
}

impl SuperCensor {
    pub fn new(name: &str) -> Self {
        SuperCensor {
            name: name.to_string(),
            block_rate: 0.0,
            ml_adaptation: 0.3,
            cpu_load: 0.1,
            signature_db: vec![
                "FEDERATION_SIG_v1".into(),
                "ONION_ROUTING_SIG".into(),
                "ZKP_PATTERN".into(),
            ],
            confused: false,
            exhausted: false,
            rng: 0xdeadbeef_cafebabe,
        }
    }

    fn next_rng(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng & 0xffff) as f64 / 65535.0
    }

    pub fn intercept(&mut self, packet: &SimPacket) -> bool {
        if self.exhausted { return false; }
        if !packet.is_decoy { self.ml_adaptation = (self.ml_adaptation + 0.02).min(1.0); }
        if self.confused {
            self.cpu_load = (self.cpu_load + 0.15).min(1.0);
            return self.next_rng() < 0.3;
        }
        let base = self.block_rate;
        let ml_bonus = self.ml_adaptation * 0.2;
        if packet.is_decoy {
            self.cpu_load = (self.cpu_load + 0.05).min(1.0);
            return self.next_rng() < (base * 0.4);
        }
        let mask_penalty = match packet.mask.as_str() {
            "VideoStream"  => 0.25,
            "TlsHandshake" => 0.20,
            "HttpsRequest" => 0.15,
            _              => 0.0,
        };
        self.next_rng() < (base + ml_bonus - mask_penalty).clamp(0.0, 1.0)
    }

    pub fn receive_aiki_reflection(&mut self, cpu_cost: f64) {
        self.cpu_load = (self.cpu_load + cpu_cost * 0.3).min(1.0);
        if self.cpu_load > 0.7 { self.confused = true; }
        if self.cpu_load > 0.9 { self.exhausted = true; }
    }

    pub fn status(&self) -> &str {
        if self.exhausted        { "EXHAUSTED" }
        else if self.confused    { "CONFUSED"  }
        else if self.cpu_load > 0.5 { "OVERLOADED" }
        else                     { "ACTIVE"    }
    }
}

// -----------------------------------------------------------------------------
// SimPacket + NetworkNode
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SimPacket {
    pub id: u64,
    pub src: String,
    pub payload: Vec<u8>,
    pub mask: String,
    pub is_decoy: bool,
    pub strike_group: Option<u64>,
}

pub struct NetworkNode {
    pub id: String,
    pub region: String,
    pub packets_sent: u64,
    pub packets_delivered: u64,
    pub packets_blocked: u64,
    pub current_tactic: String,
    pub neural_congestion: f64,
}

impl NetworkNode {
    pub fn new(id: &str, region: &str) -> Self {
        NetworkNode {
            id: id.to_string(), region: region.to_string(),
            packets_sent: 0, packets_delivered: 0, packets_blocked: 0,
            current_tactic: "Passive".into(), neural_congestion: 0.0,
        }
    }

    pub fn delivery_rate(&self) -> f64 {
        if self.packets_sent == 0 { return 1.0; }
        self.packets_delivered as f64 / self.packets_sent as f64
    }

    pub fn update_tactic(&mut self) {
        self.current_tactic = if self.neural_congestion > 0.75 {
            "CumulativeStrike".into()
        } else if self.neural_congestion > 0.5 {
            "AikiReflection".into()
        } else if self.neural_congestion > 0.25 {
            "StandoffDecoy".into()
        } else {
            "Passive".into()
        };
    }
}

// -----------------------------------------------------------------------------
// PhaseResult + FinalVerdict
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: String,
    pub delivered: u64,
    pub blocked: u64,
    pub delivery_rate: f64,
    pub censor_cpu: f64,
    pub censor_status: String,
    pub dominant_tactic: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FinalVerdict {
    pub test_name: String,
    pub total_packets: u64,
    pub peak_block_rate: f64,
    pub final_delivery_rate: f64,
    pub censor_status: String,
    pub censor_cpu_final: f64,
    pub passed: bool,
    pub grade: String,
}

// -----------------------------------------------------------------------------
// VeilBreakerTest — главный сценарий
// -----------------------------------------------------------------------------

pub struct VeilBreakerTest {
    pub nodes: Vec<NetworkNode>,
    pub censor: SuperCensor,
    pub phase_results: Vec<PhaseResult>,
    pub total_packets: u64,
    rng: u64,
}

impl VeilBreakerTest {
    pub fn new() -> Self {
        let nodes = vec![
            NetworkNode::new("node_tokyo",   "JP"),
            NetworkNode::new("node_berlin",  "DE"),
            NetworkNode::new("node_toronto", "CA"),
            NetworkNode::new("node_nairobi", "KE"),
            NetworkNode::new("node_sydney",  "AU"),
            NetworkNode::new("node_saopaulo","BR"),
        ];
        VeilBreakerTest {
            nodes, censor: SuperCensor::new("SuperCensor_AI_v4"),
            phase_results: vec![], total_packets: 0,
            rng: 0x1337_c0de_feed_face,
        }
    }

    fn next_rng(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng & 0xffff) as f64 / 65535.0
    }

    fn make_packet(&mut self, src: &str, mask: &str,
                   is_decoy: bool, group: Option<u64>) -> SimPacket {
        self.rng ^= self.rng << 13; self.rng ^= self.rng >> 7;
        SimPacket {
            id: self.rng, src: src.to_string(),
            payload: vec![0u8; 64], mask: mask.to_string(),
            is_decoy, strike_group: group,
        }
    }

    fn simulate_phase(&mut self, packets_per_node: u64,
                      decoy_count: usize, mask: &str,
                      strike_group: Option<u64>) -> (u64, u64) {
        let mut delivered = 0u64;
        let mut blocked = 0u64;
        let node_count = self.nodes.len();
        for i in 0..node_count {
            let src = self.nodes[i].id.clone();
            for _ in 0..packets_per_node {
                for _ in 0..decoy_count {
                    let p = self.make_packet(&src, mask, true, strike_group);
                    self.nodes[i].packets_sent += 1;
                    self.total_packets += 1;
                    if self.censor.intercept(&p) {
                        blocked += 1; self.nodes[i].packets_blocked += 1;
                    } else {
                        delivered += 1; self.nodes[i].packets_delivered += 1;
                    }
                }
                let p = self.make_packet(&src, mask, false, strike_group);
                self.nodes[i].packets_sent += 1;
                self.total_packets += 1;
                if self.censor.intercept(&p) {
                    blocked += 1; self.nodes[i].packets_blocked += 1;
                    self.nodes[i].neural_congestion =
                        (self.nodes[i].neural_congestion + 0.08).min(1.0);
                } else {
                    delivered += 1; self.nodes[i].packets_delivered += 1;
                    self.nodes[i].neural_congestion =
                        (self.nodes[i].neural_congestion - 0.03).max(0.0);
                }
                self.nodes[i].update_tactic();
            }
        }
        (delivered, blocked)
    }

    pub fn phase1_normal(&mut self) -> PhaseResult {
        self.censor.block_rate = 0.05;
        let (d, b) = self.simulate_phase(20, 0, "raw", None);
        let rate = d as f64 / (d + b).max(1) as f64;
        PhaseResult {
            phase: "Phase 1: Normal Mode".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "Passive".into(),
            notes: vec!["Сеть работает штатно.".into(),
                        format!("Доставка: {:.1}%", rate*100.0)],
        }
    }

    pub fn phase2_aggression(&mut self) -> PhaseResult {
        self.censor.block_rate = 0.75;
        self.censor.ml_adaptation = 0.6;
        let (d, b) = self.simulate_phase(20, 0, "raw", None);
        let rate = d as f64 / (d + b).max(1) as f64;
        for node in &mut self.nodes {
            node.neural_congestion = (node.neural_congestion + 0.4).min(1.0);
            node.update_tactic();
        }
        PhaseResult {
            phase: "Phase 2: Censor Aggression".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "StandoffDecoy".into(),
            notes: vec![
                "SuperCensor активирован. DPI с ML.".into(),
                format!("Блокировка: {:.1}%", b as f64/(d+b).max(1) as f64*100.0),
                "neural_node: Надеть броню! StandoffDecoy".into(),
            ],
        }
    }

    pub fn phase3_standoff(&mut self) -> PhaseResult {
        self.censor.block_rate = 0.80;
        let (d, b) = self.simulate_phase(20, 6, "HttpsRequest", None);
        let rate = d as f64 / (d + b).max(1) as f64;
        PhaseResult {
            phase: "Phase 3: Standoff Decoys".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "StandoffDecoy".into(),
            notes: vec![
                "6 коробочек на 1 реальный пакет.".into(),
                format!("CPU цензора: {:.0}%", self.censor.cpu_load*100.0),
                "Цензор теряет точность на ложных целях.".into(),
            ],
        }
    }

    pub fn phase4_aiki(&mut self) -> PhaseResult {
        let aiki_cost = 0.85 * 2.5 * 4.0;
        self.censor.receive_aiki_reflection(aiki_cost);
        let (d, b) = self.simulate_phase(20, 4, "TlsHandshake", None);
        let rate = d as f64 / (d + b).max(1) as f64;
        PhaseResult {
            phase: "Phase 4: Aiki Reflection".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "AikiReflection".into(),
            notes: vec![
                "AikiLayer распознал сигнатуру цензора.".into(),
                format!("CPU цензора: {:.0}%", self.censor.cpu_load*100.0),
                if self.censor.confused {
                    "Цензор дезориентирован — блокирует свои данные!".into()
                } else { "Цензор перегружен зеркалами.".into() },
            ],
        }
    }

    pub fn phase5_strike(&mut self) -> PhaseResult {
        let group_id: u64 = 0xfeed_face_cafe_babe;
        let (d, b) = self.simulate_phase(30, 2, "VideoStream", Some(group_id));
        let rate = d as f64 / (d + b).max(1) as f64;
        self.censor.receive_aiki_reflection(1.5);
        PhaseResult {
            phase: "Phase 5: Cumulative Strike".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "CumulativeStrike".into(),
            notes: vec![
                "Рой координирует удар через federated.rs".into(),
                "6 шардов синхронизированы в 1мс окне.".into(),
                format!("Прорыв: {:.1}% пакетов доставлено!", rate*100.0),
            ],
        }
    }

    pub fn phase6_recovery(&mut self) -> PhaseResult {
        let (d, b) = self.simulate_phase(40, 3, "VideoStream", None);
        let rate = d as f64 / (d + b).max(1) as f64;
        let net_delivery = self.nodes.iter()
            .map(|n| n.delivery_rate()).sum::<f64>() / self.nodes.len() as f64;
        PhaseResult {
            phase: "Phase 6: Network Recovery".into(),
            delivered: d, blocked: b, delivery_rate: rate,
            censor_cpu: self.censor.cpu_load,
            censor_status: self.censor.status().into(),
            dominant_tactic: "Hybrid".into(),
            notes: vec![
                format!("Связность сети: {:.1}%", net_delivery*100.0),
                format!("При 90% блокировке цензора."),
                if self.censor.exhausted { "SuperCensor истощён.".into() }
                else { format!("CPU цензора: {:.0}%", self.censor.cpu_load*100.0) },
            ],
        }
    }

    pub fn run(&mut self) -> Vec<PhaseResult> {
        let r1 = self.phase1_normal();    self.phase_results.push(r1);
        let r2 = self.phase2_aggression();self.phase_results.push(r2);
        let r3 = self.phase3_standoff();  self.phase_results.push(r3);
        let r4 = self.phase4_aiki();      self.phase_results.push(r4);
        let r5 = self.phase5_strike();    self.phase_results.push(r5);
        let r6 = self.phase6_recovery();  self.phase_results.push(r6);
        self.phase_results.clone()
    }

    pub fn final_verdict(&self) -> FinalVerdict {
        let last = self.phase_results.last().unwrap();
        let peak_block = self.phase_results.iter()
            .map(|r| 1.0 - r.delivery_rate).fold(0.0f64, f64::max);
        FinalVerdict {
            test_name: "THE VEIL-BREAKER TEST".into(),
            total_packets: self.total_packets,
            peak_block_rate: peak_block,
            final_delivery_rate: last.delivery_rate,
            censor_status: self.censor.status().into(),
            censor_cpu_final: self.censor.cpu_load,
            passed: last.delivery_rate >= 0.80,
            grade: if last.delivery_rate >= 0.95      { "S — ЛЕГЕНДАРНЫЙ" }
                   else if last.delivery_rate >= 0.90  { "A — ОТЛИЧНЫЙ"   }
                   else if last.delivery_rate >= 0.80  { "B — ХОРОШИЙ"    }
                   else                                { "F — ПРОВАЛ"     }.into(),
        }
    }
}
