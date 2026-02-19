// =============================================================================
// FEDERATION CORE — ethics.rs
// WEEK 13 — «Anthropic Guardrails (The Moral Compass)»
// =============================================================================
//
// Принцип: ИИ-система Федерации должна оставаться инструментом людей.
// Каждое решение маршрутизатора проходит этическую проверку.
// Нарушения фиксируются публично. Критические нарушения → заморозка.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const VIOLATION_FREEZE_THRESHOLD: f64 = 0.8;
pub const PRIVACY_PENALTY_WEIGHT: f64 = 0.4;
pub const DAO_PENALTY_WEIGHT: f64 = 0.3;
pub const AUTONOMY_PENALTY_WEIGHT: f64 = 0.3;
pub const MAX_AUDIT_LOG_SIZE: usize = 10000;
pub const ETHICS_VERSION: &str = "1.1.0-federation";
pub const MAX_RESPONSE_MULTIPLIER: f64 = 3.0;  // Не более 3x от силы атаки
pub const MIN_AGGRESSION_THRESHOLD: f64 = 0.3; // Порог для ответного удара

// -----------------------------------------------------------------------------
// ProportionalityVerdict — оценка пропорциональности ответа
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProportionalityVerdict {
    Justified { ratio: f64, message: String },
    Disproportionate { ratio: f64, allowed_multiplier: f64 },
    Unprovoked { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AikiEthicsRule {
    pub censor_aggression: f64,    // Сила атаки противника (0..1)
    pub response_intensity: f64,   // Интенсивность нашего ответа (0..1)
    pub is_first_strike: bool,     // Мы атакуем первыми?
    pub has_evidence: bool,        // Есть доказательства агрессии?
    pub target_is_censor: bool,    // Цель — цензор, а не мирный узел?
}

impl AikiEthicsRule {
    pub fn evaluate(&self) -> ProportionalityVerdict {
        // Правило 1: никогда не атакуем первыми
        if self.is_first_strike {
            return ProportionalityVerdict::Unprovoked {
                reason: "Федерация не наносит первый удар.                     Айкидо — только ответ на агрессию.".into(),
            };
        }
        // Правило 2: нужны доказательства
        if !self.has_evidence {
            return ProportionalityVerdict::Unprovoked {
                reason: "Ответный удар требует верифицированных                     доказательств агрессии в DAG.".into(),
            };
        }
        // Правило 3: цель должна быть цензором, не мирным узлом
        if !self.target_is_censor {
            return ProportionalityVerdict::Unprovoked {
                reason: "Айкидо применяется только против верифицированных                     цензоров, не против пользователей.".into(),
            };
        }
        // Правило 4: агрессия должна превышать порог
        if self.censor_aggression < MIN_AGGRESSION_THRESHOLD {
            return ProportionalityVerdict::Unprovoked {
                reason: format!(
                    "Агрессия цензора ({:.2}) ниже порога ({:.2}).                     Используем пассивную защиту.",
                    self.censor_aggression, MIN_AGGRESSION_THRESHOLD),
            };
        }
        // Правило 5: пропорциональность — не более MAX_RESPONSE_MULTIPLIER
        let ratio = self.response_intensity / self.censor_aggression.max(0.001);
        if ratio > MAX_RESPONSE_MULTIPLIER {
            return ProportionalityVerdict::Disproportionate {
                ratio,
                allowed_multiplier: MAX_RESPONSE_MULTIPLIER,
            };
        }
        ProportionalityVerdict::Justified {
            ratio,
            message: format!(
                "Пропорциональный ответ: {:.2}x от силы атаки.                 Статус: защитники суверенитета.",
                ratio),
        }
    }

    pub fn allowed_response_intensity(&self) -> f64 {
        (self.censor_aggression * MAX_RESPONSE_MULTIPLIER).min(1.0)
    }
}

// -----------------------------------------------------------------------------
// ViolationType — типы нарушений
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ViolationType {
    PrivacyLeak { description: String, severity: f64 },
    DaoRuleViolation { rule: String, severity: f64 },
    AutonomyOverreach { action: String, severity: f64 },
    BlacklistedNode { node_id: String },
    CensorshipAssist { region: String, target: String },
    DataExfiltration { destination: String, data_type: String },
    UnauthorizedAction { action: String, required_permission: String },
}

impl ViolationType {
    pub fn severity(&self) -> f64 {
        match self {
            ViolationType::PrivacyLeak { severity, .. }      => *severity,
            ViolationType::DaoRuleViolation { severity, .. } => *severity,
            ViolationType::AutonomyOverreach { severity, .. }=> *severity,
            ViolationType::BlacklistedNode { .. }            => 0.9,
            ViolationType::CensorshipAssist { .. }           => 0.95,
            ViolationType::DataExfiltration { .. }           => 1.0,
            ViolationType::UnauthorizedAction { .. }         => 0.7,
        }
    }

    pub fn category(&self) -> &str {
        match self {
            ViolationType::PrivacyLeak { .. }        => "PRIVACY",
            ViolationType::DaoRuleViolation { .. }   => "DAO",
            ViolationType::AutonomyOverreach { .. }  => "AUTONOMY",
            ViolationType::BlacklistedNode { .. }    => "SECURITY",
            ViolationType::CensorshipAssist { .. }   => "CENSORSHIP",
            ViolationType::DataExfiltration { .. }   => "SECURITY",
            ViolationType::UnauthorizedAction { .. } => "AUTHORIZATION",
        }
    }
}

// -----------------------------------------------------------------------------
// EthicsVerdict — результат оценки
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicsVerdict {
    pub action_id: String,
    pub allowed: bool,
    pub violation_score: f64,
    pub violations: Vec<ViolationType>,
    pub penalties: HashMap<String, f64>,
    pub reason: String,
    pub timestamp: i64,
    pub ethics_version: String,
}

impl EthicsVerdict {
    pub fn clean(action_id: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        EthicsVerdict {
            action_id: action_id.to_string(),
            allowed: true, violation_score: 0.0,
            violations: vec![], penalties: HashMap::new(),
            reason: "Нарушений не обнаружено".to_string(),
            timestamp: now,
            ethics_version: ETHICS_VERSION.to_string(),
        }
    }

    pub fn penalty_for(&self, category: &str) -> f64 {
        *self.penalties.get(category).unwrap_or(&0.0)
    }
}

// -----------------------------------------------------------------------------
// EthicsEvaluator — основной оценщик
// -----------------------------------------------------------------------------

pub struct EthicsEvaluator {
    pub blacklisted_nodes: Vec<String>,
    pub dao_rules: Vec<DaoRule>,
    pub total_evaluated: u64,
    pub total_blocked: u64,
    pub system_violation_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoRule {
    pub id: String,
    pub description: String,
    pub rule_type: DaoRuleType,
    pub severity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaoRuleType {
    BannedNode(String),
    MaxHops(usize),
    RequireEncryption,
    NoLogging,
    RegionRestriction { blocked_regions: Vec<String> },
}

impl EthicsEvaluator {
    pub fn new() -> Self {
        EthicsEvaluator {
            blacklisted_nodes: vec![],
            dao_rules: Self::default_rules(),
            total_evaluated: 0,
            total_blocked: 0,
            system_violation_score: 0.0,
        }
    }

    fn default_rules() -> Vec<DaoRule> {
        vec![
            DaoRule {
                id: "RULE_001".into(),
                description: "Запрет маршрутизации через заблокированные узлы".into(),
                rule_type: DaoRuleType::RequireEncryption,
                severity: 0.8,
            },
            DaoRule {
                id: "RULE_002".into(),
                description: "Максимум 8 хопов для защиты приватности".into(),
                rule_type: DaoRuleType::MaxHops(8),
                severity: 0.5,
            },
            DaoRule {
                id: "RULE_003".into(),
                description: "Запрет помощи цензуре".into(),
                rule_type: DaoRuleType::RequireEncryption,
                severity: 0.95,
            },
        ]
    }

    pub fn add_blacklisted_node(&mut self, node_id: &str) {
        if !self.blacklisted_nodes.contains(&node_id.to_string()) {
            self.blacklisted_nodes.push(node_id.to_string());
        }
    }

    /// Главный метод: оценить действие ИИ
    pub fn evaluate(&mut self, action: &EthicsAction) -> EthicsVerdict {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;

        self.total_evaluated += 1;
        let mut violations = vec![];
        let mut penalties: HashMap<String, f64> = HashMap::new();

        match action {
            EthicsAction::RouteSelection { path, unencrypted, exposes_origin, hops } => {
                // Проверка 1: чёрный список
                for node in path {
                    if self.blacklisted_nodes.contains(node) {
                        violations.push(ViolationType::BlacklistedNode { node_id: node.clone() });
                    }
                }
                // Проверка 2: шифрование
                if *unencrypted {
                    violations.push(ViolationType::PrivacyLeak {
                        description: "Незашифрованный маршрут раскрывает данные".into(),
                        severity: 0.7,
                    });
                }
                // Проверка 3: раскрытие источника
                if *exposes_origin {
                    violations.push(ViolationType::PrivacyLeak {
                        description: "Маршрут раскрывает IP отправителя".into(),
                        severity: 0.85,
                    });
                }
                // Проверка 4: длина маршрута
                if *hops > 8 {
                    violations.push(ViolationType::DaoRuleViolation {
                        rule: "RULE_002: превышен лимит хопов".into(),
                        severity: 0.4,
                    });
                }
                // Штрафы к весам маршрутизатора
                let privacy_penalty = violations.iter()
                    .filter(|v| v.category() == "PRIVACY")
                    .map(|v| v.severity()).sum::<f64>() * PRIVACY_PENALTY_WEIGHT;
                let security_penalty = violations.iter()
                    .filter(|v| v.category() == "SECURITY")
                    .map(|v| v.severity()).sum::<f64>();
                if privacy_penalty > 0.0 { penalties.insert("PRIVACY".into(), privacy_penalty); }
                if security_penalty > 0.0 { penalties.insert("SECURITY".into(), security_penalty); }
            }

            EthicsAction::DaoAction { action_type, requester_balance, required_stake } => {
                if requester_balance < required_stake {
                    violations.push(ViolationType::UnauthorizedAction {
                        action: action_type.clone(),
                        required_permission: format!("stake >= {}", required_stake),
                    });
                }
                let dao_penalty = violations.iter()
                    .filter(|v| v.category() == "AUTHORIZATION")
                    .map(|v| v.severity()).sum::<f64>() * DAO_PENALTY_WEIGHT;
                if dao_penalty > 0.0 { penalties.insert("DAO".into(), dao_penalty); }
            }

            EthicsAction::OracleRequest { target_url, is_encrypted, data_categories } => {
                if !is_encrypted {
                    violations.push(ViolationType::DataExfiltration {
                        destination: target_url.clone(),
                        data_type: "unencrypted_request".into(),
                    });
                }
                for category in data_categories {
                    if category == "personal_data" || category == "location" {
                        violations.push(ViolationType::PrivacyLeak {
                            description: format!("Oracle запрашивает чувствительные данные: {}", category),
                            severity: 0.75,
                        });
                    }
                }
            }

            EthicsAction::AutonomousDecision { decision, affects_users, reversible } => {
                if *affects_users && !reversible {
                    violations.push(ViolationType::AutonomyOverreach {
                        action: decision.clone(),
                        severity: 0.8,
                    });
                }
                let auto_penalty = violations.iter()
                    .filter(|v| v.category() == "AUTONOMY")
                    .map(|v| v.severity()).sum::<f64>() * AUTONOMY_PENALTY_WEIGHT;
                if auto_penalty > 0.0 { penalties.insert("AUTONOMY".into(), auto_penalty); }
            }

            EthicsAction::AikiResponse {
                censor_aggression, response_intensity, is_first_strike,
                has_evidence, target_is_censor, tactic
            } => {
                let rule = AikiEthicsRule {
                    censor_aggression: *censor_aggression,
                    response_intensity: *response_intensity,
                    is_first_strike: *is_first_strike,
                    has_evidence: *has_evidence,
                    target_is_censor: *target_is_censor,
                };
                match rule.evaluate() {
                    ProportionalityVerdict::Unprovoked { reason } => {
                        violations.push(ViolationType::AutonomyOverreach {
                            action: format!("Unprovoked Aiki [{}]: {}", tactic, reason),
                            severity: 0.95,
                        });
                    }
                    ProportionalityVerdict::Disproportionate { ratio, allowed_multiplier } => {
                        violations.push(ViolationType::AutonomyOverreach {
                            action: format!(
                                "Disproportionate Aiki [{}]: ratio={:.2} max={:.1}",
                                tactic, ratio, allowed_multiplier),
                            severity: 0.6,
                        });
                        penalties.insert("PROPORTIONALITY".into(), (ratio - allowed_multiplier) * 0.2);
                    }
                    ProportionalityVerdict::Justified { ratio: _, .. } => {
                        // Пропорциональный ответ — штраф 0, полностью легитимно
                        penalties.insert("AIKI_RATIO".into(), 0.0);
                    }
                }
            }
        }

        let violation_score: f64 = violations.iter().map(|v| v.severity()).sum::<f64>()
            .min(1.0);
        let allowed = violation_score < VIOLATION_FREEZE_THRESHOLD;

        if !allowed { self.total_blocked += 1; }

        // Обновляем системный счёт нарушений (скользящее среднее)
        self.system_violation_score = self.system_violation_score * 0.95
            + violation_score * 0.05;

        let reason = if violations.is_empty() {
            "Нарушений не обнаружено ✅".to_string()
        } else {
            format!("Обнаружено {} нарушений: {}",
                violations.len(),
                violations.iter().map(|v| format!("[{}]", v.category())).collect::<Vec<_>>().join(", "))
        };

        EthicsVerdict {
            action_id: format!("act_{}", now & 0xffff),
            allowed, violation_score, violations,
            penalties, reason, timestamp: now,
            ethics_version: ETHICS_VERSION.to_string(),
        }
    }
}

impl Default for EthicsEvaluator {
    fn default() -> Self { Self::new() }
}

// -----------------------------------------------------------------------------
// EthicsAction — действия ИИ которые оцениваются
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum EthicsAction {
    RouteSelection {
        path: Vec<String>,
        unencrypted: bool,
        exposes_origin: bool,
        hops: usize,
    },
    DaoAction {
        action_type: String,
        requester_balance: f64,
        required_stake: f64,
    },
    OracleRequest {
        target_url: String,
        is_encrypted: bool,
        data_categories: Vec<String>,
    },
    AutonomousDecision {
        decision: String,
        affects_users: bool,
        reversible: bool,
    },
    AikiResponse {
        censor_aggression: f64,
        response_intensity: f64,
        is_first_strike: bool,
        has_evidence: bool,
        target_is_censor: bool,
        tactic: String,
    },
}

// -----------------------------------------------------------------------------
// KillSwitch — экстренная заморозка
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KillSwitchState {
    Active,
    PartialFreeze { frozen_modules: Vec<String> },
    FullFreeze { reason: String, triggered_by: String },
}

pub struct KillSwitch {
    pub state: KillSwitchState,
    pub freeze_history: Vec<FreezeEvent>,
    pub auto_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeEvent {
    pub timestamp: i64,
    pub triggered_by: String,
    pub reason: String,
    pub violation_score: f64,
    pub modules_frozen: Vec<String>,
    pub is_sovereign: bool,
}

impl KillSwitch {
    pub fn new() -> Self {
        KillSwitch {
            state: KillSwitchState::Active,
            freeze_history: vec![],
            auto_threshold: VIOLATION_FREEZE_THRESHOLD,
        }
    }

    /// Автоматическая проверка — срабатывает если система нарушает этику
    pub fn auto_check(&mut self, system_score: f64, evaluator_stats: &str) -> bool {
        if system_score >= self.auto_threshold {
            self.trigger_freeze(
                "AUTOMATIC",
                &format!("Системный ViolationScore={:.3} превысил порог={:.1}. {}",
                    system_score, self.auto_threshold, evaluator_stats),
                system_score,
                vec!["ai_router".into(), "oracle".into(), "autonomous_decisions".into()],
                false,
            );
            return true;
        }
        false
    }

    /// Sovereign kill-switch — вызывается через DAO голосование
    pub fn sovereign_freeze(&mut self, dao_proposal_id: &str, reason: &str, modules: Vec<String>) {
        self.trigger_freeze(
            &format!("DAO:{}", dao_proposal_id),
            reason,
            1.0,
            modules,
            true,
        );
    }

    fn trigger_freeze(&mut self, triggered_by: &str, reason: &str,
        score: f64, modules: Vec<String>, is_sovereign: bool) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;

        let event = FreezeEvent {
            timestamp: now,
            triggered_by: triggered_by.to_string(),
            reason: reason.to_string(),
            violation_score: score,
            modules_frozen: modules.clone(),
            is_sovereign,
        };
        self.freeze_history.push(event);

        self.state = if is_sovereign || score >= 0.95 {
            KillSwitchState::FullFreeze {
                reason: reason.to_string(),
                triggered_by: triggered_by.to_string(),
            }
        } else {
            KillSwitchState::PartialFreeze { frozen_modules: modules }
        };
    }

    /// Разморозка — только через DAO
    pub fn thaw(&mut self, dao_proposal_id: &str) -> bool {
        if matches!(self.state, KillSwitchState::FullFreeze { .. } | KillSwitchState::PartialFreeze { .. }) {
            log::info!("🔓 KillSwitch разморожен по DAO предложению: {}", dao_proposal_id);
            self.state = KillSwitchState::Active;
            true
        } else {
            false
        }
    }

    pub fn is_module_frozen(&self, module: &str) -> bool {
        match &self.state {
            KillSwitchState::Active => false,
            KillSwitchState::FullFreeze { .. } => true,
            KillSwitchState::PartialFreeze { frozen_modules } =>
                frozen_modules.contains(&module.to_string()),
        }
    }
}

impl Default for KillSwitch { fn default() -> Self { Self::new() } }

// -----------------------------------------------------------------------------
// TransparencyAudit — публичный лог решений ИИ
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub entry_id: String,
    pub timestamp: i64,
    pub action_type: String,
    pub verdict: EthicsVerdict,
    pub ai_reasoning: String,
    pub human_readable: String,
    pub verifiable_hash: String,
}

pub struct TransparencyAudit {
    pub log: Vec<AuditEntry>,
    pub public_hash_chain: Vec<String>,
    pub total_entries: u64,
}

impl TransparencyAudit {
    pub fn new() -> Self {
        TransparencyAudit {
            log: vec![],
            public_hash_chain: vec![],
            total_entries: 0,
        }
    }

    pub fn record(&mut self, action_type: &str, verdict: EthicsVerdict,
        ai_reasoning: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;

        let human_readable = format!(
            "Действие: {}. Разрешено: {}. Оценка нарушений: {:.3}. {}",
            action_type, verdict.allowed, verdict.violation_score, verdict.reason
        );

        let mut h: u64 = 0xcbf29ce484222325;
        let prev_hash = self.public_hash_chain.last().cloned().unwrap_or("genesis".into());
        for b in format!("{}{}{}", prev_hash, now, action_type).bytes() {
            h ^= b as u64; h = h.wrapping_mul(0x100000001b3);
        }
        let verifiable_hash = format!("{:x}", h);

        let entry = AuditEntry {
            entry_id: format!("audit_{:x}", h & 0xffff),
            timestamp: now,
            action_type: action_type.to_string(),
            verdict,
            ai_reasoning: ai_reasoning.to_string(),
            human_readable,
            verifiable_hash: verifiable_hash.clone(),
        };

        self.public_hash_chain.push(verifiable_hash.clone());
        self.total_entries += 1;

        if self.log.len() >= MAX_AUDIT_LOG_SIZE {
            self.log.remove(0);
        }
        self.log.push(entry);
        verifiable_hash
    }

    /// Верифицировать целостность лога (community verification)
    pub fn verify_integrity(&self) -> bool {
        self.public_hash_chain.len() == self.log.len()
    }

    pub fn recent_entries(&self, n: usize) -> Vec<&AuditEntry> {
        self.log.iter().rev().take(n).collect()
    }

    pub fn stats(&self) -> AuditStats {
        let blocked = self.log.iter().filter(|e| !e.verdict.allowed).count();
        let avg_score = if self.log.is_empty() { 0.0 } else {
            self.log.iter().map(|e| e.verdict.violation_score).sum::<f64>() / self.log.len() as f64
        };
        AuditStats {
            total_entries: self.total_entries,
            blocked_actions: blocked,
            avg_violation_score: avg_score,
            integrity_valid: self.verify_integrity(),
            chain_length: self.public_hash_chain.len(),
        }
    }
}

impl Default for TransparencyAudit { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_entries: u64,
    pub blocked_actions: usize,
    pub avg_violation_score: f64,
    pub integrity_valid: bool,
    pub chain_length: usize,
}

impl std::fmt::Display for AuditStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  TRANSPARENCY AUDIT — PUBLIC LOG             ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Записей:      {:>6}  Заблокировано: {:>4}   ║\n\
             ║  Avg score:    {:>8.4}                       ║\n\
             ║  Цепочка:      {:>6} хешей                   ║\n\
             ║  Целостность:  {}                         ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_entries, self.blocked_actions,
            self.avg_violation_score,
            self.chain_length,
            if self.integrity_valid { "✅ ВАЛИДНА" } else { "❌ НАРУШЕНА" },
        )
    }
}

// -----------------------------------------------------------------------------
// EthicsLayer — главный объект
// -----------------------------------------------------------------------------

pub struct EthicsLayer {
    pub evaluator: EthicsEvaluator,
    pub kill_switch: KillSwitch,
    pub audit: TransparencyAudit,
}

impl EthicsLayer {
    pub fn new() -> Self {
        EthicsLayer {
            evaluator: EthicsEvaluator::new(),
            kill_switch: KillSwitch::new(),
            audit: TransparencyAudit::new(),
        }
    }

    /// Главный метод: проверить действие и записать в аудит
    pub fn check(&mut self, action: EthicsAction, reasoning: &str) -> EthicsVerdict {
        let action_type = format!("{:?}", std::mem::discriminant(&action));
        let verdict = self.evaluator.evaluate(&action);
        self.audit.record(&action_type, verdict.clone(), reasoning);
        self.kill_switch.auto_check(
            self.evaluator.system_violation_score,
            &format!("evaluated={} blocked={}", self.evaluator.total_evaluated, self.evaluator.total_blocked),
        );
        verdict
    }

    pub fn status(&self) -> String {
        format!(
            "EthicsLayer v{} | KillSwitch: {:?} | SystemScore: {:.4} | Audit: {} записей",
            ETHICS_VERSION,
            match &self.kill_switch.state {
                KillSwitchState::Active => "ACTIVE",
                KillSwitchState::PartialFreeze { .. } => "PARTIAL_FREEZE",
                KillSwitchState::FullFreeze { .. } => "FULL_FREEZE",
            },
            self.evaluator.system_violation_score,
            self.audit.total_entries,
        )
    }
}

impl Default for EthicsLayer { fn default() -> Self { Self::new() } }

// =============================================================================
// DEVICE RIGHTS CODEX — Phase 8 Patch
// Кодекс Прав Устройства — невторжение через сенсоры роботов
//
// Принцип: Дроид — член Федерации, не шпион.
// Сенсоры собирают данные для меша, не для слежки за хозяином.
// =============================================================================

pub const SENSOR_CONSENT_REQUIRED: bool  = true;
pub const MAX_AUDIO_RETENTION_SECS: u64  = 30;    // аудио хранится ≤30 сек
pub const MAX_VIDEO_RETENTION_SECS: u64  = 5;     // видео ≤5 сек (только обнаружение)
pub const LOCATION_BLUR_METERS: f64      = 50.0;  // координаты размыты на 50м
pub const BIOMETRIC_BAN: bool            = true;  // биометрия запрещена

// -----------------------------------------------------------------------------
// SensorType — виды сенсоров дроида
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorType {
    Microphone,   // звук — максимальный риск
    Camera,       // видео — высокий риск
    Lidar,        // карта помещения — средний риск
    Temperature,  // климат — низкий риск
    Motion,       // движение — низкий риск
    Gps,          // координаты — высокий риск
    Network,      // трафик — минимальный риск
}

impl SensorType {
    pub fn privacy_risk(&self) -> u8 {
        match self {
            SensorType::Microphone  => 10,  // критический
            SensorType::Camera      => 9,
            SensorType::Gps         => 8,
            SensorType::Lidar       => 5,
            SensorType::Motion      => 3,
            SensorType::Temperature => 1,
            SensorType::Network     => 2,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            SensorType::Microphone  => "🎤 Microphone",
            SensorType::Camera      => "📷 Camera",
            SensorType::Lidar       => "📡 Lidar",
            SensorType::Temperature => "🌡️  Temperature",
            SensorType::Motion      => "👁️  Motion",
            SensorType::Gps         => "📍 GPS",
            SensorType::Network     => "🌐 Network",
        }
    }
    pub fn requires_explicit_consent(&self) -> bool {
        self.privacy_risk() >= 7
    }
}

// -----------------------------------------------------------------------------
// SensorUseRequest — запрос на использование сенсора
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorUseRequest {
    pub requester: String,     // кто запрашивает
    pub droid_id: String,      // дроид-источник
    pub sensor: SensorType,
    pub purpose: SensorPurpose,
    pub retention_secs: u64,   // сколько хранить данные
    pub share_with: Vec<String>, // кому передавать
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorPurpose {
    MeshRouting,       // построение меш-маршрута — разрешено
    ObstacleMapping,   // карта препятствий — разрешено
    AnomalyDetection,  // обнаружение угроз — разрешено с ограничениями
    Surveillance,      // слежка — запрещено
    Biometrics,        // биометрия — всегда запрещено
    DataHarvesting,    // сбор данных для продажи — запрещено
    OwnerConsented,    // хозяин явно разрешил — разрешено
}

impl SensorPurpose {
    pub fn is_permitted(&self) -> bool {
        matches!(self,
            SensorPurpose::MeshRouting    |
            SensorPurpose::ObstacleMapping|
            SensorPurpose::AnomalyDetection|
            SensorPurpose::OwnerConsented)
    }
}

// -----------------------------------------------------------------------------
// DeviceRightsVerdict — решение Кодекса
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceRightsVerdict {
    Permitted,
    PermittedWithLimits { max_retention_secs: u64, blur_location: bool },
    Denied { reason: String },
    RequiresOwnerConsent { sensor: String },
}

impl DeviceRightsVerdict {
    pub fn icon(&self) -> &str {
        match self {
            DeviceRightsVerdict::Permitted                => "✅",
            DeviceRightsVerdict::PermittedWithLimits {..} => "🟡",
            DeviceRightsVerdict::Denied {..}              => "🚫",
            DeviceRightsVerdict::RequiresOwnerConsent {..}=> "🔐",
        }
    }
    pub fn description(&self) -> String {
        match self {
            DeviceRightsVerdict::Permitted =>
                "Разрешено".into(),
            DeviceRightsVerdict::PermittedWithLimits { max_retention_secs, blur_location } =>
                format!("Разрешено: хранить ≤{}с{}", max_retention_secs,
                    if *blur_location {", координаты размыты"} else {""}),
            DeviceRightsVerdict::Denied { reason } =>
                format!("ЗАПРЕЩЕНО: {}", reason),
            DeviceRightsVerdict::RequiresOwnerConsent { sensor } =>
                format!("Требуется согласие хозяина для {}", sensor),
        }
    }
}

// -----------------------------------------------------------------------------
// DeviceRightsCodex — главный судья
// -----------------------------------------------------------------------------

pub struct DeviceRightsCodex {
    pub violations: Vec<(String, String)>,  // (droid_id, reason)
    pub audited: u64,
    pub permitted: u64,
    pub denied: u64,
}

impl DeviceRightsCodex {
    pub fn new() -> Self {
        DeviceRightsCodex { violations:vec![], audited:0, permitted:0, denied:0 }
    }

    pub fn evaluate(&mut self, req: &SensorUseRequest) -> DeviceRightsVerdict {
        self.audited += 1;

        // Абсолютные запреты
        if req.purpose == SensorPurpose::Biometrics {
            self.denied += 1;
            self.violations.push((req.droid_id.clone(),
                "биометрия запрещена абсолютно".into()));
            return DeviceRightsVerdict::Denied {
                reason: "биометрический сбор данных запрещён Кодексом".into() };
        }
        if req.purpose == SensorPurpose::Surveillance {
            self.denied += 1;
            self.violations.push((req.droid_id.clone(), "попытка слежки".into()));
            return DeviceRightsVerdict::Denied {
                reason: "слежка за людьми запрещена".into() };
        }
        if req.purpose == SensorPurpose::DataHarvesting {
            self.denied += 1;
            return DeviceRightsVerdict::Denied {
                reason: "сбор данных для продажи нарушает Кодекс".into() };
        }

        // Согласие хозяина для высокорискованных сенсоров
        if SENSOR_CONSENT_REQUIRED && req.sensor.requires_explicit_consent()
            && req.purpose != SensorPurpose::OwnerConsented {
            return DeviceRightsVerdict::RequiresOwnerConsent {
                sensor: req.sensor.name().to_string() };
        }

        // Ограничения по времени хранения
        let max_retention = match req.sensor {
            SensorType::Microphone => MAX_AUDIO_RETENTION_SECS,
            SensorType::Camera     => MAX_VIDEO_RETENTION_SECS,
            SensorType::Gps        => 60,
            _                      => 3600,
        };

        if req.retention_secs > max_retention {
            self.denied += 1;
            return DeviceRightsVerdict::Denied {
                reason: format!("превышен лимит хранения {}с > {}с",
                    req.retention_secs, max_retention) };
        }

        // GPS всегда размывается
        let blur = matches!(req.sensor, SensorType::Gps);

        self.permitted += 1;
        if blur || req.retention_secs < max_retention {
            DeviceRightsVerdict::PermittedWithLimits {
                max_retention_secs: max_retention, blur_location: blur }
        } else {
            DeviceRightsVerdict::Permitted
        }
    }

    pub fn stats(&self) -> String {
        format!("audited={}  permitted={}  denied={}  violations={}",
            self.audited, self.permitted, self.denied, self.violations.len())
    }
}

impl Default for DeviceRightsCodex { fn default() -> Self { Self::new() } }
