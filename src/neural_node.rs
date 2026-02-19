// =============================================================================
// FEDERATION CORE — neural_node.rs
// PHASE 4 — «Neural Node: Узел который думает»
// =============================================================================
//
// Реализует:
//   1. NeuralState     — матрица весов для выбора соседей
//   2. FeedForward     — прямой проход через сеть
//   3. backpropagate_success() — обучение на успехе
//   4. predict_congestion()   — предсказание заторов
//   5. NeuralRouter    — замена Softmax AI Router на нейронный
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Константы
// -----------------------------------------------------------------------------

pub const INPUT_SIZE: usize = 5;
pub const HIDDEN_SIZE: usize = 8;
pub const OUTPUT_SIZE: usize = 5;
pub const LEARNING_RATE: f64 = 0.15;
pub const MOMENTUM: f64 = 0.5;
pub const CONGESTION_WINDOW: usize = 10;
pub const CONGESTION_THRESHOLD: f64 = 0.65;

// -----------------------------------------------------------------------------
// Функции активации
// -----------------------------------------------------------------------------

fn relu(x: f64) -> f64 { x.max(0.0) }
fn relu_derivative(x: f64) -> f64 { if x > 0.0 { 1.0 } else { 0.0 } }
fn sigmoid(x: f64) -> f64 { 1.0 / (1.0 + (-x).exp()) }
fn softmax(v: &[f64]) -> Vec<f64> {
    let max = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = v.iter().map(|x| (x - max).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|e| e / sum).collect()
}

// -----------------------------------------------------------------------------
// NeuralWeights — матрица весов слоя
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralWeights {
    pub weights: Vec<Vec<f64>>,
    pub biases: Vec<f64>,
    pub velocity: Vec<Vec<f64>>,
    pub bias_velocity: Vec<f64>,
}

impl NeuralWeights {
    pub fn new(input: usize, output: usize, seed: u64) -> Self {
        let mut rng = seed;
        let weights = (0..output).map(|_| {
            (0..input).map(|_| {
                rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
                ((rng as i64 % 1000) as f64) / 5000.0
            }).collect()
        }).collect();
        let biases = vec![0.0; output];
        let velocity = vec![vec![0.0; input]; output];
        let bias_velocity = vec![0.0; output];
        NeuralWeights { weights, biases, velocity, bias_velocity }
    }

    pub fn forward(&self, input: &[f64]) -> Vec<f64> {
        self.weights.iter().zip(self.biases.iter()).map(|(row, bias)| {
            row.iter().zip(input.iter()).map(|(w, x)| w * x).sum::<f64>() + bias
        }).collect()
    }

    pub fn update(&mut self, grad_w: &[Vec<f64>], grad_b: &[f64]) {
        for i in 0..self.weights.len() {
            for j in 0..self.weights[i].len() {
                self.velocity[i][j] = MOMENTUM * self.velocity[i][j]
                    - LEARNING_RATE * grad_w[i][j];
                self.weights[i][j] += self.velocity[i][j];
            }
            self.bias_velocity[i] = MOMENTUM * self.bias_velocity[i]
                - LEARNING_RATE * grad_b[i];
            self.biases[i] += self.bias_velocity[i];
        }
    }
}

// -----------------------------------------------------------------------------
// NeuralState — полное состояние нейронной сети узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralState {
    pub node_id: String,
    pub layer1: NeuralWeights,
    pub layer2: NeuralWeights,
    pub training_steps: u64,
    pub total_loss: f64,
    pub success_rate: f64,
    pub neighbor_weights: HashMap<String, f64>,
    pub congestion_history: Vec<f64>,
    pub last_prediction: Option<CongestionPrediction>,
}

impl NeuralState {
    pub fn new(node_id: &str) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in node_id.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        NeuralState {
            node_id: node_id.to_string(),
            layer1: NeuralWeights::new(INPUT_SIZE, HIDDEN_SIZE, h),
            layer2: NeuralWeights::new(HIDDEN_SIZE, OUTPUT_SIZE, h ^ 0xdeadbeef),
            training_steps: 0,
            total_loss: 0.0,
            success_rate: 0.5,
            neighbor_weights: HashMap::new(),
            congestion_history: vec![],
            last_prediction: None,
        }
    }

    /// Прямой проход: входной вектор → [route_weight, congestion_prob, quality_score]
    pub fn forward(&self, input: &NeuralInput) -> NeuralOutput {
        let x = input.to_vector();
        let h1: Vec<f64> = self.layer1.forward(&x).iter().map(|&v| relu(v)).collect();
        let out = self.layer2.forward(&h1);
        let probs = softmax(&out);
        let congestion  = sigmoid(out[1]);
        let decoy       = sigmoid(out[3]);
        let strike      = sigmoid(out[4]);
        NeuralOutput {
            route_weight:    sigmoid(out[0]),
            congestion_prob: congestion,
            quality_score:   sigmoid(out[2]),
            decoy_intensity: decoy,
            strike_focus:    strike,
            softmax_probs:   probs,
            hidden_state:    h1,
            tactic:          NeuralTactic::decide(congestion, decoy, strike),
        }
    }

    /// Обучение на успехе: пакет дошёл → закрепляем путь
    pub fn backpropagate_success(&mut self, input: &NeuralInput,
        target: &NeuralTarget, neighbor_id: &str) {
        let x = input.to_vector();
        let h1_raw = self.layer1.forward(&x);
        let h1: Vec<f64> = h1_raw.iter().map(|&v| relu(v)).collect();
        let out = self.layer2.forward(&h1);

        // Loss = MSE между выходом и целевым значением
        let target_vec = target.to_vector();
        let loss: f64 = out.iter().zip(target_vec.iter())
            .map(|(o, t)| (o - t).powi(2)).sum::<f64>() / OUTPUT_SIZE as f64;
        self.total_loss = self.total_loss * 0.99 + loss * 0.01;

        // Градиент output слоя: δ = 2*(out - target) / N
        let delta2: Vec<f64> = out.iter().zip(target_vec.iter())
            .map(|(o, t)| 2.0 * (o - t) / OUTPUT_SIZE as f64).collect();

        // Градиент весов layer2: dL/dW2 = δ2 ⊗ h1
        let grad_w2: Vec<Vec<f64>> = delta2.iter()
            .map(|d| h1.iter().map(|h| d * h).collect()).collect();
        let grad_b2: Vec<f64> = delta2.clone();

        // Backprop через hidden: δ1 = (W2^T · δ2) * relu'(h1_raw)
        let mut delta1 = vec![0.0; HIDDEN_SIZE];
        for j in 0..HIDDEN_SIZE {
            for k in 0..OUTPUT_SIZE {
                delta1[j] += self.layer2.weights[k][j] * delta2[k];
            }
            delta1[j] *= relu_derivative(h1_raw[j]);
        }

        // Градиент весов layer1: dL/dW1 = δ1 ⊗ x
        let grad_w1: Vec<Vec<f64>> = delta1.iter()
            .map(|d| x.iter().map(|xi| d * xi).collect()).collect();
        let grad_b1: Vec<f64> = delta1;

        // Обновляем веса
        self.layer1.update(&grad_w1, &grad_b1);
        self.layer2.update(&grad_w2, &grad_b2);

        // Обновляем вес соседа
        let reward = if target.success { 0.1 } else { -0.05 };
        let w = self.neighbor_weights.entry(neighbor_id.to_string()).or_insert(0.5);
        *w = (*w + reward).clamp(0.0, 1.0);

        // Обновляем success rate
        if target.success {
            self.success_rate = self.success_rate * 0.95 + 0.05;
        } else {
            self.success_rate *= 0.95;
        }

        self.training_steps += 1;
    }

    /// Предсказать затор на основе истории задержек
    pub fn predict_congestion(&mut self, current_latency_ms: f64) -> CongestionPrediction {
        self.congestion_history.push(current_latency_ms);
        if self.congestion_history.len() > CONGESTION_WINDOW {
            self.congestion_history.remove(0);
        }

        let history = &self.congestion_history;
        if history.len() < 3 {
            return CongestionPrediction {
                probability: 0.0, trend: Trend::Stable,
                predicted_latency_ms: current_latency_ms,
                confidence: 0.1, action: "Недостаточно данных".into(),
            };
        }

        // Линейный тренд: наклон скользящего среднего
        let n = history.len() as f64;
        let mean = history.iter().sum::<f64>() / n;
        let slope = history.iter().enumerate()
            .map(|(i, &v)| (i as f64 - n / 2.0) * (v - mean))
            .sum::<f64>() / history.iter().enumerate()
            .map(|(i, _)| (i as f64 - n / 2.0).powi(2))
            .sum::<f64>().max(1e-10);

        // Нейронная оценка через forward pass
        let norm_latency = (current_latency_ms / 200.0).min(1.0);
        let _norm_slope = (slope / 50.0).clamp(-1.0, 1.0);
        let std_dev = {
            let var = history.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
            var.sqrt() / 100.0
        };

        let neural_input = NeuralInput {
            latency: norm_latency,
            bandwidth: 1.0 - norm_latency,
            reliability: 1.0 - std_dev.min(1.0),
            trust: self.success_rate,
            ethics_score: 1.0,
        };
        let neural_out = self.forward(&neural_input);

        let base_prob = neural_out.congestion_prob;
        let trend_factor = if slope > 5.0 { slope / 50.0 } else { 0.0 };
        let probability = (base_prob + trend_factor * 0.3).clamp(0.0, 1.0);

        let trend = if slope > 5.0 { Trend::Rising }
            else if slope < -5.0 { Trend::Falling }
            else { Trend::Stable };

        let predicted_latency = current_latency_ms + slope * 5.0;
        let confidence = (history.len() as f64 / CONGESTION_WINDOW as f64).min(1.0);

        let action = if probability > CONGESTION_THRESHOLD {
            format!("⚠️  ЗАТОР ПРЕДСКАЗАН! Переключить маршрут (prob={:.1}%)", probability * 100.0)
        } else if probability > 0.4 {
            format!("📈 Нагрузка растёт, мониторинг (prob={:.1}%)", probability * 100.0)
        } else {
            format!("✅ Сеть стабильна (prob={:.1}%)", probability * 100.0)
        };

        let pred = CongestionPrediction {
            probability, trend, predicted_latency_ms: predicted_latency,
            confidence, action,
        };
        self.last_prediction = Some(pred.clone());
        pred
    }
}

// -----------------------------------------------------------------------------
// Вспомогательные структуры
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralInput {
    pub latency: f64,
    pub bandwidth: f64,
    pub reliability: f64,
    pub trust: f64,
    pub ethics_score: f64,
}

impl NeuralInput {
    pub fn to_vector(&self) -> Vec<f64> {
        vec![self.latency, self.bandwidth, self.reliability,
             self.trust, self.ethics_score]
    }

    pub fn from_ssau(latency_ms: f64, bandwidth_mbps: f64,
        reliability: f64, trust: f64) -> Self {
        NeuralInput {
            latency:      (latency_ms / 200.0).min(1.0),
            bandwidth:    (bandwidth_mbps / 1000.0).min(1.0),
            reliability,
            trust:        trust.clamp(0.0, 1.0),
            ethics_score: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NeuralTactic {
    Passive,
    StandoffDecoy,
    CumulativeStrike,
    AikiReflection,
    Hybrid,
}

impl NeuralTactic {
    /// Тактика определяется по входному вектору напрямую
    /// neural outputs используются как веса уверенности
    pub fn decide(congestion: f64, decoy: f64, strike: f64) -> Self {
        // Нормализуем относительно центра 0.5
        let d = decoy - 0.5;    // отклонение decoy от нейтрали
        let s = strike - 0.5;   // отклонение strike от нейтрали
        let c = congestion - 0.5;

        // Выбираем по максимальному отклонению
        if c > 0.1 && s > 0.05 {
            NeuralTactic::AikiReflection
        } else if s > 0.1 && d > 0.1 {
            NeuralTactic::Hybrid
        } else if s > d && s > 0.05 {
            NeuralTactic::CumulativeStrike
        } else if d > s && d > 0.05 {
            NeuralTactic::StandoffDecoy
        } else {
            NeuralTactic::Passive
        }
    }

    /// Тактика с учётом входного вектора (более точная)
    pub fn decide_from_input(latency: f64, congestion: f64,
        decoy: f64, strike: f64) -> Self {
        // Латентность напрямую говорит об угрозе
        if latency > 0.80 && congestion > 0.60 {
            NeuralTactic::AikiReflection
        } else if latency > 0.70 && decoy > 0.50 {
            NeuralTactic::StandoffDecoy
        } else if latency < 0.50 && strike > 0.50 {
            NeuralTactic::CumulativeStrike
        } else if latency > 0.40 && decoy > 0.50 && strike > 0.50 {
            NeuralTactic::Hybrid
        } else {
            NeuralTactic::Passive
        }
    }
    pub fn name(&self) -> &str {
        match self {
            NeuralTactic::Passive          => "Passive",
            NeuralTactic::StandoffDecoy    => "StandoffDecoy",
            NeuralTactic::CumulativeStrike => "CumulativeStrike",
            NeuralTactic::AikiReflection   => "AikiReflection",
            NeuralTactic::Hybrid           => "Hybrid",
        }
    }
}

pub struct NeuralOutput {
    pub route_weight:    f64,
    pub congestion_prob: f64,
    pub quality_score:   f64,
    pub decoy_intensity: f64,  // 0=нет брони  1=максимум коробочек
    pub strike_focus:    f64,  // 0=рассеянный  1=кумулятивный удар
    pub softmax_probs:   Vec<f64>,
    pub hidden_state:    Vec<f64>,
    pub tactic:          NeuralTactic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralTarget {
    pub route_weight:  f64,
    pub congestion:    f64,
    pub quality:       f64,
    pub decoy:         f64,  // целевая интенсивность брони
    pub strike:        f64,  // целевой фокус удара
    pub success:       bool,
}

impl NeuralTarget {
    pub fn success_route(quality: f64) -> Self {
        NeuralTarget { route_weight: 0.9, congestion: 0.1,
            quality, decoy: 0.1, strike: 0.2, success: true }
    }
    pub fn failed_route() -> Self {
        NeuralTarget { route_weight: 0.1, congestion: 0.9,
            quality: 0.1, decoy: 0.8, strike: 0.1, success: false }
    }
    pub fn under_attack(decoy: f64, strike: f64) -> Self {
        NeuralTarget { route_weight: 0.5, congestion: 0.7,
            quality: 0.4, decoy, strike, success: false }
    }
    pub fn cumulative_strike() -> Self {
        NeuralTarget { route_weight: 0.8, congestion: 0.3,
            quality: 0.7, decoy: 0.2, strike: 0.95, success: true }
    }
    pub fn to_vector(&self) -> Vec<f64> {
        vec![self.route_weight, self.congestion, self.quality,
             self.decoy, self.strike]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CongestionPrediction {
    pub probability: f64,
    pub trend: Trend,
    pub predicted_latency_ms: f64,
    pub confidence: f64,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trend { Rising, Falling, Stable }

// -----------------------------------------------------------------------------
// NeuralRouter — нейронный маршрутизатор
// -----------------------------------------------------------------------------

pub struct NeuralRouter {
    pub node_id: String,
    pub states: HashMap<String, NeuralState>,
    pub global_state: NeuralState,
    pub routes_computed: u64,
    pub routes_improved: u64,
}

impl NeuralRouter {
    pub fn new(node_id: &str) -> Self {
        NeuralRouter {
            node_id: node_id.to_string(),
            states: HashMap::new(),
            global_state: NeuralState::new(node_id),
            routes_computed: 0,
            routes_improved: 0,
        }
    }

    /// Оценить маршрут через нейронную сеть
    pub fn score_route(&mut self, neighbor_id: &str, input: &NeuralInput) -> NeuralOutput {
        let state = self.states.entry(neighbor_id.to_string())
            .or_insert_with(|| NeuralState::new(neighbor_id));
        self.routes_computed += 1;
        state.forward(input)
    }

    /// Выбрать лучший маршрут из кандидатов
    pub fn select_best(&mut self, candidates: Vec<(String, NeuralInput)>) -> Option<String> {
        if candidates.is_empty() { return None; }
        let scored: Vec<(String, f64)> = candidates.iter().map(|(id, input)| {
            let state = self.states.entry(id.clone())
                .or_insert_with(|| NeuralState::new(id));
            let out = state.forward(input);
            let score = out.route_weight * 0.5
                + out.quality_score * 0.3
                + (1.0 - out.congestion_prob) * 0.2;
            let neighbor_bonus = *state.neighbor_weights.get(id).unwrap_or(&0.5);
            (id.clone(), score + neighbor_bonus * 0.1)
        }).collect();

        scored.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }

    /// Обучить сеть на результате доставки
    pub fn train_on_delivery(&mut self, neighbor_id: &str,
        input: &NeuralInput, success: bool, quality: f64) {
        let target = if success {
            NeuralTarget::success_route(quality)
        } else {
            NeuralTarget::failed_route()
        };
        let state = self.states.entry(neighbor_id.to_string())
            .or_insert_with(|| NeuralState::new(neighbor_id));
        state.backpropagate_success(input, &target, neighbor_id);
        if success { self.routes_improved += 1; }
    }

    pub fn stats(&self) -> RouterNeuralStats {
        let avg_success = if self.states.is_empty() { 0.0 } else {
            self.states.values().map(|s| s.success_rate).sum::<f64>()
                / self.states.len() as f64
        };
        RouterNeuralStats {
            nodes_tracked: self.states.len(),
            routes_computed: self.routes_computed,
            routes_improved: self.routes_improved,
            avg_success_rate: avg_success,
            training_steps: self.states.values().map(|s| s.training_steps).sum(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RouterNeuralStats {
    pub nodes_tracked: usize,
    pub routes_computed: u64,
    pub routes_improved: u64,
    pub avg_success_rate: f64,
    pub training_steps: u64,
}

impl std::fmt::Display for RouterNeuralStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  NEURAL ROUTER — LEARNING STATS              ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Узлов отслежено:  {:>6}                     ║\n\
             ║  Маршрутов:        {:>6}  Улучшено: {:>6}   ║\n\
             ║  Avg success rate: {:>8.4}                   ║\n\
             ║  Training steps:   {:>6}                     ║\n\
             ╚══════════════════════════════════════════════╝",
            self.nodes_tracked,
            self.routes_computed, self.routes_improved,
            self.avg_success_rate, self.training_steps,
        )
    }
}

// =============================================================================
// RESOURCE SELF-AWARENESS — Phase 8 Patch
// Локальный ИИ знает свои ограничения и адаптирует нагрузку
//
//   ResourceProfile  — снимок ресурсов узла
//   ComputeBudget    — сколько ИИ может потратить
//   AdaptiveScheduler — диспетчер задач по бюджету
// =============================================================================

pub const CPU_HEAVY_THRESHOLD: f64  = 0.70; // >70% CPU = тяжело
pub const MEM_CRITICAL: f64         = 0.85; // >85% RAM = критично
pub const BATTERY_LOW: f64          = 0.20; // <20% = энергосбережение
pub const THERMAL_THROTTLE: f64     = 80.0; // °C — троттлинг

// -----------------------------------------------------------------------------
// ResourceProfile — текущее состояние ресурсов узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ResourceProfile {
    pub node_id: String,
    pub cpu_cores: u8,
    pub cpu_load: f64,        // 0.0-1.0
    pub ram_total_mb: u32,
    pub ram_used_mb: u32,
    pub battery_pct: Option<f64>,  // None = сетевое питание
    pub temp_celsius: f64,
    pub is_mobile: bool,
    pub device_role: String,   // из inventory.rs
}

impl ResourceProfile {
    pub fn ram_load(&self) -> f64 {
        self.ram_used_mb as f64 / self.ram_total_mb.max(1) as f64
    }
    pub fn is_throttling(&self) -> bool {
        self.temp_celsius >= THERMAL_THROTTLE
    }
    pub fn is_low_battery(&self) -> bool {
        self.battery_pct.map(|b| b < BATTERY_LOW).unwrap_or(false)
    }
    pub fn compute_score(&self) -> f64 {
        // Нормализованная вычислительная мощность 0.0-1.0
        let cpu_avail  = (1.0 - self.cpu_load).max(0.0);
        let ram_avail  = (1.0 - self.ram_load()).max(0.0);
        let thermal_ok = if self.is_throttling() { 0.5 } else { 1.0 };
        let battery_ok = if self.is_low_battery() { 0.3 } else { 1.0 };
        cpu_avail * 0.4 + ram_avail * 0.3 + thermal_ok * 0.2 + battery_ok * 0.1
    }
}

// -----------------------------------------------------------------------------
// ComputeBudget — что ИИ может позволить себе делать
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ComputeBudget {
    Full,       // Sentinel/Citadel — всё доступно
    Reduced,    // Workstation — ограничена тяжёлая аналитика
    Minimal,    // Ghost/Mobile — только базовые операции
    Emergency,  // критически мало ресурсов — только heartbeat
}

impl ComputeBudget {
    pub fn from_profile(p: &ResourceProfile) -> Self {
        let score = p.compute_score();
        if p.is_low_battery() || score < 0.15  { ComputeBudget::Emergency }
        else if score < 0.35 || p.is_throttling() { ComputeBudget::Minimal }
        else if score < 0.65                     { ComputeBudget::Reduced }
        else                                     { ComputeBudget::Full }
    }
    pub fn name(&self) -> &str {
        match self {
            ComputeBudget::Full      => "⚡ Full",
            ComputeBudget::Reduced   => "🟡 Reduced",
            ComputeBudget::Minimal   => "🟠 Minimal",
            ComputeBudget::Emergency => "🔴 Emergency",
        }
    }
    // Максимальное число нейронных слоёв для forward pass
    pub fn max_layers(&self) -> usize {
        match self {
            ComputeBudget::Full      => 3,
            ComputeBudget::Reduced   => 2,
            ComputeBudget::Minimal   => 1,
            ComputeBudget::Emergency => 0,
        }
    }
    // Разрешены ли тяжёлые операции
    pub fn allows_heavy_analytics(&self) -> bool {
        matches!(self, ComputeBudget::Full)
    }
    pub fn allows_federated_training(&self) -> bool {
        matches!(self, ComputeBudget::Full | ComputeBudget::Reduced)
    }
    pub fn allows_onion_routing(&self) -> bool {
        !matches!(self, ComputeBudget::Emergency)
    }
    pub fn inference_interval_ms(&self) -> u32 {
        match self {
            ComputeBudget::Full      => 100,
            ComputeBudget::Reduced   => 500,
            ComputeBudget::Minimal   => 2000,
            ComputeBudget::Emergency => 10000,
        }
    }
}

// -----------------------------------------------------------------------------
// AdaptiveTask — задача с весом ресурсов
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AdaptiveTask {
    pub name: String,
    pub required_budget: ComputeBudget,
    pub cpu_weight: f64,   // относительная нагрузка на CPU
    pub priority: u8,      // 0=низкий 255=критический
}

impl AdaptiveTask {
    pub fn new(name: &str, budget: ComputeBudget, cpu: f64, prio: u8) -> Self {
        AdaptiveTask { name: name.to_string(), required_budget: budget,
            cpu_weight: cpu, priority: prio }
    }
    pub fn standard_tasks() -> Vec<Self> {
        vec![
            AdaptiveTask::new("heartbeat",           ComputeBudget::Emergency, 0.01, 255),
            AdaptiveTask::new("basic_routing",       ComputeBudget::Minimal,   0.05, 200),
            AdaptiveTask::new("onion_relay",         ComputeBudget::Minimal,   0.10, 180),
            AdaptiveTask::new("neural_inference",    ComputeBudget::Reduced,   0.20, 150),
            AdaptiveTask::new("reputation_update",   ComputeBudget::Reduced,   0.15, 120),
            AdaptiveTask::new("federated_training",  ComputeBudget::Reduced,   0.35, 100),
            AdaptiveTask::new("heavy_analytics",     ComputeBudget::Full,      0.60,  80),
            AdaptiveTask::new("dao_simulation",      ComputeBudget::Full,      0.70,  70),
            AdaptiveTask::new("zk_proof_generation", ComputeBudget::Full,      0.55,  90),
        ]
    }
}

// -----------------------------------------------------------------------------
// AdaptiveScheduler — диспетчер задач по бюджету
// -----------------------------------------------------------------------------

pub struct AdaptiveScheduler {
    pub node_id: String,
    pub profile: ResourceProfile,
    pub budget: ComputeBudget,
    pub scheduled: Vec<AdaptiveTask>,
    pub skipped: Vec<AdaptiveTask>,
}

impl AdaptiveScheduler {
    pub fn new(profile: ResourceProfile) -> Self {
        let budget = ComputeBudget::from_profile(&profile);
        AdaptiveScheduler {
            node_id: profile.node_id.clone(),
            profile, budget,
            scheduled: vec![], skipped: vec![],
        }
    }

    pub fn schedule(&mut self, tasks: Vec<AdaptiveTask>) {
        self.scheduled.clear();
        self.skipped.clear();

        let budget_level = match self.budget {
            ComputeBudget::Full      => 4u8,
            ComputeBudget::Reduced   => 3,
            ComputeBudget::Minimal   => 2,
            ComputeBudget::Emergency => 1,
        };

        let mut sorted = tasks;
        sorted.sort_by(|a,b| b.priority.cmp(&a.priority));

        let mut total_cpu = 0.0f64;
        for task in sorted {
            let task_level = match task.required_budget {
                ComputeBudget::Emergency => 1u8,
                ComputeBudget::Minimal   => 2,
                ComputeBudget::Reduced   => 3,
                ComputeBudget::Full      => 4,
            };
            let cpu_ok = total_cpu + task.cpu_weight <= 0.90;
            if task_level <= budget_level && cpu_ok {
                total_cpu += task.cpu_weight;
                self.scheduled.push(task);
            } else {
                self.skipped.push(task);
            }
        }
    }

    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            node_id: self.node_id.clone(),
            budget: self.budget.clone(),
            compute_score: self.profile.compute_score(),
            scheduled_count: self.scheduled.len(),
            skipped_count: self.skipped.len(),
            cpu_load: self.profile.cpu_load,
            ram_load: self.profile.ram_load(),
            is_throttling: self.profile.is_throttling(),
            inference_interval_ms: self.budget.inference_interval_ms(),
        }
    }
}

#[derive(Debug)]
pub struct SchedulerStats {
    pub node_id: String,
    pub budget: ComputeBudget,
    pub compute_score: f64,
    pub scheduled_count: usize,
    pub skipped_count: usize,
    pub cpu_load: f64,
    pub ram_load: f64,
    pub is_throttling: bool,
    pub inference_interval_ms: u32,
}
