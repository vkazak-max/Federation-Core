// =============================================================================
// FEDERATION CORE — federated.rs
// PHASE 4 / STEP 2 — «Federated Averaging (FedAvg)»
// =============================================================================
//
// Реализует:
//   1. ModelWeights      — сериализуемые веса нейросети
//   2. LocalTrainer      — локальное обучение на приватных данных
//   3. FedAvgAggregator  — агрегация весов без доступа к данным
//   4. FederatedRound    — один раунд федеративного обучения
//   5. GlobalModel       — глобальная модель + история раундов
// =============================================================================

use crate::neural_node::{NeuralInput, NeuralTarget, NeuralState,
    INPUT_SIZE, HIDDEN_SIZE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_PARTICIPANTS: usize = 3;

// -----------------------------------------------------------------------------
// TacticReport — узел сообщает что сработало, без передачи данных
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticReport {
    pub node_id: String,
    pub region: String,
    pub tactic: String,
    pub censor_type: String,
    pub success_rate: f64,
    pub rounds_tested: u32,
    pub timestamp: i64,
}

impl TacticReport {
    pub fn new(node_id: &str, region: &str, tactic: &str,
        censor_type: &str, success_rate: f64, rounds: u32) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;
        TacticReport {
            node_id: node_id.to_string(), region: region.to_string(),
            tactic: tactic.to_string(), censor_type: censor_type.to_string(),
            success_rate, rounds_tested: rounds, timestamp: now,
        }
    }
}

// -----------------------------------------------------------------------------
// GlobalDefenseModel — коллективная тактическая память
// -----------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalDefenseModel {
    // censor_type → (tactic → avg_success_rate)
    pub tactic_scores: HashMap<String, HashMap<String, f64>>,
    // censor_type → лучшая тактика
    pub best_tactics: HashMap<String, String>,
    pub total_reports: u64,
    pub last_updated_round: u32,
}

impl GlobalDefenseModel {
    pub fn new() -> Self { Self::default() }

    pub fn absorb_report(&mut self, report: &TacticReport) {
        let scores = self.tactic_scores
            .entry(report.censor_type.clone()).or_default();
        // Скользящее среднее: новый = 0.7*старый + 0.3*новый
        let entry = scores.entry(report.tactic.clone()).or_insert(0.5);
        *entry = *entry * 0.7 + report.success_rate * 0.3;
        self.total_reports += 1;
        // Обновляем лучшую тактику для этого цензора
        if let Some((best_tactic, _best_score)) = scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) {
            self.best_tactics.insert(
                report.censor_type.clone(), best_tactic.clone());
        }
    }

    pub fn best_tactic_for(&self, censor_type: &str) -> Option<&str> {
        self.best_tactics.get(censor_type).map(|s| s.as_str())
    }

    pub fn score_for(&self, censor_type: &str, tactic: &str) -> f64 {
        self.tactic_scores.get(censor_type)
            .and_then(|m| m.get(tactic))
            .cloned().unwrap_or(0.5)
    }

    pub fn display(&self) {
        println!("   Глобальная тактическая память:");
        for (censor, tactics) in &self.tactic_scores {
            let best = self.best_tactics.get(censor)
                .cloned().unwrap_or("?".into());
            println!("   Цензор [{:20}] → лучшая тактика: [{}]", censor, best);
            let mut sorted: Vec<_> = tactics.iter().collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
            for (tactic, score) in sorted.iter().take(3) {
                let bar = "█".repeat((*score * 20.0) as usize);
                println!("      {:20} {:.3} {}", tactic, score, bar);
            }
        }
    }
}
pub const MAX_ROUNDS: usize = 100;
pub const CONVERGENCE_THRESHOLD: f64 = 0.001;
pub const LOCAL_EPOCHS: usize = 5;

// -----------------------------------------------------------------------------
// ModelWeights — плоское представление весов для передачи по сети
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWeights {
    pub node_id: String,
    pub round: u32,
    pub l1_weights: Vec<f64>,
    pub l1_biases: Vec<f64>,
    pub l2_weights: Vec<f64>,
    pub l2_biases: Vec<f64>,
    pub training_samples: usize,
    pub local_loss: f64,
    pub local_accuracy: f64,
    pub data_hash: String,
}

impl ModelWeights {
    /// Извлечь веса из NeuralState (не передаём данные — только веса)
    pub fn from_neural_state(state: &NeuralState, round: u32,
        samples: usize, loss: f64, accuracy: f64) -> Self {
        let l1_weights: Vec<f64> = state.layer1.weights.iter()
            .flat_map(|row| row.iter().cloned()).collect();
        let l2_weights: Vec<f64> = state.layer2.weights.iter()
            .flat_map(|row| row.iter().cloned()).collect();

        // Hash данных — доказываем что обучились, не раскрывая данные
        let mut h: u64 = 0xcbf29ce484222325;
        for &w in &l1_weights {
            let bits = w.to_bits();
            h ^= bits; h = h.wrapping_mul(0x100000001b3);
        }
        let data_hash = format!("zkh_{:x}", h & 0xffffffffffff);

        ModelWeights {
            node_id: state.node_id.clone(), round,
            l1_weights, l1_biases: state.layer1.biases.clone(),
            l2_weights, l2_biases: state.layer2.biases.clone(),
            training_samples: samples, local_loss: loss,
            local_accuracy: accuracy, data_hash,
        }
    }

    /// Загрузить веса обратно в NeuralState
    pub fn apply_to_state(&self, state: &mut NeuralState) {
        let chunk = HIDDEN_SIZE;
        for (i, row) in state.layer1.weights.iter_mut().enumerate() {
            for (j, w) in row.iter_mut().enumerate() {
                if let Some(&v) = self.l1_weights.get(i * INPUT_SIZE + j) {
                    *w = v;
                }
            }
        }
        for (i, b) in state.layer1.biases.iter_mut().enumerate() {
            if let Some(&v) = self.l1_biases.get(i) { *b = v; }
        }
        for (i, row) in state.layer2.weights.iter_mut().enumerate() {
            for (j, w) in row.iter_mut().enumerate() {
                if let Some(&v) = self.l2_weights.get(i * chunk + j) {
                    *w = v;
                }
            }
        }
        for (i, b) in state.layer2.biases.iter_mut().enumerate() {
            if let Some(&v) = self.l2_biases.get(i) { *b = v; }
        }
    }

    pub fn total_params(&self) -> usize {
        self.l1_weights.len() + self.l1_biases.len()
            + self.l2_weights.len() + self.l2_biases.len()
    }
}

// -----------------------------------------------------------------------------
// LocalTrainer — локальное обучение на приватных данных узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDataPoint {
    pub input: Vec<f64>,
    pub target: Vec<f64>,
    pub label: String,
}

impl LocalDataPoint {
    pub fn censorship_bypass(success: bool, latency: f64, region: &str) -> Self {
        LocalDataPoint {
            input: vec![
                (latency / 200.0).min(1.0),
                if success { 0.9 } else { 0.1 },
                if success { 0.95 } else { 0.3 },
                0.8, 1.0,
            ],
            target: vec![
                if success { 0.9 } else { 0.1 },
                if success { 0.1 } else { 0.9 },
                if success { 0.85 } else { 0.15 },
            ],
            label: format!("censorship_bypass_{}_{}", region, if success {"ok"} else {"fail"}),
        }
    }

    pub fn attack_pattern(threat_level: f64, attack_type: &str) -> Self {
        LocalDataPoint {
            input: vec![threat_level, 1.0 - threat_level, 0.5, 0.3, 0.9],
            target: vec![1.0 - threat_level, threat_level, 0.5],
            label: format!("attack_{}", attack_type),
        }
    }
}

pub struct LocalTrainer {
    pub node_id: String,
    pub region: String,
    pub local_data: Vec<LocalDataPoint>,
    pub state: NeuralState,
    pub epochs_trained: u64,
    pub local_loss_history: Vec<f64>,
}

impl LocalTrainer {
    pub fn new(node_id: &str, region: &str) -> Self {
        LocalTrainer {
            node_id: node_id.to_string(),
            region: region.to_string(),
            local_data: vec![],
            state: NeuralState::new(node_id),
            epochs_trained: 0,
            local_loss_history: vec![],
        }
    }

    pub fn add_experience(&mut self, point: LocalDataPoint) {
        self.local_data.push(point);
    }

    /// Локальное обучение — данные НИКОГДА не покидают узел
    pub fn train_local(&mut self, epochs: usize) -> (f64, f64) {
        if self.local_data.is_empty() { return (1.0, 0.0); }

        let mut total_loss = 0.0;
        let mut correct = 0;

        for _ in 0..epochs {
            let mut epoch_loss = 0.0;

            for point in &self.local_data {
                let input = NeuralInput {
                    latency:      point.input[0],
                    bandwidth:    point.input[1],
                    reliability:  point.input[2],
                    trust:        point.input[3],
                    ethics_score: point.input[4],
                };
                let target = NeuralTarget {
                    route_weight: point.target[0],
                    congestion:   point.target[1],
                    quality:      point.target[2],
                    decoy:        point.target.get(3).cloned().unwrap_or(0.1),
                    strike:       point.target.get(4).cloned().unwrap_or(0.2),
                    success:      point.target[0] > 0.5,
                };

                let out = self.state.forward(&input);
                let loss: f64 = out.softmax_probs.iter()
                    .zip(point.target.iter())
                    .map(|(o, t)| (o - t).powi(2)).sum::<f64>();
                epoch_loss += loss;

                self.state.backpropagate_success(&input, &target, &self.node_id);
                if (out.route_weight > 0.5) == (point.target[0] > 0.5) {
                    correct += 1;
                }
            }

            total_loss += epoch_loss / self.local_data.len() as f64;
            self.epochs_trained += 1;
        }

        let avg_loss = total_loss / epochs as f64;
        let accuracy = correct as f64 / (self.local_data.len() * epochs) as f64;
        self.local_loss_history.push(avg_loss);
        (avg_loss, accuracy)
    }

    /// Экспортируем ТОЛЬКО веса — данные остаются на узле
    pub fn export_weights(&self, round: u32, loss: f64, accuracy: f64) -> ModelWeights {
        ModelWeights::from_neural_state(
            &self.state, round, self.local_data.len(), loss, accuracy
        )
    }

    /// Принять глобальные веса и начать следующий раунд
    pub fn apply_global_weights(&mut self, weights: &ModelWeights) {
        weights.apply_to_state(&mut self.state);
    }
}

// -----------------------------------------------------------------------------
// FedAvgAggregator — агрегация весов (FedAvg алгоритм)
// -----------------------------------------------------------------------------

pub struct FedAvgAggregator {
    pub round: u32,
    pub collected: Vec<ModelWeights>,
    pub aggregation_history: Vec<AggregationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub round: u32,
    pub participants: usize,
    pub total_samples: usize,
    pub avg_local_loss: f64,
    pub avg_local_accuracy: f64,
    pub weight_divergence: f64,
    pub global_weights: ModelWeights,
}

impl FedAvgAggregator {
    pub fn new() -> Self {
        FedAvgAggregator { round: 0, collected: vec![], aggregation_history: vec![] }
    }

    pub fn collect(&mut self, weights: ModelWeights) {
        self.collected.push(weights);
    }

    /// FedAvg: W_global = Σ (n_k / N) * W_k
    pub fn aggregate(&mut self) -> Option<AggregationResult> {
        if self.collected.len() < MIN_PARTICIPANTS { return None; }

        let total_samples: usize = self.collected.iter()
            .map(|w| w.training_samples).sum();
        if total_samples == 0 { return None; }

        // Взвешенное среднее весов
        let param_count = self.collected[0].l1_weights.len();
        let mut agg_l1w = vec![0.0f64; param_count];
        let mut agg_l1b = vec![0.0f64; self.collected[0].l1_biases.len()];
        let mut agg_l2w = vec![0.0f64; self.collected[0].l2_weights.len()];
        let mut agg_l2b = vec![0.0f64; self.collected[0].l2_biases.len()];

        for w in &self.collected {
            let alpha = w.training_samples as f64 / total_samples as f64;
            for (i, &v) in w.l1_weights.iter().enumerate() { agg_l1w[i] += alpha * v; }
            for (i, &v) in w.l1_biases.iter().enumerate()  { agg_l1b[i] += alpha * v; }
            for (i, &v) in w.l2_weights.iter().enumerate() { agg_l2w[i] += alpha * v; }
            for (i, &v) in w.l2_biases.iter().enumerate()  { agg_l2b[i] += alpha * v; }
        }

        // Считаем дивергенцию весов (насколько узлы расходятся)
        let divergence = self.collected.iter().map(|w| {
            w.l1_weights.iter().zip(agg_l1w.iter())
                .map(|(wi, wg)| (wi - wg).powi(2)).sum::<f64>()
                / param_count as f64
        }).sum::<f64>() / self.collected.len() as f64;

        let avg_loss = self.collected.iter().map(|w| w.local_loss).sum::<f64>()
            / self.collected.len() as f64;
        let avg_acc = self.collected.iter().map(|w| w.local_accuracy).sum::<f64>()
            / self.collected.len() as f64;

        // Создаём глобальные веса
        let global = ModelWeights {
            node_id: "GLOBAL".to_string(),
            round: self.round,
            l1_weights: agg_l1w,
            l1_biases: agg_l1b,
            l2_weights: agg_l2w,
            l2_biases: agg_l2b,
            training_samples: total_samples,
            local_loss: avg_loss,
            local_accuracy: avg_acc,
            data_hash: format!("global_r{}", self.round),
        };

        let result = AggregationResult {
            round: self.round,
            participants: self.collected.len(),
            total_samples, avg_local_loss: avg_loss,
            avg_local_accuracy: avg_acc,
            weight_divergence: divergence,
            global_weights: global,
        };

        self.aggregation_history.push(result.clone());
        self.collected.clear();
        self.round += 1;
        Some(result)
    }

    pub fn ready(&self) -> bool {
        self.collected.len() >= MIN_PARTICIPANTS
    }
}

impl Default for FedAvgAggregator { fn default() -> Self { Self::new() } }

// -----------------------------------------------------------------------------
// FederatedNetwork — полная федеративная сеть
// -----------------------------------------------------------------------------

pub struct FederatedNetwork {
    pub trainers: HashMap<String, LocalTrainer>,
    pub aggregator: FedAvgAggregator,
    pub global_round: u32,
    pub convergence_history: Vec<f64>,
    pub tactic_reports: Vec<TacticReport>,
    pub defense_model: GlobalDefenseModel,
}

impl FederatedNetwork {
    pub fn new() -> Self {
        FederatedNetwork {
            trainers: HashMap::new(),
            aggregator: FedAvgAggregator::new(),
            global_round: 0,
            convergence_history: vec![],
            tactic_reports: vec![],
            defense_model: GlobalDefenseModel::new(),
        }
    }

    pub fn add_node(&mut self, node_id: &str, region: &str) {
        self.trainers.insert(node_id.to_string(),
            LocalTrainer::new(node_id, region));
    }

    /// Один раунд федеративного обучения
    pub fn run_round(&mut self) -> Option<AggregationResult> {
        // 1. Каждый узел обучается локально
        let mut exported = vec![];
        for trainer in self.trainers.values_mut() {
            if trainer.local_data.is_empty() { continue; }
            let (loss, acc) = trainer.train_local(LOCAL_EPOCHS);
            let w = trainer.export_weights(self.global_round, loss, acc);
            exported.push(w);
        }

        // 2. Отправляем ТОЛЬКО веса агрегатору (не данные)
        for w in exported { self.aggregator.collect(w); }

        // 3. FedAvg агрегация
        if let Some(result) = self.aggregator.aggregate() {
            self.convergence_history.push(result.avg_local_loss);

            // 4. Рассылаем глобальные веса обратно узлам
            let global_w = result.global_weights.clone();
            for trainer in self.trainers.values_mut() {
                trainer.apply_global_weights(&global_w);
            }

            self.global_round += 1;
            return Some(result);
        }
        None
    }

    /// Узел сообщает о тактическом опыте
    pub fn submit_tactic_report(&mut self, report: TacticReport) {
        self.defense_model.absorb_report(&report);
        self.tactic_reports.push(report);
    }

    /// Получить рекомендацию для конкретного типа цензора
    pub fn recommend_tactic(&self, censor_type: &str) -> String {
        self.defense_model.best_tactic_for(censor_type)
            .unwrap_or("StandoffDecoy")
            .to_string()
    }

    /// Запустить тактический раунд — агрегация тактических знаний
    pub fn run_tactical_round(&mut self, new_reports: Vec<TacticReport>)
        -> TacticalRoundResult {
        let count = new_reports.len();
        let mut censor_updates: HashMap<String, String> = HashMap::new();

        for report in new_reports {
            // Запоминаем старую лучшую тактику
            let old_best = self.defense_model
                .best_tactic_for(&report.censor_type)
                .map(|s| s.to_string());
            self.submit_tactic_report(report.clone());
            // Проверяем изменилась ли рекомендация
            let new_best = self.recommend_tactic(&report.censor_type);
            if old_best.as_deref() != Some(&new_best) {
                censor_updates.insert(report.censor_type.clone(), new_best);
            }
        }

        TacticalRoundResult {
            reports_processed: count,
            tactic_switches: censor_updates.len(),
            updated_censors: censor_updates,
            total_censor_types: self.defense_model.tactic_scores.len(),
            global_round: self.global_round,
        }
    }

    pub fn is_converged(&self) -> bool {
        if self.convergence_history.len() < 3 { return false; }
        let n = self.convergence_history.len();
        let delta = (self.convergence_history[n-1]
            - self.convergence_history[n-3]).abs();
        delta < CONVERGENCE_THRESHOLD
    }

    pub fn stats(&self) -> FedStats {
        FedStats {
            total_nodes: self.trainers.len(),
            global_round: self.global_round,
            total_samples: self.trainers.values().map(|t| t.local_data.len()).sum(),
            converged: self.is_converged(),
            avg_local_loss: self.convergence_history.last().cloned().unwrap_or(1.0),
            rounds_history: self.convergence_history.clone(),
        }
    }
}

impl Default for FederatedNetwork { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct TacticalRoundResult {
    pub reports_processed: usize,
    pub tactic_switches: usize,
    pub updated_censors: HashMap<String, String>,
    pub total_censor_types: usize,
    pub global_round: u32,
}

impl std::fmt::Display for TacticalRoundResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "TacticalRound: отчётов={} переключений={} цензоров={}",
            self.reports_processed, self.tactic_switches, self.total_censor_types)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FedStats {
    pub total_nodes: usize,
    pub global_round: u32,
    pub total_samples: usize,
    pub converged: bool,
    pub avg_local_loss: f64,
    pub rounds_history: Vec<f64>,
}

impl std::fmt::Display for FedStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  FEDERATED LEARNING — NETWORK STATUS         ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Узлов:      {:>4}  Раундов:    {:>4}         ║\n\
             ║  Примеров:   {:>4}  Сошлось:    {}          ║\n\
             ║  Avg loss:   {:>8.4}                         ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_nodes, self.global_round,
            self.total_samples,
            if self.converged { "✅ ДА " } else { "⏳ НЕТ" },
            self.avg_local_loss,
        )
    }
}
