// =============================================================================
// src/adaptive_censor.rs — Война 2.0: Адаптивный SuperCensor
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const LEARNING_RATE: f64 = 0.15;
const MEMORY_DECAY: f64 = 0.95;
const PATTERN_THRESHOLD: f64 = 0.70;
const COORDINATION_DELAY_TICKS: u64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BypassTactic {
    Mirage, Onion, Mesh, Satellite, Mutation, Strike, Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockStrategy {
    InternetKill { coverage: f64 },
    DpiFingerprint { effectiveness: f64 },
    SatelliteJam { power: f64 },
    TargetedBlock { ips: Vec<String> },
    PhysicalPressure { rate: f64 },
    Exhaustion { intensity: f64 },
    Combined(Vec<BlockStrategy>),
}

#[derive(Debug, Clone)]
pub struct PatternDetector {
    observed: Vec<(BypassTactic, f64)>,
    tactic_counts: HashMap<BypassTactic, u64>,
    tactic_effectiveness: HashMap<BypassTactic, f64>,
}

impl PatternDetector {
    pub fn new() -> Self {
        PatternDetector {
            observed: Vec::new(),
            tactic_counts: HashMap::new(),
            tactic_effectiveness: HashMap::new(),
        }
    }

    pub fn observe(&mut self, tactic: BypassTactic, success_rate: f64) {
        self.observed.push((tactic.clone(), success_rate));
        *self.tactic_counts.entry(tactic.clone()).or_insert(0) += 1;
        let current = self.tactic_effectiveness.get(&tactic).copied().unwrap_or(0.5);
        let updated = current * (1.0 - LEARNING_RATE) + success_rate * LEARNING_RATE;
        self.tactic_effectiveness.insert(tactic, updated);
    }

    pub fn dominant_tactic(&self) -> Option<BypassTactic> {
        self.tactic_counts.iter().max_by_key(|(_, &count)| count).map(|(tactic, _)| tactic.clone())
    }

    pub fn most_effective_tactic(&self) -> Option<(BypassTactic, f64)> {
        self.tactic_effectiveness.iter().max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()).map(|(t, &e)| (t.clone(), e))
    }

    pub fn decay(&mut self) {
        for eff in self.tactic_effectiveness.values_mut() { *eff *= MEMORY_DECAY; }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveStrategy {
    pub current: BlockStrategy,
    pub effectiveness: f64,
    ticks_since_change: u64,
    cooldown: u64,
}

impl AdaptiveStrategy {
    pub fn new(initial: BlockStrategy) -> Self {
        AdaptiveStrategy { current: initial, effectiveness: 0.5, ticks_since_change: 0, cooldown: 5 }
    }

    pub fn adapt(&mut self, detector: &PatternDetector) {
        self.ticks_since_change += 1;
        if self.ticks_since_change < self.cooldown || self.effectiveness > 0.75 { return; }
        if let Some((tactic, eff)) = detector.most_effective_tactic() {
            if eff > PATTERN_THRESHOLD {
                self.current = Self::counter_strategy(&tactic);
                self.ticks_since_change = 0;
            }
        }
    }

    pub fn counter_strategy(tactic: &BypassTactic) -> BlockStrategy {
        match tactic {
            BypassTactic::Mirage => BlockStrategy::DpiFingerprint { effectiveness: 0.80 },
            BypassTactic::Onion => BlockStrategy::TargetedBlock { ips: vec!["entry".to_string()] },
            BypassTactic::Mesh => BlockStrategy::PhysicalPressure { rate: 0.20 },
            BypassTactic::Satellite => BlockStrategy::SatelliteJam { power: 0.85 },
            BypassTactic::Mutation => BlockStrategy::Combined(vec![
                BlockStrategy::DpiFingerprint { effectiveness: 0.70 },
                BlockStrategy::Exhaustion { intensity: 0.60 },
            ]),
            BypassTactic::Strike => BlockStrategy::InternetKill { coverage: 0.90 },
            BypassTactic::Unknown => BlockStrategy::DpiFingerprint { effectiveness: 0.60 },
        }
    }

    pub fn update_effectiveness(&mut self, bypass_rate: f64) {
        let new_eff = 1.0 - bypass_rate;
        self.effectiveness = self.effectiveness * 0.7 + new_eff * 0.3;
    }
}

#[derive(Debug, Clone)]
pub struct RegionCensor {
    pub region: String,
    pub detector: PatternDetector,
    pub strategy: AdaptiveStrategy,
    pub resources: f64,
    pub last_sync_tick: u64,
}

impl RegionCensor {
    pub fn new(region: &str) -> Self {
        RegionCensor {
            region: region.to_string(),
            detector: PatternDetector::new(),
            strategy: AdaptiveStrategy::new(BlockStrategy::DpiFingerprint { effectiveness: 0.60 }),
            resources: 1.0,
            last_sync_tick: 0,
        }
    }

    pub fn tick(&mut self, bypass_rate: f64, observed_tactic: BypassTactic) {
        self.detector.observe(observed_tactic, bypass_rate);
        self.strategy.adapt(&self.detector);
        self.strategy.update_effectiveness(bypass_rate);
        self.detector.decay();
        self.resources *= 0.98;
    }
}

#[derive(Debug, Clone)]
pub struct MultiRegionCoordinator {
    pub censors: HashMap<String, RegionCensor>,
    pub global_tick: u64,
}

impl MultiRegionCoordinator {
    pub fn new(regions: Vec<&str>) -> Self {
        let mut censors = HashMap::new();
        for region in regions { censors.insert(region.to_string(), RegionCensor::new(region)); }
        MultiRegionCoordinator { censors, global_tick: 0 }
    }

    pub fn coordinate(&mut self) {
        self.global_tick += 1;
        if !self.global_tick.is_multiple_of(COORDINATION_DELAY_TICKS) { return; }
        let mut best_tactic: Option<BypassTactic> = None;
        let mut best_eff = 0.0;
        for censor in self.censors.values() {
            if let Some((tactic, eff)) = censor.detector.most_effective_tactic() {
                if eff > best_eff { best_eff = eff; best_tactic = Some(tactic); }
            }
        }
        if let Some(tactic) = best_tactic {
            let counter = AdaptiveStrategy::counter_strategy(&tactic);
            for censor in self.censors.values_mut() {
                if censor.strategy.effectiveness < 0.60 {
                    censor.strategy.current = counter.clone();
                    censor.strategy.ticks_since_change = 0;
                }
            }
        }
    }

    pub fn tick_region(&mut self, region: &str, bypass_rate: f64, tactic: BypassTactic) {
        if let Some(censor) = self.censors.get_mut(region) { censor.tick(bypass_rate, tactic); }
    }

    pub fn average_effectiveness(&self) -> f64 {
        if self.censors.is_empty() { return 0.0; }
        let sum: f64 = self.censors.values().map(|c| c.strategy.effectiveness).sum();
        sum / self.censors.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_detector_observe() {
        let mut detector = PatternDetector::new();
        detector.observe(BypassTactic::Mirage, 0.80);
        detector.observe(BypassTactic::Mirage, 0.85);
        assert_eq!(detector.dominant_tactic(), Some(BypassTactic::Mirage));
    }

    #[test]
    fn test_most_effective_tactic() {
        let mut detector = PatternDetector::new();
        detector.observe(BypassTactic::Onion, 0.60);
        detector.observe(BypassTactic::Mesh, 0.90);
        let (tactic, _) = detector.most_effective_tactic().unwrap();
        assert_eq!(tactic, BypassTactic::Mesh);
    }

    #[test]
    fn test_adaptive_strategy_counter() {
        let counter = AdaptiveStrategy::counter_strategy(&BypassTactic::Satellite);
        match counter {
            BlockStrategy::SatelliteJam { .. } => {},
            _ => panic!("Неверная контр-стратегия"),
        }
    }

    #[test]
    fn test_multi_region_coordination() {
        let mut coord = MultiRegionCoordinator::new(vec!["CN", "RU", "KP"]);
        assert_eq!(coord.censors.len(), 3);
        coord.tick_region("CN", 0.70, BypassTactic::Mirage);
        coord.coordinate();
        assert!(coord.average_effectiveness() > 0.0);
    }

    #[test]
    fn test_strategy_adapts() {
        let mut strategy = AdaptiveStrategy::new(BlockStrategy::DpiFingerprint { effectiveness: 0.60 });
        let mut detector = PatternDetector::new();
        
        // Наблюдаем тактику Mesh несколько раз, чтобы эффективность превысила 0.70
        for _ in 0..10 {
            detector.observe(BypassTactic::Mesh, 0.85);
        }
        strategy.effectiveness = 0.40;
        strategy.ticks_since_change = 10;
        strategy.adapt(&detector);
        match strategy.current {
            BlockStrategy::PhysicalPressure { .. } => {},
            _ => panic!("Стратегия не адаптировалась"),
        }
    }
}
