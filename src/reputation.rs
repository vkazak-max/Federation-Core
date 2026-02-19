// =============================================================================
// FEDERATION CORE — reputation.rs
// PHASE 5 / STEP 3 — «Reputation & Social Capital»
// =============================================================================
//
// Репутация = f(время, успехи, этика, регион)
// Формула накопления:
//   rep += delivery_weight * region_difficulty * tactic_multiplier
//   rep -= betrayal_penalty (необратимо при помощи цензору)
//
// Уровни: Ghost → Newcomer → Reliable → Trusted → Veteran → Legend
// DAO вес = reputation_score ^ DAO_WEIGHT_EXPONENT
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const REP_BASE_DELIVERY: f64    = 0.10;  // за каждую доставку
pub const REP_AIKI_MULT: f64        = 3.0;   // множитель AikiReflection
pub const REP_STRIKE_MULT: f64      = 2.0;   // множитель CumulativeStrike
pub const REP_DECOY_MULT: f64       = 1.5;   // множитель StandoffDecoy
pub const REP_BETRAYAL_SLASH: f64   = 0.50;  // -50% репутации за предательство
pub const REP_FAILURE_DECAY: f64    = 0.02;  // -2% за каждый провал
pub const DAO_WEIGHT_EXPONENT: f64  = 0.7;   // сглаживание для DAO
pub const LEGEND_THRESHOLD: f64     = 500.0;
pub const VETERAN_THRESHOLD: f64    = 100.0;
pub const TRUSTED_THRESHOLD: f64    = 30.0;
pub const RELIABLE_THRESHOLD: f64   = 10.0;
pub const NEWCOMER_THRESHOLD: f64   = 1.0;

// -----------------------------------------------------------------------------
// ReputationTier — уровень репутации
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReputationTier {
    Ghost,      // < 1.0   — неизвестный
    Newcomer,   // 1-10    — новичок
    Reliable,   // 10-30   — надёжный
    Trusted,    // 30-100  — доверенный
    Veteran,    // 100-500 — ветеран
    Legend,     // 500+    — легенда
}

impl ReputationTier {
    pub fn from_score(score: f64) -> Self {
        if score >= LEGEND_THRESHOLD  { ReputationTier::Legend   }
        else if score >= VETERAN_THRESHOLD  { ReputationTier::Veteran  }
        else if score >= TRUSTED_THRESHOLD  { ReputationTier::Trusted  }
        else if score >= RELIABLE_THRESHOLD { ReputationTier::Reliable }
        else if score >= NEWCOMER_THRESHOLD { ReputationTier::Newcomer }
        else                                { ReputationTier::Ghost    }
    }

    pub fn name(&self) -> &str {
        match self {
            ReputationTier::Ghost    => "👻 Ghost",
            ReputationTier::Newcomer => "🌱 Newcomer",
            ReputationTier::Reliable => "🔵 Reliable",
            ReputationTier::Trusted  => "🟣 Trusted",
            ReputationTier::Veteran  => "🟠 Veteran",
            ReputationTier::Legend   => "🌟 Legend",
        }
    }

    pub fn dao_weight_bonus(&self) -> f64 {
        match self {
            ReputationTier::Ghost    => 0.0,
            ReputationTier::Newcomer => 0.5,
            ReputationTier::Reliable => 1.0,
            ReputationTier::Trusted  => 1.5,
            ReputationTier::Veteran  => 2.0,
            ReputationTier::Legend   => 3.0,
        }
    }
}

// -----------------------------------------------------------------------------
// ReputationEvent — одно событие влияющее на репутацию
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEventKind {
    SuccessfulDelivery { tactic: String, region_difficulty: f64 },
    FailedDelivery     { region: String },
    AikiVictory        { censor_cpu_drained: f64 },
    EthicsViolation    { violation: String, severity: f64 },
    Betrayal           { evidence_hash: String },
    DaoParticipation   { proposal_id: String },
    LongTermUptime     { days: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub node_id: String,
    pub kind: ReputationEventKind,
    pub rep_delta: f64,
    pub timestamp: i64,
    pub is_slash: bool,
}

// -----------------------------------------------------------------------------
// NodeReputation — профиль репутации узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReputation {
    pub node_id: String,
    pub score: f64,
    pub tier: ReputationTier,
    pub total_deliveries: u64,
    pub successful_deliveries: u64,
    pub aiki_victories: u64,
    pub betrayals: u32,           // необратимо фиксируется
    pub ethics_violations: u32,
    pub uptime_days: u32,
    pub dao_participations: u32,
    pub history: Vec<ReputationEvent>,
    pub is_blacklisted: bool,     // после 3 предательств
    pub stake: f64,               // репутационный залог
}

impl NodeReputation {
    pub fn new(node_id: &str) -> Self {
        NodeReputation {
            node_id: node_id.to_string(),
            score: 0.0,
            tier: ReputationTier::Ghost,
            total_deliveries: 0,
            successful_deliveries: 0,
            aiki_victories: 0,
            betrayals: 0,
            ethics_violations: 0,
            uptime_days: 0,
            dao_participations: 0,
            history: vec![],
            is_blacklisted: false,
            stake: 0.0,
        }
    }

    pub fn delivery_rate(&self) -> f64 {
        if self.total_deliveries == 0 { return 0.0; }
        self.successful_deliveries as f64 / self.total_deliveries as f64
    }

    pub fn dao_voting_weight(&self) -> f64 {
        if self.is_blacklisted { return 0.0; }
        // DAO вес = score^0.7 + tier_bonus
        // Нельзя купить — только заработать
        let base = self.score.powf(DAO_WEIGHT_EXPONENT);
        base + self.tier.dao_weight_bonus()
    }

    pub fn update_tier(&mut self) {
        self.tier = ReputationTier::from_score(self.score);
    }
}

// -----------------------------------------------------------------------------
// ReputationRegistry — реестр репутаций
// -----------------------------------------------------------------------------

pub struct ReputationRegistry {
    pub nodes: HashMap<String, NodeReputation>,
    pub total_events: u64,
    pub total_slashes: u64,
    pub blacklisted_count: u32,
}

impl ReputationRegistry {
    pub fn new() -> Self {
        ReputationRegistry {
            nodes: HashMap::new(),
            total_events: 0,
            total_slashes: 0,
            blacklisted_count: 0,
        }
    }

    fn get_or_create(&mut self, node_id: &str) -> &mut NodeReputation {
        self.nodes.entry(node_id.to_string())
            .or_insert_with(|| NodeReputation::new(node_id))
    }

    fn now() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64
    }

    /// Успешная доставка
    pub fn record_delivery(&mut self, node_id: &str, tactic: &str,
                            region_difficulty: f64) -> f64 {
        let tactic_mult = match tactic {
            "AikiReflection"   => REP_AIKI_MULT,
            "CumulativeStrike" => REP_STRIKE_MULT,
            "StandoffDecoy"    => REP_DECOY_MULT,
            "Hybrid"           => (REP_AIKI_MULT + REP_STRIKE_MULT) / 2.0,
            _                  => 1.0,
        };
        let delta = REP_BASE_DELIVERY * (1.0 + region_difficulty * 3.0) * tactic_mult;

        let node = self.get_or_create(node_id);
        if node.is_blacklisted { return 0.0; }
        node.score += delta;
        node.stake += delta * 0.1;
        node.total_deliveries += 1;
        node.successful_deliveries += 1;
        node.update_tier();

        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::SuccessfulDelivery {
                tactic: tactic.to_string(),
                region_difficulty,
            },
            rep_delta: delta, timestamp: Self::now(), is_slash: false,
        };
        node.history.push(event);
        self.total_events += 1;
        delta
    }

    /// Провал доставки
    pub fn record_failure(&mut self, node_id: &str, region: &str) -> f64 {
        let node = self.get_or_create(node_id);
        let delta = -(node.score * REP_FAILURE_DECAY).max(0.01);
        node.score = (node.score + delta).max(0.0);
        node.total_deliveries += 1;
        node.stake = (node.stake + delta * 0.05).max(0.0);
        node.update_tier();
        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::FailedDelivery {
                region: region.to_string() },
            rep_delta: delta, timestamp: Self::now(), is_slash: false,
        };
        node.history.push(event);
        self.total_events += 1;
        delta
    }

    /// Победа Айкидо — особая награда
    pub fn record_aiki_victory(&mut self, node_id: &str,
                                cpu_drained: f64) -> f64 {
        let delta = cpu_drained * REP_AIKI_MULT * 2.0;
        let node = self.get_or_create(node_id);
        if node.is_blacklisted { return 0.0; }
        node.score += delta;
        node.stake += delta * 0.2;
        node.aiki_victories += 1;
        node.update_tier();
        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::AikiVictory { censor_cpu_drained: cpu_drained },
            rep_delta: delta, timestamp: Self::now(), is_slash: false,
        };
        node.history.push(event);
        self.total_events += 1;
        delta
    }

    /// Нарушение этики
    pub fn record_ethics_violation(&mut self, node_id: &str,
                                    violation: &str, severity: f64) -> f64 {
        let node = self.get_or_create(node_id);
        let delta = -(node.score * severity * 0.3).max(0.1);
        node.score = (node.score + delta).max(0.0);
        node.ethics_violations += 1;
        node.update_tier();
        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::EthicsViolation {
                violation: violation.to_string(), severity },
            rep_delta: delta, timestamp: Self::now(), is_slash: true,
        };
        node.history.push(event);
        self.total_events += 1;
        self.total_slashes += 1;
        delta
    }

    /// ПРЕДАТЕЛЬСТВО — помощь цензору
    pub fn record_betrayal(&mut self, node_id: &str,
                           evidence_hash: &str) -> f64 {
        // Вычисляем всё до изменения self
        let (slash, newly_blacklisted) = {
            let node = self.get_or_create(node_id);
            let slash = node.score * REP_BETRAYAL_SLASH;
            node.score -= slash;
            node.stake = 0.0;
            node.betrayals += 1;
            let newly_blacklisted = if node.betrayals >= 3 {
                node.is_blacklisted = true;
                node.score = 0.0;
                true
            } else { false };
            node.update_tier();
            let event = ReputationEvent {
                node_id: node_id.to_string(),
                kind: ReputationEventKind::Betrayal {
                    evidence_hash: evidence_hash.to_string() },
                rep_delta: -slash, timestamp: Self::now(), is_slash: true,
            };
            node.history.push(event);
            (slash, newly_blacklisted)
        };
        // Теперь borrow закрыт — можно обновить счётчики
        if newly_blacklisted { self.blacklisted_count += 1; }
        self.total_events += 1;
        self.total_slashes += 1;
        -slash
    }

    /// Участие в DAO голосовании
    pub fn record_dao_participation(&mut self, node_id: &str,
                                    proposal_id: &str) -> f64 {
        let node = self.get_or_create(node_id);
        let delta = 0.05 * node.tier.dao_weight_bonus().max(0.1);
        node.score += delta;
        node.dao_participations += 1;
        node.update_tier();
        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::DaoParticipation {
                proposal_id: proposal_id.to_string() },
            rep_delta: delta, timestamp: Self::now(), is_slash: false,
        };
        node.history.push(event);
        self.total_events += 1;
        delta
    }

    /// Долгосрочный аптайм
    pub fn record_uptime(&mut self, node_id: &str, days: u32) -> f64 {
        let node = self.get_or_create(node_id);
        let delta = (days as f64).sqrt() * 0.5;
        node.score += delta;
        node.uptime_days += days;
        node.update_tier();
        let event = ReputationEvent {
            node_id: node_id.to_string(),
            kind: ReputationEventKind::LongTermUptime { days },
            rep_delta: delta, timestamp: Self::now(), is_slash: false,
        };
        node.history.push(event);
        self.total_events += 1;
        delta
    }

    pub fn leaderboard(&self, n: usize) -> Vec<(&NodeReputation, usize)> {
        let mut v: Vec<&NodeReputation> = self.nodes.values()
            .filter(|n| !n.is_blacklisted).collect();
        v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        v.into_iter().take(n).enumerate()
            .map(|(i, n)| (n, i + 1)).collect()
    }

    pub fn dao_weights(&self) -> Vec<(String, f64)> {
        let mut v: Vec<(String, f64)> = self.nodes.values()
            .map(|n| (n.node_id.clone(), n.dao_voting_weight()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v
    }

    pub fn stats(&self) -> RegistryStats {
        let scores: Vec<f64> = self.nodes.values()
            .map(|n| n.score).collect();
        let avg = if scores.is_empty() { 0.0 }
            else { scores.iter().sum::<f64>() / scores.len() as f64 };
        RegistryStats {
            total_nodes: self.nodes.len(),
            total_events: self.total_events,
            total_slashes: self.total_slashes,
            blacklisted: self.blacklisted_count,
            avg_score: avg,
            legends: self.nodes.values()
                .filter(|n| n.tier == ReputationTier::Legend).count(),
            veterans: self.nodes.values()
                .filter(|n| n.tier == ReputationTier::Veteran).count(),
        }
    }
}

impl Default for ReputationRegistry {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_nodes: usize,
    pub total_events: u64,
    pub total_slashes: u64,
    pub blacklisted: u32,
    pub avg_score: f64,
    pub legends: usize,
    pub veterans: usize,
}

impl std::fmt::Display for RegistryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  REPUTATION REGISTRY — STATS                 ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Узлов:  {:>4}  Событий: {:>6}  Slash: {:>3}  ║\n\
             ║  Бан:    {:>4}  Среднее: {:>8.3}             ║\n\
             ║  Legend: {:>4}  Veteran: {:>4}               ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_nodes, self.total_events, self.total_slashes,
            self.blacklisted, self.avg_score,
            self.legends, self.veterans,
        )
    }
}

// =============================================================================
// TRUST GRAPH — Phase 8 Patch
// Доверие распространяется по ссылкам как PageRank
//
//   TrustEdge    — направленная связь доверия A→B
//   TrustGraph   — полный граф доверия сети
//   TrustRank    — итеративный расчёт влияния узла
// =============================================================================

pub const TRUST_DECAY: f64         = 0.15; // затухание по хопам
pub const PAGERANK_DAMPING: f64    = 0.85; // классический PageRank d
pub const PAGERANK_ITERATIONS: u32 = 20;   // итераций сходимости
pub const MIN_TRUST_EDGE: f64      = 0.10; // минимальный вес ребра

// -----------------------------------------------------------------------------
// TrustEdge — направленное ребро доверия
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEdge {
    pub from: String,
    pub to: String,
    pub weight: f64,       // 0.0-1.0
    pub vouches: u32,      // сколько раз поручились
    pub betrayals: u32,    // сколько раз предали
    pub created_at: i64,
}

impl TrustEdge {
    pub fn new(from: &str, to: &str, initial_weight: f64) -> Self {
        TrustEdge {
            from: from.to_string(), to: to.to_string(),
            weight: initial_weight.clamp(0.0, 1.0),
            vouches: 1, betrayals: 0,
            created_at: 0,
        }
    }
    pub fn effective_weight(&self) -> f64 {
        if self.betrayals > 0 {
            (self.weight * 0.5f64.powi(self.betrayals as i32)).max(0.0)
        } else {
            self.weight
        }
    }
    pub fn vouch(&mut self) {
        self.vouches += 1;
        self.weight = (self.weight + 0.05).min(1.0);
    }
    pub fn betray(&mut self) {
        self.betrayals += 1;
        self.weight = (self.weight * 0.5).max(0.0);
    }
}

// -----------------------------------------------------------------------------
// TrustGraph — граф доверия
// -----------------------------------------------------------------------------

pub struct TrustGraph {
    pub edges: Vec<TrustEdge>,
    pub trust_ranks: HashMap<String, f64>,
    pub iterations_run: u32,
}

impl TrustGraph {
    pub fn new() -> Self {
        TrustGraph { edges: vec![], trust_ranks: HashMap::new(), iterations_run: 0 }
    }

    pub fn add_edge(&mut self, from: &str, to: &str, weight: f64) {
        if let Some(e) = self.edges.iter_mut()
            .find(|e| e.from == from && e.to == to) {
            e.weight = weight.clamp(0.0, 1.0);
        } else {
            self.edges.push(TrustEdge::new(from, to, weight));
        }
    }

    pub fn vouch(&mut self, from: &str, to: &str) {
        if let Some(e) = self.edges.iter_mut()
            .find(|e| e.from == from && e.to == to) {
            e.vouch();
        } else {
            self.edges.push(TrustEdge::new(from, to, 0.5));
        }
    }

    pub fn betray(&mut self, from: &str, to: &str) {
        if let Some(e) = self.edges.iter_mut()
            .find(|e| e.from == from && e.to == to) {
            e.betray();
        }
    }

    // Все узлы в графе
    fn all_nodes(&self) -> Vec<String> {
        let mut nodes = std::collections::HashSet::new();
        for e in &self.edges {
            nodes.insert(e.from.clone());
            nodes.insert(e.to.clone());
        }
        nodes.into_iter().collect()
    }

    // Исходящие рёбра от узла
    fn outgoing(&self, node: &str) -> Vec<&TrustEdge> {
        self.edges.iter().filter(|e| e.from == node).collect()
    }

    // PageRank-подобный алгоритм для доверия
    pub fn compute_trust_rank(&mut self, seed_reputations: &HashMap<String, f64>) {
        let nodes = self.all_nodes();
        let n = nodes.len().max(1) as f64;

        // Инициализация: базовое доверие пропорционально репутации
        let total_rep: f64 = seed_reputations.values().sum::<f64>().max(1.0);
        let mut ranks: HashMap<String, f64> = nodes.iter().map(|node| {
            let base = seed_reputations.get(node).copied().unwrap_or(1.0);
            (node.clone(), base / total_rep)
        }).collect();

        // PageRank итерации
        for _ in 0..PAGERANK_ITERATIONS {
            let mut new_ranks: HashMap<String, f64> = nodes.iter()
                .map(|nd| (nd.clone(), (1.0 - PAGERANK_DAMPING) / n)).collect();

            for node in &nodes {
                let outgoing = self.outgoing(node);
                let total_weight: f64 = outgoing.iter()
                    .map(|e| e.effective_weight()).sum::<f64>().max(1e-9);
                let rank = ranks.get(node).copied().unwrap_or(0.0);

                for edge in outgoing {
                    let contribution = PAGERANK_DAMPING * rank
                        * (edge.effective_weight() / total_weight);
                    *new_ranks.entry(edge.to.clone()).or_insert(0.0) += contribution;
                }
            }
            ranks = new_ranks;
        }

        // Нормализуем 0..1
        let max_rank = ranks.values().cloned().fold(0.0f64, f64::max).max(1e-9);
        self.trust_ranks = ranks.into_iter()
            .map(|(k,v)| (k, v / max_rank)).collect();
        self.iterations_run = PAGERANK_ITERATIONS;
    }

    pub fn trust_rank_of(&self, node: &str) -> f64 {
        self.trust_ranks.get(node).copied().unwrap_or(0.0)
    }

    // Транзитивное доверие от A до B через граф (BFS с затуханием)
    pub fn transitive_trust(&self, from: &str, to: &str) -> f64 {
        if from == to { return 1.0; }
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((from.to_string(), 1.0f64));
        visited.insert(from.to_string());
        let mut best = 0.0f64;

        while let Some((node, trust)) = queue.pop_front() {
            if trust < MIN_TRUST_EDGE { continue; }
            for edge in self.outgoing(&node) {
                let new_trust = trust * edge.effective_weight() * (1.0 - TRUST_DECAY);
                if edge.to == to { best = best.max(new_trust); continue; }
                if !visited.contains(&edge.to) && new_trust >= MIN_TRUST_EDGE {
                    visited.insert(edge.to.clone());
                    queue.push_back((edge.to.clone(), new_trust));
                }
            }
        }
        best
    }

    pub fn top_trusted(&self, n: usize) -> Vec<(&str, f64)> {
        let mut v: Vec<(&str, f64)> = self.trust_ranks.iter()
            .map(|(k,v)| (k.as_str(), *v)).collect();
        v.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
        v.into_iter().take(n).collect()
    }

    pub fn stats(&self) -> String {
        format!("nodes={}  edges={}  iterations={}",
            self.all_nodes().len(), self.edges.len(), self.iterations_run)
    }
}

impl Default for TrustGraph { fn default() -> Self { Self::new() } }
