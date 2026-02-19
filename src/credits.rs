// =============================================================================
// FEDERATION CORE — credits.rs
// PHASE 5 / STEP 1 — «Proof-of-Bypass (PoB)»
// =============================================================================
//
// Экономика Федерации:
//   Credits = f(bypass_success, region_difficulty, tactic_complexity)
//
// Формула:
//   base_reward = BYPASS_BASE * difficulty_multiplier
//   tactic_bonus = base * tactic_coefficient
//   total = (base + tactic_bonus) * streak_multiplier
//
// Чем сложнее регион и тактика — тем выше награда.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BYPASS_BASE: f64        = 1.0;   // базовая награда за прорыв
pub const AIKI_BONUS: f64         = 2.5;   // коэффициент за AikiReflection
pub const STRIKE_BONUS: f64       = 1.8;   // коэффициент за CumulativeStrike
pub const DECOY_BONUS: f64        = 1.2;   // коэффициент за StandoffDecoy
pub const PASSIVE_BONUS: f64      = 0.5;   // коэффициент за пассивный маршрут
pub const MAX_STREAK_MULT: f64    = 3.0;   // максимальный множитель серии
pub const STREAK_STEP: f64        = 0.1;   // шаг роста серии
pub const DIFFICULTY_SCALE: f64   = 4.0;   // масштаб сложности региона
pub const EVIDENCE_BONUS: f64     = 0.3;   // бонус за публикацию доказательств

// -----------------------------------------------------------------------------
// RegionDifficulty — сложность региона
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDifficulty {
    pub region_code: String,
    pub block_rate: f64,          // текущая блокировка (0..1)
    pub dpi_level: u8,            // уровень DPI (1-5)
    pub has_ml_censor: bool,      // есть ML-цензура?
    pub internet_shutdown: bool,  // были shutdown-ы?
    pub difficulty_score: f64,    // итоговый балл сложности
}

impl RegionDifficulty {
    pub fn new(code: &str, block_rate: f64, dpi_level: u8,
               has_ml: bool, shutdown: bool) -> Self {
        let mut r = RegionDifficulty {
            region_code: code.to_string(),
            block_rate, dpi_level, has_ml_censor: has_ml,
            internet_shutdown: shutdown, difficulty_score: 0.0,
        };
        r.difficulty_score = r.compute_difficulty();
        r
    }

    pub fn compute_difficulty(&self) -> f64 {
        let base = self.block_rate;
        let dpi_factor = self.dpi_level as f64 / 5.0 * 0.3;
        let ml_factor = if self.has_ml_censor { 0.2 } else { 0.0 };
        let shutdown_factor = if self.internet_shutdown { 0.3 } else { 0.0 };
        (base + dpi_factor + ml_factor + shutdown_factor).min(1.0)
    }

    pub fn difficulty_multiplier(&self) -> f64 {
        1.0 + self.difficulty_score * DIFFICULTY_SCALE
    }

    pub fn label(&self) -> &str {
        match self.difficulty_score as u8 {
            _ if self.difficulty_score >= 0.8 => "🔴 EXTREME",
            _ if self.difficulty_score >= 0.6 => "🟠 HIGH",
            _ if self.difficulty_score >= 0.4 => "🟡 MEDIUM",
            _ if self.difficulty_score >= 0.2 => "🟢 LOW",
            _                                  => "⚪ MINIMAL",
        }
    }
}

// Предустановленные регионы
pub fn known_regions() -> HashMap<String, RegionDifficulty> {
    let mut m = HashMap::new();
    let regions = vec![
        RegionDifficulty::new("CN", 0.85, 5, true,  false),
        RegionDifficulty::new("RU", 0.60, 4, false, true),
        RegionDifficulty::new("IR", 0.75, 4, false, true),
        RegionDifficulty::new("KP", 0.99, 5, false, true),
        RegionDifficulty::new("ET", 0.70, 3, false, true),
        RegionDifficulty::new("BY", 0.55, 3, false, true),
        RegionDifficulty::new("DE", 0.05, 1, false, false),
        RegionDifficulty::new("JP", 0.10, 2, false, false),
        RegionDifficulty::new("CA", 0.08, 2, false, false),
        RegionDifficulty::new("AU", 0.15, 2, false, false),
    ];
    for r in regions { m.insert(r.region_code.clone(), r); }
    m
}

// -----------------------------------------------------------------------------
// BypassEvent — одно успешное событие
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassEvent {
    pub event_id: u64,
    pub node_id: String,
    pub region: String,
    pub tactic: String,
    pub packets_delivered: u64,
    pub censor_cpu_drained: f64,   // сколько CPU цензора потрачено
    pub difficulty: f64,
    pub has_evidence: bool,        // опубликовано в DAG?
    pub credits_earned: f64,
    pub timestamp: i64,
}

impl BypassEvent {
    pub fn compute_credits(tactic: &str, packets: u64,
        difficulty_mult: f64, cpu_drained: f64,
        has_evidence: bool) -> f64 {

        let tactic_coeff = match tactic {
            "AikiReflection"   => AIKI_BONUS,
            "CumulativeStrike" => STRIKE_BONUS,
            "StandoffDecoy"    => DECOY_BONUS,
            "Hybrid"           => (AIKI_BONUS + STRIKE_BONUS) / 2.0,
            _                  => PASSIVE_BONUS,
        };

        let base = BYPASS_BASE * difficulty_mult;
        let tactic_bonus = base * tactic_coeff;
        let packet_bonus = (packets as f64).ln().max(1.0) * 0.1;
        let cpu_bonus = cpu_drained * 0.5; // бонус за истощение цензора
        let evidence_bonus = if has_evidence { EVIDENCE_BONUS } else { 0.0 };

        base + tactic_bonus + packet_bonus + cpu_bonus + evidence_bonus
    }
}

// -----------------------------------------------------------------------------
// CreditLedger — книга учёта credits
// -----------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CreditLedger {
    pub balances: HashMap<String, f64>,
    pub events: Vec<BypassEvent>,
    pub streaks: HashMap<String, u32>,      // серия успехов узла
    pub total_credits_issued: f64,
    pub event_counter: u64,
}

impl CreditLedger {
    pub fn new() -> Self { Self::default() }

    pub fn record_bypass(&mut self, node_id: &str, region: &str,
        tactic: &str, packets: u64, cpu_drained: f64,
        difficulty: &RegionDifficulty, has_evidence: bool) -> f64 {

        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;

        // Streak multiplier
        let streak = self.streaks.entry(node_id.to_string()).or_insert(0);
        *streak += 1;
        let streak_mult = (1.0 + *streak as f64 * STREAK_STEP)
            .min(MAX_STREAK_MULT);

        let base_credits = BypassEvent::compute_credits(
            tactic, packets,
            difficulty.difficulty_multiplier(),
            cpu_drained, has_evidence,
        );
        let total = base_credits * streak_mult;

        // Обновляем баланс
        *self.balances.entry(node_id.to_string()).or_insert(0.0) += total;
        self.total_credits_issued += total;
        self.event_counter += 1;

        let event = BypassEvent {
            event_id: self.event_counter,
            node_id: node_id.to_string(),
            region: region.to_string(),
            tactic: tactic.to_string(),
            packets_delivered: packets,
            censor_cpu_drained: cpu_drained,
            difficulty: difficulty.difficulty_score,
            has_evidence,
            credits_earned: total,
            timestamp: now,
        };
        self.events.push(event);
        total
    }

    pub fn record_failure(&mut self, node_id: &str) {
        // Провал сбрасывает серию
        if let Some(s) = self.streaks.get_mut(node_id) { *s = 0; }
    }

    pub fn balance(&self, node_id: &str) -> f64 {
        self.balances.get(node_id).cloned().unwrap_or(0.0)
    }

    pub fn top_nodes(&self, n: usize) -> Vec<(String, f64)> {
        let mut v: Vec<_> = self.balances.iter()
            .map(|(k, v)| (k.clone(), *v)).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        v.into_iter().take(n).collect()
    }

    pub fn stats(&self) -> LedgerStats {
        let avg = if !self.balances.is_empty() {
            self.balances.values().sum::<f64>() / self.balances.len() as f64
        } else { 0.0 };

        let max_streak = self.streaks.values().cloned().max().unwrap_or(0);
        let top = self.top_nodes(1);

        LedgerStats {
            total_nodes: self.balances.len(),
            total_events: self.events.len(),
            total_issued: self.total_credits_issued,
            avg_balance: avg,
            max_streak,
            top_node: top.into_iter().next(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerStats {
    pub total_nodes: usize,
    pub total_events: usize,
    pub total_issued: f64,
    pub avg_balance: f64,
    pub max_streak: u32,
    pub top_node: Option<(String, f64)>,
}

impl std::fmt::Display for LedgerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  PROOF-OF-BYPASS LEDGER — STATS              ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Узлов:    {:>4}  Событий:  {:>6}           ║\n\
             ║  Выдано:   {:>10.2} credits               ║\n\
             ║  Среднее:  {:>10.2} credits/узел          ║\n\
             ║  Макс.серия: {:>3}  Лидер: {:>14}        ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_nodes, self.total_events,
            self.total_issued, self.avg_balance,
            self.max_streak,
            self.top_node.as_ref().map(|(n,_)| n.as_str()).unwrap_or("?"),
        )
    }
}

// =============================================================================
// ECOLOGICAL BONUSES — Phase 8 Patch
// Зелёная экономика: старое железо = выше бонус
// Recycling Multiplier + Upgrade Fund взносы
// =============================================================================

pub const RECYCLE_BASE_BONUS: f64   = 0.30; // +30% за старое железо
pub const UPGRADE_FUND_RATE: f64    = 0.05; // 5% от кредитов → фонд апгрейда
pub const VETERAN_HW_YEARS: u32     = 3;    // железо ≥3 лет = "vintage"
pub const ANCIENT_HW_YEARS: u32     = 7;    // железо ≥7 лет = "ancient"
pub const MAX_RECYCLE_MULT: f64     = 2.5;  // потолок множителя

// -----------------------------------------------------------------------------
// HardwareAge — возраст железа влияет на бонус
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareAge {
    Modern,    // <3 лет — стандарт
    Vintage,   // 3-6 лет — +recycling бонус
    Ancient,   // 7+ лет — максимальный бонус
}

impl HardwareAge {
    pub fn from_years(years: u32) -> Self {
        if years >= ANCIENT_HW_YEARS        { HardwareAge::Ancient }
        else if years >= VETERAN_HW_YEARS   { HardwareAge::Vintage }
        else                                { HardwareAge::Modern }
    }
    pub fn recycle_multiplier(&self) -> f64 {
        match self {
            HardwareAge::Modern  => 1.00,
            HardwareAge::Vintage => 1.50, // +50%
            HardwareAge::Ancient => 2.50, // +150%
        }
    }
    pub fn label(&self) -> &str {
        match self {
            HardwareAge::Modern  => "🔵 Modern",
            HardwareAge::Vintage => "🟡 Vintage",
            HardwareAge::Ancient => "🟤 Ancient",
        }
    }
    pub fn description(&self) -> &str {
        match self {
            HardwareAge::Modern  => "новое железо",
            HardwareAge::Vintage => "ветеранское железо (+50% кредитов)",
            HardwareAge::Ancient => "древнее железо (+150% кредитов)",
        }
    }
}

// -----------------------------------------------------------------------------
// EcoProfile — экологический профиль узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EcoProfile {
    pub node_id: String,
    pub hw_age_years: u32,
    pub hw_age: HardwareAge,
    pub recycle_mult: f64,
    pub total_eco_earned: f64,    // всего заработано через recycle
    pub upgrade_fund_paid: f64,   // всего внесено в фонд апгрейда
    pub is_recycled_device: bool, // устройство из вторсырья
}

impl EcoProfile {
    pub fn new(node_id: &str, hw_age_years: u32, is_recycled: bool) -> Self {
        let hw_age = HardwareAge::from_years(hw_age_years);
        let mut recycle_mult = hw_age.recycle_multiplier();
        if is_recycled { recycle_mult = (recycle_mult * 1.2).min(MAX_RECYCLE_MULT); }
        EcoProfile {
            node_id: node_id.to_string(),
            hw_age_years, hw_age, recycle_mult,
            total_eco_earned: 0.0, upgrade_fund_paid: 0.0,
            is_recycled_device: is_recycled,
        }
    }

    pub fn apply(&mut self, base_credits: f64) -> EcoReward {
        let recycled_credits = base_credits * self.recycle_mult;
        let eco_bonus        = recycled_credits - base_credits;
        let upgrade_contrib  = recycled_credits * UPGRADE_FUND_RATE;
        let net_credits      = recycled_credits - upgrade_contrib;

        self.total_eco_earned   += eco_bonus;
        self.upgrade_fund_paid  += upgrade_contrib;

        EcoReward {
            base_credits,
            recycle_mult: self.recycle_mult,
            recycled_credits,
            eco_bonus,
            upgrade_fund_contribution: upgrade_contrib,
            net_credits,
            hw_age_label: self.hw_age.label().to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EcoReward {
    pub base_credits: f64,
    pub recycle_mult: f64,
    pub recycled_credits: f64,
    pub eco_bonus: f64,
    pub upgrade_fund_contribution: f64,
    pub net_credits: f64,
    pub hw_age_label: String,
}

// -----------------------------------------------------------------------------
// UpgradeFund — фонд апгрейда железа (дополнение к pools.rs)
// -----------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct UpgradeFund {
    pub balance: f64,
    pub total_contributed: f64,
    pub contributions: Vec<(String, f64)>,  // (node_id, amount)
    pub disbursements: Vec<(String, f64)>,  // (node_id, amount)
}

impl UpgradeFund {
    pub fn new() -> Self { Self::default() }

    pub fn contribute(&mut self, node_id: &str, amount: f64) {
        self.balance += amount;
        self.total_contributed += amount;
        self.contributions.push((node_id.to_string(), amount));
    }

    pub fn disburse(&mut self, node_id: &str, amount: f64) -> bool {
        if self.balance < amount { return false; }
        self.balance -= amount;
        self.disbursements.push((node_id.to_string(), amount));
        true
    }

    pub fn top_contributors(&self, n: usize) -> Vec<(&str, f64)> {
        let mut map: HashMap<&str, f64> = HashMap::new();
        for (node, amt) in &self.contributions {
            *map.entry(node.as_str()).or_insert(0.0) += amt;
        }
        let mut v: Vec<(&str, f64)> = map.into_iter().collect();
        v.sort_by(|a,b| b.1.partial_cmp(&a.1).unwrap());
        v.into_iter().take(n).collect()
    }

    pub fn stats(&self) -> String {
        format!("balance={:.2}💎  contributed={:.2}💎  disbursed={:.2}💎  nodes={}",
            self.balance, self.total_contributed,
            self.disbursements.iter().map(|(_,a)| a).sum::<f64>(),
            self.contributions.iter().map(|(n,_)| n.as_str())
                .collect::<std::collections::HashSet<_>>().len())
    }
}
