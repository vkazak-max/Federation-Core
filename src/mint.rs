// =============================================================================
// FEDERATION CORE — mint.rs
// PHASE 5 / STEP 4 — «Algorithmic Emission & Economic Equilibrium»
// =============================================================================
//
// Credits не рисуются — доказываются.
// Каждый токен = подтверждённый акт прорыва цензуры.
//
// Формула эмиссии:
//   mint_amount = BASE_REWARD * difficulty_mult * tactic_mult * halving_factor
//
// Halving: каждые HALVING_INTERVAL прорывов награда делится на 2
// Burn:    BURN_RATE от каждой рыночной комиссии уничтожается
// Supply:  MAX_SUPPLY — абсолютный потолок эмиссии
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BASE_REWARD: f64          = 10.0;
pub const MAX_SUPPLY: f64           = 21_000_000.0; // как Bitcoin
pub const HALVING_INTERVAL: u64     = 1_000_000;    // каждый 1M прорывов
pub const BURN_RATE: f64            = 0.30;          // 30% комиссии сгорает
pub const MIN_REWARD: f64           = 0.001;         // минимальная награда
pub const TREASURY_RATE: f64        = 0.10;          // 10% в казну DAO

// -----------------------------------------------------------------------------
// HalvingSchedule — расписание халвинга
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalvingSchedule {
    pub interval: u64,
    pub current_epoch: u32,
    pub next_halving_at: u64,
    pub current_multiplier: f64,
}

impl HalvingSchedule {
    pub fn new(interval: u64) -> Self {
        HalvingSchedule {
            interval,
            current_epoch: 0,
            next_halving_at: interval,
            current_multiplier: 1.0,
        }
    }

    /// Обновить халвинг по числу прорывов
    pub fn update(&mut self, total_bypasses: u64) -> bool {
        if total_bypasses >= self.next_halving_at {
            self.current_epoch += 1;
            self.next_halving_at += self.interval;
            self.current_multiplier /= 2.0;
            true // халвинг произошёл
        } else { false }
    }

    pub fn reward_factor(&self) -> f64 {
        self.current_multiplier.max(MIN_REWARD / BASE_REWARD)
    }

    pub fn blocks_to_next(&self, total: u64) -> u64 {
        self.next_halving_at.saturating_sub(total)
    }
}

// -----------------------------------------------------------------------------
// BurnLedger — учёт сожжённых токенов
// -----------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BurnLedger {
    pub total_burned: f64,
    pub burn_events: Vec<BurnEvent>,
    pub burns_by_reason: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnEvent {
    pub amount: f64,
    pub reason: String,
    pub timestamp: i64,
}

impl BurnLedger {
    pub fn burn(&mut self, amount: f64, reason: &str) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;
        self.total_burned += amount;
        *self.burns_by_reason.entry(reason.to_string()).or_insert(0.0) += amount;
        self.burn_events.push(BurnEvent {
            amount, reason: reason.to_string(), timestamp: now });
    }

    pub fn burn_rate_30d(&self) -> f64 {
        // Упрощённо — среднее по последним 30 событиям
        let recent: Vec<f64> = self.burn_events.iter()
            .rev().take(30).map(|e| e.amount).collect();
        if recent.is_empty() { return 0.0; }
        recent.iter().sum::<f64>() / recent.len() as f64
    }
}

// -----------------------------------------------------------------------------
// MintEvent — одна эмиссия
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintEvent {
    pub event_id: u64,
    pub node_id: String,
    pub region: String,
    pub tactic: String,
    pub gross_minted: f64,    // создано до вычетов
    pub burned: f64,          // сожжено сразу
    pub treasury: f64,        // в казну DAO
    pub net_to_node: f64,     // получает узел
    pub difficulty: f64,
    pub halving_epoch: u32,
    pub total_supply_after: f64,
}

// -----------------------------------------------------------------------------
// MintEngine — главный эмиссионный центр
// -----------------------------------------------------------------------------

pub struct MintEngine {
    pub total_supply: f64,
    pub total_bypasses: u64,
    pub halving: HalvingSchedule,
    pub burn_ledger: BurnLedger,
    pub treasury: f64,
    pub node_earnings: HashMap<String, f64>,
    pub mint_history: Vec<MintEvent>,
    pub event_counter: u64,
    pub is_exhausted: bool,    // достигнут MAX_SUPPLY
}

impl MintEngine {
    pub fn new() -> Self {
        MintEngine {
            total_supply: 0.0,
            total_bypasses: 0,
            halving: HalvingSchedule::new(HALVING_INTERVAL),
            burn_ledger: BurnLedger::default(),
            treasury: 0.0,
            node_earnings: HashMap::new(),
            mint_history: vec![],
            event_counter: 0,
            is_exhausted: false,
        }
    }

    /// Главная функция — минтить Credits за доказанный прорыв
    pub fn mint_for_bypass(&mut self, node_id: &str, region: &str,
                            tactic: &str, difficulty: f64) -> Option<MintEvent> {
        if self.is_exhausted { return None; }

        // Тактический множитель
        let tactic_mult = match tactic {
            "AikiReflection"   => 2.5,
            "CumulativeStrike" => 1.8,
            "StandoffDecoy"    => 1.3,
            "Hybrid"           => 2.0,
            _                  => 1.0,
        };

        // Сложность региона (1.0 + difficulty * 4.0)
        let diff_mult = 1.0 + difficulty * 4.0;

        // Халвинг фактор
        let halving_factor = self.halving.reward_factor();

        // Эмиссия
        let gross = BASE_REWARD * diff_mult * tactic_mult * halving_factor;

        // Проверяем потолок
        let remaining = MAX_SUPPLY - self.total_supply;
        let gross = gross.min(remaining);
        if gross < MIN_REWARD {
            self.is_exhausted = true;
            return None;
        }

        // Распределение: burn + treasury + node
        let burned  = gross * BURN_RATE;
        let treasury = gross * TREASURY_RATE;
        let net      = gross - burned - treasury;

        // Применяем
        self.total_supply += net + treasury; // burned не входит в supply
        self.treasury += treasury;
        self.burn_ledger.burn(burned, "mint_burn");
        *self.node_earnings.entry(node_id.to_string()).or_insert(0.0) += net;

        // Халвинг
        self.total_bypasses += 1;
        let _halving_triggered = self.halving.update(self.total_bypasses);

        self.event_counter += 1;
        let event = MintEvent {
            event_id: self.event_counter,
            node_id: node_id.to_string(),
            region: region.to_string(),
            tactic: tactic.to_string(),
            gross_minted: gross,
            burned, treasury,
            net_to_node: net,
            difficulty,
            halving_epoch: self.halving.current_epoch,
            total_supply_after: self.total_supply,
        };
        self.mint_history.push(event.clone());
        Some(event)
    }

    /// Сжечь рыночную комиссию (deflationary pressure)
    pub fn burn_market_fee(&mut self, fee: f64) -> f64 {
        let burn_amount = fee * BURN_RATE;
        self.burn_ledger.burn(burn_amount, "market_fee");
        burn_amount
    }

    /// Симуляция N прорывов — быстрый расчёт
    pub fn simulate_bypasses(&mut self, count: u64, node_id: &str,
                              region: &str, tactic: &str,
                              difficulty: f64) -> SimResult {
        let supply_before = self.total_supply;
        let _bypasses_before = self.total_bypasses;
        let mut minted = 0.0;
        let mut burned = 0.0;
        let mut halvings = 0;

        for _ in 0..count {
            if let Some(e) = self.mint_for_bypass(node_id, region, tactic, difficulty) {
                minted += e.gross_minted;
                burned += e.burned;
                if e.halving_epoch > self.halving.current_epoch
                    .saturating_sub(halvings as u32) {
                    halvings += 1;
                }
            } else { break; }
        }

        SimResult {
            bypasses: count,
            total_minted: minted,
            total_burned: burned,
            net_supply_added: self.total_supply - supply_before,
            halvings_triggered: halvings,
            avg_per_bypass: if count > 0 { minted / count as f64 } else { 0.0 },
            supply_after: self.total_supply,
            inflation_rate: (self.total_supply - supply_before)
                / supply_before.max(1.0) * 100.0,
        }
    }

    pub fn supply_stats(&self) -> SupplyStats {
        let top: Vec<(String, f64)> = {
            let mut v: Vec<_> = self.node_earnings.iter()
                .map(|(k, v)| (k.clone(), *v)).collect();
            v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            v.into_iter().take(5).collect()
        };

        SupplyStats {
            total_supply: self.total_supply,
            max_supply: MAX_SUPPLY,
            pct_issued: self.total_supply / MAX_SUPPLY * 100.0,
            total_burned: self.burn_ledger.total_burned,
            treasury: self.treasury,
            total_bypasses: self.total_bypasses,
            halving_epoch: self.halving.current_epoch,
            next_halving_in: self.halving.blocks_to_next(self.total_bypasses),
            top_earners: top,
        }
    }
}

impl Default for MintEngine { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct SimResult {
    pub bypasses: u64,
    pub total_minted: f64,
    pub total_burned: f64,
    pub net_supply_added: f64,
    pub halvings_triggered: usize,
    pub avg_per_bypass: f64,
    pub supply_after: f64,
    pub inflation_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SupplyStats {
    pub total_supply: f64,
    pub max_supply: f64,
    pub pct_issued: f64,
    pub total_burned: f64,
    pub treasury: f64,
    pub total_bypasses: u64,
    pub halving_epoch: u32,
    pub next_halving_in: u64,
    pub top_earners: Vec<(String, f64)>,
}

impl std::fmt::Display for SupplyStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bar_len = (self.pct_issued / 2.0) as usize;
        let bar = "█".repeat(bar_len) + &"░".repeat(50 - bar_len);
        write!(f,
            "╔══════════════════════════════════════════════════════╗\n\
             ║  FEDERATION MINT — SUPPLY STATS                      ║\n\
             ╠══════════════════════════════════════════════════════╣\n\
             ║  Supply:  {:>12.2} / {:>12.2} ({:.2}%)      ║\n\
             ║  [{}]  ║\n\
             ║  Сожжено: {:>12.2}  Казна: {:>10.2}          ║\n\
             ║  Прорывов:{:>12}  Халвинг эпоха: {:>4}         ║\n\
             ║  До след. халвинга: {:>10} прорывов             ║\n\
             ╚══════════════════════════════════════════════════════╝",
            self.total_supply, self.max_supply, self.pct_issued,
            bar,
            self.total_burned, self.treasury,
            self.total_bypasses, self.halving_epoch,
            self.next_halving_in,
        )
    }
}

// =============================================================================
// ADAPTIVE EMISSION — Phase 8 Patch
// IdeaLab влияет на параметры эмиссии через одобренные предложения
//
//   IdeaLabSignal  — сигнал от одобренной идеи
//   EmissionPolicy — текущая политика эмиссии
//   AdaptiveMint   — движок с динамическими параметрами
// =============================================================================

pub const POLICY_CHANGE_COOLDOWN: u64 = 100; // прорывов между изменениями
pub const MAX_BURN_RATE: f64           = 0.50;
pub const MIN_BURN_RATE: f64           = 0.10;
pub const MAX_TACTIC_MULT: f64         = 4.0;
pub const MIN_TACTIC_MULT: f64         = 0.5;

// -----------------------------------------------------------------------------
// IdeaLabSignal — одобренная идея меняет параметр эмиссии
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeaLabSignal {
    pub proposal_id: u64,
    pub title: String,
    pub domain: String,
    pub param: EmissionParam,
    pub delta: f64,        // изменение параметра
    pub ai_confidence: f64,// уверенность ИИ из SimReport
    pub approved_by: usize,// сколько сценариев одобрили
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EmissionParam {
    BurnRate,
    TacticMultiplier { tactic: String },
    DifficultyWeight,
    TreasuryRate,
    BaseReward,
}

impl EmissionParam {
    pub fn name(&self) -> String {
        match self {
            EmissionParam::BurnRate              => "BURN_RATE".into(),
            EmissionParam::TacticMultiplier { tactic } => format!("TACTIC_MULT[{}]", tactic),
            EmissionParam::DifficultyWeight      => "DIFF_WEIGHT".into(),
            EmissionParam::TreasuryRate          => "TREASURY_RATE".into(),
            EmissionParam::BaseReward            => "BASE_REWARD".into(),
        }
    }
}

// -----------------------------------------------------------------------------
// EmissionPolicy — текущая активная политика
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmissionPolicy {
    pub burn_rate: f64,
    pub treasury_rate: f64,
    pub base_reward: f64,
    pub diff_weight: f64,
    pub tactic_mults: HashMap<String, f64>,
    pub version: u32,
    pub last_changed_at: u64,   // total_bypasses при последнем изменении
    pub change_log: Vec<String>,
}

impl EmissionPolicy {
    pub fn default_policy() -> Self {
        let mut tactic_mults = HashMap::new();
        tactic_mults.insert("AikiReflection".into(),   2.5);
        tactic_mults.insert("CumulativeStrike".into(), 1.8);
        tactic_mults.insert("StandoffDecoy".into(),    1.3);
        tactic_mults.insert("Hybrid".into(),           2.0);
        tactic_mults.insert("Passive".into(),          1.0);
        EmissionPolicy {
            burn_rate: 0.30, treasury_rate: 0.10,
            base_reward: 1.0, diff_weight: 4.0,
            tactic_mults, version: 1,
            last_changed_at: 0, change_log: vec![],
        }
    }

    pub fn apply_signal(&mut self, signal: &IdeaLabSignal,
                         current_bypasses: u64) -> PolicyChangeResult {
        // Cooldown защита
        if current_bypasses - self.last_changed_at < POLICY_CHANGE_COOLDOWN {
            return PolicyChangeResult {
                applied: false, param: signal.param.name(),
                old_val: 0.0, new_val: 0.0,
                reason: format!("cooldown: ещё {} прорывов",
                    POLICY_CHANGE_COOLDOWN - (current_bypasses - self.last_changed_at)),
            };
        }

        // ИИ уверен достаточно?
        if signal.ai_confidence < 0.70 {
            return PolicyChangeResult {
                applied: false, param: signal.param.name(),
                old_val: 0.0, new_val: 0.0,
                reason: format!("низкая уверенность ИИ: {:.0}%", signal.ai_confidence*100.0),
            };
        }

        let (old_val, new_val) = match &signal.param {
            EmissionParam::BurnRate => {
                let old = self.burn_rate;
                let new = (old + signal.delta).clamp(MIN_BURN_RATE, MAX_BURN_RATE);
                self.burn_rate = new;
                (old, new)
            }
            EmissionParam::TreasuryRate => {
                let old = self.treasury_rate;
                let new = (old + signal.delta).clamp(0.05, 0.25);
                self.treasury_rate = new;
                (old, new)
            }
            EmissionParam::BaseReward => {
                let old = self.base_reward;
                let new = (old + signal.delta).clamp(0.1, 10.0);
                self.base_reward = new;
                (old, new)
            }
            EmissionParam::DifficultyWeight => {
                let old = self.diff_weight;
                let new = (old + signal.delta).clamp(1.0, 8.0);
                self.diff_weight = new;
                (old, new)
            }
            EmissionParam::TacticMultiplier { tactic } => {
                let old = *self.tactic_mults.get(tactic).unwrap_or(&1.0);
                let new = (old + signal.delta).clamp(MIN_TACTIC_MULT, MAX_TACTIC_MULT);
                self.tactic_mults.insert(tactic.clone(), new);
                (old, new)
            }
        };

        self.version += 1;
        self.last_changed_at = current_bypasses;
        self.change_log.push(format!("v{}: {} {:.3}→{:.3} (P{} conf={:.0}%)",
            self.version, signal.param.name(), old_val, new_val,
            signal.proposal_id, signal.ai_confidence*100.0));

        PolicyChangeResult {
            applied: true, param: signal.param.name(),
            old_val, new_val, reason: "OK".into(),
        }
    }

    pub fn tactic_mult(&self, tactic: &str) -> f64 {
        *self.tactic_mults.get(tactic).unwrap_or(&1.0)
    }
}

#[derive(Debug)]
pub struct PolicyChangeResult {
    pub applied: bool,
    pub param: String,
    pub old_val: f64,
    pub new_val: f64,
    pub reason: String,
}

// -----------------------------------------------------------------------------
// AdaptiveMintEngine — движок с живой политикой
// -----------------------------------------------------------------------------

pub struct AdaptiveMintEngine {
    pub policy: EmissionPolicy,
    pub total_bypasses: u64,
    pub total_minted: f64,
    pub pending_signals: Vec<IdeaLabSignal>,
    pub applied_signals: Vec<IdeaLabSignal>,
}

impl AdaptiveMintEngine {
    pub fn new() -> Self {
        AdaptiveMintEngine {
            policy: EmissionPolicy::default_policy(),
            total_bypasses: 0, total_minted: 0.0,
            pending_signals: vec![], applied_signals: vec![],
        }
    }

    pub fn propose_change(&mut self, signal: IdeaLabSignal) {
        self.pending_signals.push(signal);
    }

    pub fn process_signals(&mut self) -> Vec<PolicyChangeResult> {
        let signals = std::mem::take(&mut self.pending_signals);
        let bypasses = self.total_bypasses;
        signals.into_iter().map(|sig| {
            let r = self.policy.apply_signal(&sig, bypasses);
            if r.applied { self.applied_signals.push(sig); }
            r
        }).collect()
    }

    pub fn mint(&mut self, tactic: &str, difficulty: f64) -> f64 {
        let tactic_mult  = self.policy.tactic_mult(tactic);
        let diff_mult    = 1.0 + difficulty * self.policy.diff_weight;
        let gross        = self.policy.base_reward * diff_mult * tactic_mult;
        let burned       = gross * self.policy.burn_rate;
        let treasury     = gross * self.policy.treasury_rate;
        let net          = gross - burned - treasury;
        self.total_bypasses += 1;
        self.total_minted   += net;
        net
    }
}

impl Default for AdaptiveMintEngine { fn default() -> Self { Self::new() } }
