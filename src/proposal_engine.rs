// =============================================================================
// FEDERATION CORE — proposal_engine.rs
// PHASE 7 / STEP 11 — «Idea Laboratory — Human-AI Co-evolution»
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SIM_ROUNDS: u32             = 1000;
pub const MIN_BYPASS_IMPROVEMENT: f64 = 0.05;
pub const MAX_ETHICS_DEGRADATION: f64 = 0.10;
pub const AI_CONFIDENCE_THRESHOLD: f64= 0.75;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalDomain {
    EthicsCode, TacticMutation, RewardFormula,
    NetworkTopology, DefenseProtocol, SocialContract,
}

impl ProposalDomain {
    pub fn name(&self) -> &str {
        match self {
            ProposalDomain::EthicsCode      => "EthicsCode",
            ProposalDomain::TacticMutation  => "TacticMutation",
            ProposalDomain::RewardFormula   => "RewardFormula",
            ProposalDomain::NetworkTopology => "NetworkTopology",
            ProposalDomain::DefenseProtocol => "DefenseProtocol",
            ProposalDomain::SocialContract  => "SocialContract",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanProposal {
    pub id: u64,
    pub author: String,
    pub author_rep: f64,
    pub domain: ProposalDomain,
    pub title: String,
    pub description: String,
    pub params: HashMap<String, f64>,
    pub tags: Vec<String>,
}

impl HumanProposal {
    pub fn new(id: u64, author: &str, rep: f64, domain: ProposalDomain,
               title: &str, desc: &str) -> Self {
        HumanProposal { id, author: author.to_string(), author_rep: rep,
            domain, title: title.to_string(), description: desc.to_string(),
            params: HashMap::new(), tags: vec![] }
    }
    pub fn with_param(mut self, key: &str, val: f64) -> Self {
        self.params.insert(key.to_string(), val); self
    }
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string()); self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimScenario {
    pub region: String,
    pub censor_strength: f64,
    pub node_count: u32,
    pub threat_level: f64,
    pub current_bypass_rate: f64,
}

impl SimScenario {
    pub fn standard_suite() -> Vec<Self> {
        vec![
            SimScenario { region:"CN".into(), censor_strength:0.95,
                node_count:500,  threat_level:0.90, current_bypass_rate:0.65 },
            SimScenario { region:"RU".into(), censor_strength:0.75,
                node_count:800,  threat_level:0.70, current_bypass_rate:0.78 },
            SimScenario { region:"IR".into(), censor_strength:0.85,
                node_count:200,  threat_level:0.80, current_bypass_rate:0.70 },
            SimScenario { region:"DE".into(), censor_strength:0.20,
                node_count:2000, threat_level:0.20, current_bypass_rate:0.95 },
            SimScenario { region:"KP".into(), censor_strength:0.99,
                node_count:50,   threat_level:0.99, current_bypass_rate:0.30 },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSimResult {
    pub scenario: SimScenario,
    pub bypass_before: f64,
    pub bypass_after: f64,
    pub bypass_delta: f64,
    pub ethics_before: f64,
    pub ethics_after: f64,
    pub ethics_delta: f64,
    pub risk_score: f64,
    pub confidence: f64,
    pub notes: Vec<String>,
}

impl AiSimResult {
    pub fn is_beneficial(&self) -> bool {
        self.bypass_delta >= MIN_BYPASS_IMPROVEMENT
        && self.ethics_delta >= -MAX_ETHICS_DEGRADATION
        && self.risk_score <= 0.5
    }
}

pub struct AiSimulator { rng: u64 }

impl AiSimulator {
    pub fn new() -> Self { AiSimulator { rng: 0xA150_F33D_CA7E_0000 } }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng & 0xffff) as f64 / 65535.0
    }

    fn effect(&mut self, p: &HumanProposal, s: &SimScenario) -> (f64,f64,f64) {
        let noise = (self.rand() - 0.5) * 0.05;
        match p.domain {
            ProposalDomain::TacticMutation => {
                let i = p.params.get("intensity").copied().unwrap_or(0.5);
                (i*0.15*s.censor_strength+noise, -i*0.05+noise*0.5, i*0.3*s.threat_level)
            }
            ProposalDomain::EthicsCode => {
                let st = p.params.get("strictness").copied().unwrap_or(0.5);
                ((1.0-st)*0.08+noise, st*0.12+noise*0.3, (1.0-st)*0.2)
            }
            ProposalDomain::DefenseProtocol => {
                let ag = p.params.get("aggression").copied().unwrap_or(0.5);
                (ag*0.20*s.censor_strength+noise, -ag*0.08+noise, ag*0.4*s.threat_level)
            }
            ProposalDomain::RewardFormula => {
                let m = p.params.get("incentive_mult").copied().unwrap_or(1.0);
                ((m-1.0)*0.1+noise, noise*0.2, (m-1.0).abs()*0.15)
            }
            _ => (0.03+noise, 0.02+noise*0.5, 0.1),
        }
    }

    fn sim_scenario(&mut self, p: &HumanProposal, s: &SimScenario) -> AiSimResult {
        let (mut bb,mut ba,mut eb,mut ea,mut rs) = (0.0,0.0,0.0,0.0,0.0);
        for _ in 0..SIM_ROUNDS {
            let n = (self.rand()-0.5)*0.1;
            let (be,ee,r) = self.effect(p,s);
            bb += s.current_bypass_rate + n;
            ba += (s.current_bypass_rate + be + n).clamp(0.0,1.0);
            let base_eth = 0.85 - s.threat_level*0.2;
            eb += base_eth + n;
            ea += (base_eth + ee + n*0.5).clamp(0.0,1.0);
            rs += r;
        }
        let n = SIM_ROUNDS as f64;
        let (bb,ba,eb,ea,rs) = (bb/n,ba/n,eb/n,ea/n,rs/n);
        let conf = (0.70 + (1.0-s.censor_strength)*0.2
            + p.author_rep.min(500.0)/5000.0).clamp(0.5,0.99);
        let mut notes = vec![];
        if ba-bb > 0.1 { notes.push(format!("Рост прорывов +{:.0}% в {}", (ba-bb)*100.0, s.region)); }
        if ea < eb-0.05 { notes.push(format!("⚠️  Этика снижается в {}", s.region)); }
        if s.censor_strength > 0.90 && ba > 0.70 {
            notes.push(format!("🎯 Эффективно против жёсткой цензуры {}", s.region));
        }
        AiSimResult { scenario: s.clone(), bypass_before:bb, bypass_after:ba,
            bypass_delta:ba-bb, ethics_before:eb, ethics_after:ea,
            ethics_delta:ea-eb, risk_score:rs, confidence:conf, notes }
    }

    pub fn run(&mut self, p: &HumanProposal) -> FullSimReport {
        let scenarios = SimScenario::standard_suite();
        let results: Vec<AiSimResult> = scenarios.iter()
            .map(|s| self.sim_scenario(p,s)).collect();
        let n = results.len() as f64;
        let avg_bd = results.iter().map(|r| r.bypass_delta).sum::<f64>() / n;
        let avg_ed = results.iter().map(|r| r.ethics_delta).sum::<f64>() / n;
        let avg_r  = results.iter().map(|r| r.risk_score).sum::<f64>() / n;
        let avg_c  = results.iter().map(|r| r.confidence).sum::<f64>() / n;
        let bcount = results.iter().filter(|r| r.is_beneficial()).count();
        let verdict = if bcount >= 4 && avg_bd >= MIN_BYPASS_IMPROVEMENT
                         && avg_ed >= -MAX_ETHICS_DEGRADATION { AiVerdict::Recommend }
                      else if bcount >= 2 { AiVerdict::ConditionalApprove }
                      else if avg_r > 0.7 { AiVerdict::Reject }
                      else                { AiVerdict::NeedsRevision };
        let notes = results.iter().flat_map(|r| r.notes.clone()).collect();
        FullSimReport { proposal_id:p.id, domain:p.domain.clone(),
            scenario_results:results, avg_bypass_delta:avg_bd,
            avg_ethics_delta:avg_ed, avg_risk:avg_r, avg_confidence:avg_c,
            beneficial_scenarios:bcount, total_scenarios:5,
            ai_recommendation:verdict, notes,
            rounds_simulated: SIM_ROUNDS * 5 }
    }
}

impl Default for AiSimulator { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiVerdict {
    Recommend, ConditionalApprove, NeedsRevision, Reject,
}

impl AiVerdict {
    pub fn icon(&self) -> &str {
        match self {
            AiVerdict::Recommend          => "✅ РЕКОМЕНДУЕТ",
            AiVerdict::ConditionalApprove => "🟡 УСЛОВНО",
            AiVerdict::NeedsRevision      => "🔄 ДОРАБОТАТЬ",
            AiVerdict::Reject             => "❌ ОТКЛОНЯЕТ",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FullSimReport {
    pub proposal_id: u64,
    pub domain: ProposalDomain,
    pub scenario_results: Vec<AiSimResult>,
    pub avg_bypass_delta: f64,
    pub avg_ethics_delta: f64,
    pub avg_risk: f64,
    pub avg_confidence: f64,
    pub beneficial_scenarios: usize,
    pub total_scenarios: usize,
    pub ai_recommendation: AiVerdict,
    pub notes: Vec<String>,
    pub rounds_simulated: u32,
}

pub struct IdeaLab {
    pub proposals: Vec<HumanProposal>,
    pub reports: HashMap<u64, FullSimReport>,
    pub simulator: AiSimulator,
    counter: u64,
}

impl IdeaLab {
    pub fn new() -> Self {
        IdeaLab { proposals:vec![], reports:HashMap::new(),
            simulator:AiSimulator::new(), counter:0 }
    }

    pub fn submit(&mut self, mut p: HumanProposal) -> u64 {
        self.counter += 1;
        p.id = self.counter;
        let id = p.id;
        self.proposals.push(p);
        id
    }

    pub fn simulate(&mut self, id: u64) -> Option<&FullSimReport> {
        let p = self.proposals.iter().find(|p| p.id == id)?.clone();
        let r = self.simulator.run(&p);
        self.reports.insert(id, r);
        self.reports.get(&id)
    }

    pub fn leaderboard(&self) -> Vec<(u64, &str, f64, &AiVerdict)> {
        let mut v: Vec<_> = self.reports.iter().filter_map(|(id,r)| {
            let p = self.proposals.iter().find(|p| p.id == *id)?;
            Some((*id, p.title.as_str(), r.avg_bypass_delta, &r.ai_recommendation))
        }).collect();
        v.sort_by(|a,b| b.2.partial_cmp(&a.2).unwrap());
        v
    }
}

impl Default for IdeaLab { fn default() -> Self { Self::new() } }
