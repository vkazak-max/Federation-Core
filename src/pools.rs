// =============================================================================
// FEDERATION CORE — pools.rs
// PHASE 5 / STEP 7 — «Swarm Treasury & Social Guarantees»
// =============================================================================
//
// Казначейство Роя — три пула социальных гарантий:
//
//   Insurance Pool — компенсация за блокировку (потеря streak)
//   Health Pool    — накопление на апгрейд железа
//   Education Pool — аренда Sentinel для обучения Mobile нейросетей
//
// Пополнение: TREASURY_RATE от каждого mint.rs события
// Управление: DAO голосование для выплат выше LARGE_PAYOUT_THRESHOLD
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const INSURANCE_RATE: f64         = 0.40; // 40% казны → страховка
pub const HEALTH_RATE: f64            = 0.35; // 35% казны → здоровье
pub const EDUCATION_RATE: f64         = 0.25; // 25% казны → обучение
pub const INSURANCE_STREAK_MULT: f64  = 2.5;  // компенсация = streak * mult
pub const HEALTH_UPGRADE_MIN: f64     = 50.0; // минимум для апгрейда
pub const EDUCATION_HOUR_RATE: f64    = 5.0;  // credits/час аренды Sentinel
pub const LARGE_PAYOUT_THRESHOLD: f64 = 500.0;// выше — нужно DAO
pub const MAX_INSURANCE_PER_EVENT: f64= 200.0;// потолок выплаты

// -----------------------------------------------------------------------------
// PoolType — тип пула
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    Insurance,
    Health,
    Education,
}

impl PoolType {
    pub fn name(&self) -> &str {
        match self {
            PoolType::Insurance  => "🛡️  Insurance",
            PoolType::Health     => "💊 Health",
            PoolType::Education  => "🎓 Education",
        }
    }
    pub fn allocation_rate(&self) -> f64 {
        match self {
            PoolType::Insurance  => INSURANCE_RATE,
            PoolType::Health     => HEALTH_RATE,
            PoolType::Education  => EDUCATION_RATE,
        }
    }
}

// -----------------------------------------------------------------------------
// InsuranceClaim — заявка на страховую выплату
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsuranceClaim {
    pub claim_id: u64,
    pub node_id: String,
    pub reason: InsuranceReason,
    pub streak_lost: u32,
    pub credits_lost: f64,
    pub requested: f64,
    pub approved: f64,
    pub status: ClaimStatus,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsuranceReason {
    CensorBlock    { region: String, block_rate: f64 },
    HardwareFailure { component: String },
    NetworkCut     { duration_hours: u32 },
    EthicsViolation,  // не выплачивается
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClaimStatus {
    Pending,
    Approved,
    Rejected,
    RequiresDao,
    Paid,
}

impl InsuranceClaim {
    pub fn compute_payout(streak: u32, credits_lost: f64,
                          pool_balance: f64) -> f64 {
        let streak_bonus = streak as f64 * INSURANCE_STREAK_MULT;
        let raw = (credits_lost * 0.7 + streak_bonus).min(MAX_INSURANCE_PER_EVENT);
        raw.min(pool_balance * 0.1) // не более 10% пула за раз
    }
}

// -----------------------------------------------------------------------------
// HealthRequest — запрос на апгрейд железа
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRequest {
    pub request_id: u64,
    pub node_id: String,
    pub component: String,
    pub description: String,
    pub cost_estimate: f64,
    pub approved_amount: f64,
    pub status: ClaimStatus,
    pub hardware_score_before: f64,
    pub hardware_score_after: f64,  // ожидаемый после апгрейда
    pub timestamp: i64,
}

impl HealthRequest {
    pub fn roi(&self) -> f64 {
        // ROI апгрейда — насколько вырастет производительность
        if self.cost_estimate == 0.0 { return 0.0; }
        (self.hardware_score_after - self.hardware_score_before)
            / self.cost_estimate * 100.0
    }
}

// -----------------------------------------------------------------------------
// EducationSession — сессия обучения на Sentinel
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationSession {
    pub session_id: u64,
    pub student_node: String,  // Mobile/Ghost узел
    pub sentinel_node: String, // Sentinel провайдер
    pub duration_hours: f64,
    pub cost: f64,
    pub accuracy_before: f64,
    pub accuracy_after: f64,
    pub modules_trained: Vec<String>,
    pub status: SessionStatus,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Scheduled, Running, Completed, Failed,
}

impl EducationSession {
    pub fn accuracy_gain(&self) -> f64 {
        self.accuracy_after - self.accuracy_before
    }
    pub fn cost_per_accuracy_point(&self) -> f64 {
        if self.accuracy_gain() == 0.0 { return f64::MAX; }
        self.cost / self.accuracy_gain()
    }
}

// -----------------------------------------------------------------------------
// Pool — один пул с балансом и историей
// -----------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct Pool {
    pub pool_type: PoolType,
    pub balance: f64,
    pub total_received: f64,
    pub total_paid: f64,
    pub total_claims: u64,
    pub rejected_claims: u64,
}

impl Pool {
    pub fn new(pool_type: PoolType) -> Self {
        Pool { pool_type, balance: 0.0, total_received: 0.0,
               total_paid: 0.0, total_claims: 0, rejected_claims: 0 }
    }

    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
        self.total_received += amount;
    }

    pub fn withdraw(&mut self, amount: f64) -> bool {
        if amount > self.balance { return false; }
        self.balance -= amount;
        self.total_paid += amount;
        self.total_claims += 1;
        true
    }

    pub fn solvency_ratio(&self) -> f64 {
        if self.total_received == 0.0 { return 1.0; }
        1.0 - (self.total_paid / self.total_received)
    }
}

// -----------------------------------------------------------------------------
// SwarmTreasury — главное казначейство
// -----------------------------------------------------------------------------

pub struct SwarmTreasury {
    pub insurance: Pool,
    pub health: Pool,
    pub education: Pool,
    pub insurance_claims: Vec<InsuranceClaim>,
    pub health_requests: Vec<HealthRequest>,
    pub education_sessions: Vec<EducationSession>,
    pub node_insurance_history: HashMap<String, Vec<u64>>, // node → claim_ids
    pub counter: u64,
}

impl SwarmTreasury {
    pub fn new() -> Self {
        SwarmTreasury {
            insurance: Pool::new(PoolType::Insurance),
            health: Pool::new(PoolType::Health),
            education: Pool::new(PoolType::Education),
            insurance_claims: vec![],
            health_requests: vec![],
            education_sessions: vec![],
            node_insurance_history: HashMap::new(),
            counter: 0,
        }
    }

    fn now() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64
    }

    /// Пополнить казну из mint события
    pub fn deposit_from_mint(&mut self, mint_amount: f64) {
        self.insurance.deposit(mint_amount * INSURANCE_RATE);
        self.health.deposit(mint_amount * HEALTH_RATE);
        self.education.deposit(mint_amount * EDUCATION_RATE);
    }

    /// Подать заявку на страховку
    pub fn file_insurance_claim(&mut self, node_id: &str,
        reason: InsuranceReason, streak_lost: u32,
        credits_lost: f64) -> InsuranceClaim {

        self.counter += 1;
        let payout = InsuranceClaim::compute_payout(
            streak_lost, credits_lost, self.insurance.balance);

        // Этическое нарушение — отказ
        let (approved, status) = if matches!(reason, InsuranceReason::EthicsViolation) {
            (0.0, ClaimStatus::Rejected)
        } else if payout > LARGE_PAYOUT_THRESHOLD {
            (payout, ClaimStatus::RequiresDao)
        } else {
            (payout, ClaimStatus::Approved)
        };

        let claim = InsuranceClaim {
            claim_id: self.counter,
            node_id: node_id.to_string(),
            reason, streak_lost, credits_lost,
            requested: payout, approved,
            status: status.clone(), timestamp: Self::now(),
        };

        if status == ClaimStatus::Approved {
            self.insurance.withdraw(approved);
        }

        self.node_insurance_history
            .entry(node_id.to_string()).or_default()
            .push(self.counter);
        self.insurance_claims.push(claim.clone());
        claim
    }

    /// Запрос на апгрейд железа
    pub fn request_health_upgrade(&mut self, node_id: &str,
        component: &str, description: &str, cost: f64,
        score_before: f64, score_after: f64) -> HealthRequest {

        self.counter += 1;
        let (approved, status) = if cost > self.health.balance * 0.2 {
            (0.0, ClaimStatus::Rejected) // не более 20% пула
        } else if cost < HEALTH_UPGRADE_MIN {
            (0.0, ClaimStatus::Rejected) // слишком мало
        } else if cost > LARGE_PAYOUT_THRESHOLD {
            (cost, ClaimStatus::RequiresDao)
        } else {
            (cost, ClaimStatus::Approved)
        };

        if status == ClaimStatus::Approved {
            self.health.withdraw(approved);
        }

        let req = HealthRequest {
            request_id: self.counter,
            node_id: node_id.to_string(),
            component: component.to_string(),
            description: description.to_string(),
            cost_estimate: cost, approved_amount: approved,
            status, hardware_score_before: score_before,
            hardware_score_after: score_after,
            timestamp: Self::now(),
        };
        self.health_requests.push(req.clone());
        req
    }

    /// Запись образовательной сессии
    pub fn schedule_education(&mut self, student: &str,
        sentinel: &str, hours: f64, modules: Vec<String>,
        acc_before: f64, acc_after: f64) -> EducationSession {

        self.counter += 1;
        let cost = hours * EDUCATION_HOUR_RATE;
        let (status, paid_cost) = if cost <= self.education.balance {
            self.education.withdraw(cost);
            (SessionStatus::Completed, cost)
        } else {
            (SessionStatus::Failed, 0.0)
        };

        let session = EducationSession {
            session_id: self.counter,
            student_node: student.to_string(),
            sentinel_node: sentinel.to_string(),
            duration_hours: hours, cost: paid_cost,
            accuracy_before: acc_before, accuracy_after: acc_after,
            modules_trained: modules, status,
            timestamp: Self::now(),
        };
        self.education_sessions.push(session.clone());
        session
    }

    pub fn total_balance(&self) -> f64 {
        self.insurance.balance + self.health.balance + self.education.balance
    }

    pub fn treasury_stats(&self) -> TreasuryStats {
        let edu_gain: f64 = self.education_sessions.iter()
            .filter(|s| s.status == SessionStatus::Completed)
            .map(|s| s.accuracy_gain()).sum();
        let approved_claims = self.insurance_claims.iter()
            .filter(|c| c.status == ClaimStatus::Approved).count();
        let approved_upgrades = self.health_requests.iter()
            .filter(|r| r.status == ClaimStatus::Approved).count();

        TreasuryStats {
            insurance_balance: self.insurance.balance,
            health_balance: self.health.balance,
            education_balance: self.education.balance,
            total_balance: self.total_balance(),
            insurance_paid: self.insurance.total_paid,
            health_paid: self.health.total_paid,
            education_paid: self.education.total_paid,
            approved_claims, approved_upgrades,
            education_sessions: self.education_sessions.len(),
            total_accuracy_gained: edu_gain,
            insurance_solvency: self.insurance.solvency_ratio(),
        }
    }
}

impl Default for SwarmTreasury { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct TreasuryStats {
    pub insurance_balance: f64,
    pub health_balance: f64,
    pub education_balance: f64,
    pub total_balance: f64,
    pub insurance_paid: f64,
    pub health_paid: f64,
    pub education_paid: f64,
    pub approved_claims: usize,
    pub approved_upgrades: usize,
    pub education_sessions: usize,
    pub total_accuracy_gained: f64,
    pub insurance_solvency: f64,
}

impl std::fmt::Display for TreasuryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════════════╗\n\
             ║  SWARM TREASURY — STATS                              ║\n\
             ╠══════════════════════════════════════════════════════╣\n\
             ║  🛡️  Insurance: {:>8.2}💎  выплачено: {:>8.2}💎   ║\n\
             ║  💊 Health:     {:>8.2}💎  выплачено: {:>8.2}💎   ║\n\
             ║  🎓 Education:  {:>8.2}💎  выплачено: {:>8.2}💎   ║\n\
             ╠══════════════════════════════════════════════════════╣\n\
             ║  Итого:         {:>8.2}💎  Платёжесп.: {:>5.1}%    ║\n\
             ║  Страх.выплат:{:>3}  Апгрейдов:{:>3}  Сессий:{:>3}  ║\n\
             ║  Прирост точности нейросетей: {:>6.3}               ║\n\
             ╚══════════════════════════════════════════════════════╝",
            self.insurance_balance, self.insurance_paid,
            self.health_balance, self.health_paid,
            self.education_balance, self.education_paid,
            self.total_balance, self.insurance_solvency * 100.0,
            self.approved_claims, self.approved_upgrades,
            self.education_sessions, self.total_accuracy_gained,
        )
    }
}
