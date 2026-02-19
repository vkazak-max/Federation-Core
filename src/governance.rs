use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_STAKE_TO_PROPOSE: f64 = 1.0;
pub const MIN_STAKE_TO_VOTE: f64 = 0.1;
pub const QUORUM_PERCENT: f64 = 0.51;
pub const VOTING_PERIOD_SECS: u64 = 259200; // 72 часа
pub const FAST_VOTE_PERIOD_SECS: u64 = 3600; // 1 час
pub const MAX_ACTIVE_PROPOSALS: usize = 10;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalType {
    NetworkParam { param: String, old_value: String, new_value: String },
    BanNode { node_id: String, reason: String, evidence: String },
    EmergencySplit { trigger_threshold: f64, reason: String },
    RewardChange { old_base: f64, new_base: f64, reason: String },
    AddSeedNode { address: String, region: String },
    CodeUpgrade { version: String, changelog: String, checksum: String },
    CensorshipResponse { region: String, countermeasure: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Expired,
    Executed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub voter_id: String,
    pub stake_weight: f64,
    pub in_favor: bool,
    pub comment: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub proposer_id: String,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub status: ProposalStatus,
    pub votes: Vec<Vote>,
    pub votes_for: f64,
    pub votes_against: f64,
    pub total_stake_participated: f64,
    pub is_fast_track: bool,
    pub executed: bool,
}

impl Proposal {
    pub fn new(
        proposer_id: &str,
        proposal_type: ProposalType,
        title: &str,
        description: &str,
        is_fast_track: bool,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in format!("{}{}{}", proposer_id, now, title).bytes() {
            h ^= b as u64; h = h.wrapping_mul(0x100000001b3);
        }
        let period = if is_fast_track { FAST_VOTE_PERIOD_SECS } else { VOTING_PERIOD_SECS };
        Proposal {
            id: format!("prop_{:x}", h & 0xffffffff),
            proposer_id: proposer_id.to_string(),
            proposal_type, title: title.to_string(),
            description: description.to_string(),
            created_at: now,
            expires_at: now + period as i64 * 1000,
            status: ProposalStatus::Active,
            votes: vec![], votes_for: 0.0, votes_against: 0.0,
            total_stake_participated: 0.0,
            is_fast_track, executed: false,
        }
    }

    pub fn quorum_reached(&self, total_supply: f64) -> bool {
        if total_supply == 0.0 { return false; }
        self.total_stake_participated / total_supply >= QUORUM_PERCENT
    }

    pub fn is_passing(&self) -> bool {
        self.votes_for > self.votes_against
    }

    pub fn participation_rate(&self, total_supply: f64) -> f64 {
        if total_supply == 0.0 { 0.0 }
        else { self.total_stake_participated / total_supply }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GovernanceDao {
    pub proposals: HashMap<String, Proposal>,
    pub executed_proposals: Vec<String>,
    pub total_votes_cast: u64,
    pub total_stake_voted: f64,
}

impl GovernanceDao {
    pub fn new() -> Self { Self::default() }

    pub fn submit_proposal(
        &mut self,
        proposer_id: &str,
        proposer_balance: f64,
        proposal_type: ProposalType,
        title: &str,
        description: &str,
        is_fast_track: bool,
    ) -> Result<String, String> {
        if proposer_balance < MIN_STAKE_TO_PROPOSE {
            return Err(format!("Недостаточно монет: {:.4} < {}", proposer_balance, MIN_STAKE_TO_PROPOSE));
        }
        let active = self.proposals.values()
            .filter(|p| p.status == ProposalStatus::Active).count();
        if active >= MAX_ACTIVE_PROPOSALS {
            return Err(format!("Максимум активных предложений: {}", MAX_ACTIVE_PROPOSALS));
        }
        let proposal = Proposal::new(proposer_id, proposal_type, title, description, is_fast_track);
        let id = proposal.id.clone();
        self.proposals.insert(id.clone(), proposal);
        Ok(id)
    }

    pub fn cast_vote(
        &mut self,
        proposal_id: &str,
        voter_id: &str,
        voter_balance: f64,
        in_favor: bool,
        comment: &str,
    ) -> Result<VoteResult, String> {
        if voter_balance < MIN_STAKE_TO_VOTE {
            return Err(format!("Недостаточно монет для голосования: {:.4}", voter_balance));
        }
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or_else(|| format!("Предложение не найдено: {}", proposal_id))?;
        if proposal.status != ProposalStatus::Active {
            return Err(format!("Голосование закрыто: {:?}", proposal.status));
        }
        if proposal.votes.iter().any(|v| v.voter_id == voter_id) {
            return Err(format!("Уже проголосовал: {}", voter_id));
        }
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let vote = Vote {
            voter_id: voter_id.to_string(),
            stake_weight: voter_balance,
            in_favor, comment: comment.to_string(), timestamp: now,
        };
        if in_favor { proposal.votes_for += voter_balance; }
        else { proposal.votes_against += voter_balance; }
        proposal.total_stake_participated += voter_balance;
        proposal.votes.push(vote);
        self.total_votes_cast += 1;
        self.total_stake_voted += voter_balance;
        Ok(VoteResult {
            proposal_id: proposal_id.to_string(),
            votes_for: proposal.votes_for,
            votes_against: proposal.votes_against,
            participation: proposal.total_stake_participated,
            current_result: if proposal.votes_for > proposal.votes_against {
                "ПРИНЯТО".to_string() } else { "ОТКЛОНЕНО".to_string() },
        })
    }

    pub fn finalize_proposal(
        &mut self,
        proposal_id: &str,
        total_supply: f64,
    ) -> Option<ExecutionResult> {
        let proposal = self.proposals.get_mut(proposal_id)?;
        if proposal.status != ProposalStatus::Active { return None; }
        let quorum = proposal.quorum_reached(total_supply);
        let passing = proposal.is_passing();
        proposal.status = if !quorum {
            ProposalStatus::Expired
        } else if passing {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        };
        if proposal.status == ProposalStatus::Passed {
            proposal.executed = true;
            self.executed_proposals.push(proposal_id.to_string());
            Some(ExecutionResult {
                proposal_id: proposal_id.to_string(),
                action: format!("{:?}", proposal.proposal_type),
                votes_for: proposal.votes_for,
                votes_against: proposal.votes_against,
                participation_rate: proposal.participation_rate(total_supply),
                success: true,
            })
        } else {
            Some(ExecutionResult {
                proposal_id: proposal_id.to_string(),
                action: format!("{:?}", proposal.proposal_type),
                votes_for: proposal.votes_for,
                votes_against: proposal.votes_against,
                participation_rate: proposal.participation_rate(total_supply),
                success: false,
            })
        }
    }

    pub fn active_proposals(&self) -> Vec<&Proposal> {
        let mut v: Vec<&Proposal> = self.proposals.values()
            .filter(|p| p.status == ProposalStatus::Active).collect();
        v.sort_by_key(|p| p.created_at);
        v
    }

    pub fn stats(&self) -> DaoStats {
        let total = self.proposals.len();
        let passed = self.proposals.values().filter(|p| p.status == ProposalStatus::Passed).count();
        let rejected = self.proposals.values().filter(|p| p.status == ProposalStatus::Rejected).count();
        let active = self.proposals.values().filter(|p| p.status == ProposalStatus::Active).count();
        DaoStats {
            total_proposals: total, passed, rejected, active,
            executed: self.executed_proposals.len(),
            total_votes_cast: self.total_votes_cast,
            total_stake_voted: self.total_stake_voted,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteResult {
    pub proposal_id: String,
    pub votes_for: f64,
    pub votes_against: f64,
    pub participation: f64,
    pub current_result: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub proposal_id: String,
    pub action: String,
    pub votes_for: f64,
    pub votes_against: f64,
    pub participation_rate: f64,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaoStats {
    pub total_proposals: usize,
    pub passed: usize,
    pub rejected: usize,
    pub active: usize,
    pub executed: usize,
    pub total_votes_cast: u64,
    pub total_stake_voted: f64,
}

impl std::fmt::Display for DaoStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  FEDERATION DAO — GOVERNANCE STATUS          ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Предложений: {:>4} (активных: {:>3})         ║\n\
             ║  Принято:     {:>4}  Отклонено: {:>3}         ║\n\
             ║  Исполнено:   {:>4}                           ║\n\
             ║  Голосов:     {:>4}  Монет:  {:>8.2}         ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_proposals, self.active,
            self.passed, self.rejected,
            self.executed,
            self.total_votes_cast, self.total_stake_voted,
        )
    }
}

// =============================================================================
// PHASE 7 / STEP 10 — Меритократическое Правительство
// Вес голоса = Reputation^0.7  (нелинейная меритократия)
// Ветераны влияют на прошивку Федерации
// =============================================================================

pub const MERIT_EXPONENT: f64       = 0.7;   // Reputation^0.7
pub const VETERAN_THRESHOLD: f64    = 100.0; // ≥100 = Veteran
pub const ELDER_THRESHOLD: f64      = 500.0; // ≥500 = Elder
pub const FOUNDING_THRESHOLD: f64   = 1000.0;// ≥1000 = Founding Father
pub const FIRMWARE_QUORUM: f64      = 0.67;  // 2/3 для прошивки
pub const EMERGENCY_QUORUM: f64     = 0.51;  // простое большинство
pub const DELEGATE_MAX: usize       = 5;     // максимум делегатов

// -----------------------------------------------------------------------------
// MeritTier — уровень влияния
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MeritTier {
    Newcomer,       // <10 rep
    Member,         // 10-49
    Veteran,        // 50-99 (исправлено: ≥50 как в reputation.rs)
    Elder,          // 100-499
    FoundingFather, // ≥500
}

impl MeritTier {
    pub fn from_rep(rep: f64) -> Self {
        if rep >= FOUNDING_THRESHOLD      { MeritTier::FoundingFather }
        else if rep >= ELDER_THRESHOLD    { MeritTier::Elder }
        else if rep >= VETERAN_THRESHOLD  { MeritTier::Veteran }
        else if rep >= 10.0              { MeritTier::Member }
        else                              { MeritTier::Newcomer }
    }
    pub fn name(&self) -> &str {
        match self {
            MeritTier::Newcomer       => "🌱 Newcomer",
            MeritTier::Member         => "⚙️  Member",
            MeritTier::Veteran        => "⚔️  Veteran",
            MeritTier::Elder          => "🏛️  Elder",
            MeritTier::FoundingFather => "👑 Founding Father",
        }
    }
    pub fn can_propose(&self) -> bool {
        !matches!(self, MeritTier::Newcomer)
    }
    pub fn can_veto_firmware(&self) -> bool {
        matches!(self, MeritTier::Elder | MeritTier::FoundingFather)
    }
}

// -----------------------------------------------------------------------------
// VotingPower — меритократический вес
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingPower {
    pub node_id: String,
    pub reputation: f64,
    pub raw_weight: f64,      // reputation^0.7
    pub delegate_bonus: f64,  // делегированная сила
    pub total_weight: f64,
    pub tier: MeritTier,
    pub delegated_to: Option<String>,
    pub delegates: Vec<String>,
}

impl VotingPower {
    pub fn compute(node_id: &str, reputation: f64) -> Self {
        let raw_weight = reputation.max(0.0).powf(MERIT_EXPONENT);
        let tier = MeritTier::from_rep(reputation);
        VotingPower {
            node_id: node_id.to_string(),
            reputation, raw_weight,
            delegate_bonus: 0.0,
            total_weight: raw_weight,
            tier, delegated_to: None,
            delegates: vec![],
        }
    }

    pub fn add_delegate_power(&mut self, delegate_weight: f64) {
        if self.delegates.len() < DELEGATE_MAX {
            self.delegate_bonus += delegate_weight * 0.5; // 50% делегированной силы
            self.total_weight = self.raw_weight + self.delegate_bonus;
        }
    }
}

// -----------------------------------------------------------------------------
// FirmwareProposal — предложение по прошивке
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareProposal {
    pub proposal_id: u64,
    pub proposer: String,
    pub proposer_tier: MeritTier,
    pub kind: FirmwareKind,
    pub description: String,
    pub code_hash: String,
    pub votes_for: f64,       // сумма весов ЗА
    pub votes_against: f64,   // сумма весов ПРОТИВ
    pub vetoes: Vec<String>,  // Elder/FoundingFather вето
    pub status: FirmwareStatus,
    pub required_quorum: f64,
    pub timestamp: i64,
    pub voters: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FirmwareKind {
    TacticUpdate      { tactic: String, params: String },
    EthicsRule        { rule: String, threshold: f64 },
    MintParam         { param: String, old_val: f64, new_val: f64 },
    ReputationAlgo    { change: String },
    NetworkProtocol   { protocol: String, version: String },
    EmergencyPatch    { cve: String, severity: u8 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FirmwareStatus {
    Active,
    Passed,
    Rejected,
    Vetoed,
    Expired,
}

impl FirmwareKind {
    pub fn name(&self) -> &str {
        match self {
            FirmwareKind::TacticUpdate    {..} => "TacticUpdate",
            FirmwareKind::EthicsRule      {..} => "EthicsRule",
            FirmwareKind::MintParam       {..} => "MintParam",
            FirmwareKind::ReputationAlgo  {..} => "ReputationAlgo",
            FirmwareKind::NetworkProtocol {..} => "NetworkProtocol",
            FirmwareKind::EmergencyPatch  {..} => "EmergencyPatch",
        }
    }
    pub fn required_quorum(&self) -> f64 {
        match self {
            FirmwareKind::EmergencyPatch {..} => EMERGENCY_QUORUM,
            _                                  => FIRMWARE_QUORUM,
        }
    }
}

// -----------------------------------------------------------------------------
// MeritocracyDao — главный орган управления
// -----------------------------------------------------------------------------

pub struct MeritocracyDao {
    pub voting_powers: std::collections::HashMap<String, VotingPower>,
    pub firmware_proposals: Vec<FirmwareProposal>,
    pub total_weight: f64,
    pub proposals_passed: u64,
    pub proposals_vetoed: u64,
    pub counter: u64,
}

impl MeritocracyDao {
    pub fn new() -> Self {
        MeritocracyDao {
            voting_powers: std::collections::HashMap::new(),
            firmware_proposals: vec![],
            total_weight: 0.0,
            proposals_passed: 0,
            proposals_vetoed: 0,
            counter: 0,
        }
    }

    fn now() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64
    }

    pub fn register_voter(&mut self, node_id: &str, reputation: f64) {
        let vp = VotingPower::compute(node_id, reputation);
        self.total_weight += vp.raw_weight;
        self.voting_powers.insert(node_id.to_string(), vp);
    }

    pub fn delegate(&mut self, from: &str, to: &str) -> bool {
        let from_weight = self.voting_powers.get(from)
            .map(|v| v.raw_weight).unwrap_or(0.0);
        if from_weight == 0.0 { return false; }

        if let Some(vp) = self.voting_powers.get_mut(from) {
            vp.delegated_to = Some(to.to_string());
        }
        if let Some(vp) = self.voting_powers.get_mut(to) {
            vp.delegates.push(from.to_string());
            vp.add_delegate_power(from_weight);
        }
        true
    }

    pub fn submit_firmware(&mut self, proposer: &str,
        kind: FirmwareKind, description: &str, code_hash: &str)
        -> Result<u64, String> {

        let vp = self.voting_powers.get(proposer)
            .ok_or("узел не зарегистрирован")?;
        if !vp.tier.can_propose() {
            return Err(format!("недостаточный ранг: {}", vp.tier.name()));
        }
        let tier = vp.tier.clone();
        let quorum = kind.required_quorum();
        self.counter += 1;

        self.firmware_proposals.push(FirmwareProposal {
            proposal_id: self.counter,
            proposer: proposer.to_string(),
            proposer_tier: tier,
            kind, description: description.to_string(),
            code_hash: code_hash.to_string(),
            votes_for: 0.0, votes_against: 0.0,
            vetoes: vec![], status: FirmwareStatus::Active,
            required_quorum: quorum, timestamp: Self::now(),
            voters: std::collections::HashMap::new(),
        });
        Ok(self.counter)
    }

    pub fn vote_firmware(&mut self, proposal_id: u64,
        voter: &str, approve: bool) -> VoteFirmwareResult {

        let vp = match self.voting_powers.get(voter) {
            None => return VoteFirmwareResult::denied("не зарегистрирован"),
            Some(v) => (v.total_weight, v.tier.clone()),
        };

        let prop = match self.firmware_proposals.iter_mut()
            .find(|p| p.proposal_id == proposal_id) {
            None => return VoteFirmwareResult::denied("предложение не найдено"),
            Some(p) => p,
        };

        if prop.status != FirmwareStatus::Active {
            return VoteFirmwareResult::denied("голосование закрыто");
        }
        if prop.voters.contains_key(voter) {
            return VoteFirmwareResult::denied("уже проголосовал");
        }

        prop.voters.insert(voter.to_string(), approve);
        if approve { prop.votes_for    += vp.0; }
        else       { prop.votes_against += vp.0; }

        // Elder/FoundingFather может наложить вето
        if !approve && vp.1.can_veto_firmware() {
            prop.vetoes.push(voter.to_string());
            if prop.vetoes.len() >= 2 {
                prop.status = FirmwareStatus::Vetoed;
                return VoteFirmwareResult::vetoed(voter, prop.votes_for, prop.votes_against);
            }
        }

        VoteFirmwareResult {
            success: true, voter: voter.to_string(),
            weight: vp.0, approve,
            votes_for: prop.votes_for,
            votes_against: prop.votes_against,
            status: prop.status.clone(),
            reason: "OK".into(),
        }
    }

    pub fn finalize(&mut self, proposal_id: u64) -> FinalizeResult {
        let total = self.total_weight;
        let prop = match self.firmware_proposals.iter_mut()
            .find(|p| p.proposal_id == proposal_id) {
            None => return FinalizeResult { passed: false, reason: "не найдено".into(),
                votes_for: 0.0, votes_against: 0.0, participation: 0.0 },
            Some(p) => p,
        };

        if prop.status == FirmwareStatus::Vetoed {
            return FinalizeResult { passed: false, reason: "VETO".into(),
                votes_for: prop.votes_for, votes_against: prop.votes_against,
                participation: (prop.votes_for + prop.votes_against) / total };
        }

        let participation = (prop.votes_for + prop.votes_against) / total;
        let approval = if prop.votes_for + prop.votes_against > 0.0 {
            prop.votes_for / (prop.votes_for + prop.votes_against)
        } else { 0.0 };

        let passed = participation >= 0.10 && approval >= prop.required_quorum;
        prop.status = if passed { FirmwareStatus::Passed } else { FirmwareStatus::Rejected };
        if passed { self.proposals_passed += 1; }

        FinalizeResult { passed, votes_for: prop.votes_for,
            votes_against: prop.votes_against, participation,
            reason: format!("approval={:.1}% quorum={:.1}%",
                approval*100.0, prop.required_quorum*100.0) }
    }

    pub fn power_distribution(&self) -> Vec<(&str, f64, f64, &str)> {
        let mut dist: Vec<(&str, f64, f64, &str)> = self.voting_powers.values()
            .map(|v| (v.node_id.as_str(), v.reputation,
                      v.total_weight, v.tier.name()))
            .collect();
        dist.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
        dist
    }
}

impl Default for MeritocracyDao { fn default() -> Self { Self::new() } }

#[derive(Debug)]
pub struct VoteFirmwareResult {
    pub success: bool, pub voter: String, pub weight: f64,
    pub approve: bool, pub votes_for: f64, pub votes_against: f64,
    pub status: FirmwareStatus, pub reason: String,
}

impl VoteFirmwareResult {
    pub fn denied(reason: &str) -> Self {
        VoteFirmwareResult { success:false, voter:"".into(), weight:0.0,
            approve:false, votes_for:0.0, votes_against:0.0,
            status:FirmwareStatus::Active, reason:reason.into() }
    }
    pub fn vetoed(voter: &str, vf: f64, va: f64) -> Self {
        VoteFirmwareResult { success:true, voter:voter.into(), weight:0.0,
            approve:false, votes_for:vf, votes_against:va,
            status:FirmwareStatus::Vetoed, reason:"ELDER_VETO".into() }
    }
}

#[derive(Debug)]
pub struct FinalizeResult {
    pub passed: bool, pub reason: String,
    pub votes_for: f64, pub votes_against: f64, pub participation: f64,
}
