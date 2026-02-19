//! Phase 1: Neural Routing & Federated Learning
//!
//! Obsahuje:
//! - Neural routing demo
//! - Federated learning demo  
//! - Mutation engine demo
//! - Neural tactics demo
//! - Collective tactics demo

/// Hlavní entry point pro Phase 1
pub async fn demo_phase1() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 1: Neural Routing & SSAU Demo     ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_neural_demo().await;
    run_federated_demo().await;
    run_mutation_demo().await;
    run_neural_tactics_demo().await;
    run_collective_tactics_demo().await;
    
    println!("\n✅ Phase 1 Complete!");
}

// ═══════════════════════════════════════════════════════════════
// DEMO FUNKCE (přesunuté z main.rs)
// ═══════════════════════════════════════════════════════════════

pub async fn run_neural_demo() {
    use crate::neural_node::{NeuralRouter, NeuralInput};
    
    println!("=== NEURAL ROUTING DEMO ===");
    let mut router = NeuralRouter::new("nexus-core-01");
    let input = NeuralInput::from_ssau(25.0, 150.0, 0.88, 0.7);
    let output = router.score_route("hub-berlin-01", &input);
    
    println!("Input:  latency={:.1}ms bandwidth={:.1}Mbps", input.latency, input.bandwidth);
    println!("Output: quality_score={:.3} tactic={:?}", output.quality_score, output.tactic);
}

pub async fn run_federated_demo() {
    use crate::federated::{GlobalDefenseModel, FedAvgAggregator, LocalTrainer};
    
    println!("=== FEDERATED LEARNING DEMO ===");
    let _model = GlobalDefenseModel::new();
    let agg = FedAvgAggregator::new();
    
    let nodes = vec!["nexus", "berlin", "tokyo"];
    for node_id in nodes {
        let _trainer = LocalTrainer::new(node_id, "EU");
        // FedAvg aggregate metoda je volána automaticky
        println!("  Node {} contributed to global model", node_id);
    }
    
    println!("Global model training round: {}", agg.round);
}

pub async fn run_mutation_demo() {
    use crate::mutation::{MutationEngine, MutationStrategy};
    
    println!("=== MUTATION ENGINE DEMO ===");
    let strategy = MutationStrategy::AikiReflection { 
    exhaust_factor: 0.8,
    mirror_depth: 3
    };

    let mut engine = MutationEngine::new("nexus-core-01", strategy);
    
    let payload = b"FEDERATION_PULSE:bypass=0.87,region=CN";
    
    // Použijeme standoff layer přímo
    let bundle = engine.standoff.wrap_with_decoys(payload, 3, engine.active_mask.clone());
    let mutated = &bundle.real_payload; 
    
    println!("Original:  {} bytes", payload.len());
    println!("Mutated:   {} bytes ({} decoys)", 

        mutated.len(),
        bundle.decoys.len()
    );
    println!("Strategy:  {:?}", engine.strategy);
    println!("Mutations: {}", engine.mutations_applied);
}

pub async fn run_neural_tactics_demo() {
    use crate::neural_node::{NeuralState, NeuralInput};
    
    println!("=== NEURAL TACTICS DEMO ===");
    let _state = NeuralState::new("nexus-core-01");
    
    let scenarios = vec![
        ("Low latency", 15.0, 200.0, 0.95, 0.9),
        ("High latency", 250.0, 50.0, 0.65, 0.4),
        ("Under attack", 180.0, 80.0, 0.45, 0.2),
    ];
    
    for (name, lat, bw, rel, trust) in scenarios {
        let input = NeuralInput::from_ssau(lat, bw, rel, trust);
        println!("{:15} → latency={:.3} bandwidth={:.3} reliability={:.2}", 
            name, input.latency, input.bandwidth, rel);
    }
}

pub async fn run_collective_tactics_demo() {
    use crate::federated::GlobalDefenseModel;
    
    println!("=== COLLECTIVE TACTICS DEMO ===");
    let model = GlobalDefenseModel::new();
    
    let regions = vec![
        ("CN", 0.95, "AikiReflection"),
        ("RU", 0.75, "Hybrid"),
        ("IR", 0.85, "CumulativeStrike"),
    ];
    
    for (region, difficulty, expected) in regions {
        let score = model.score_for("SuperCensor", expected);
        println!("Region {}: difficulty={:.0}% best_tactic={} score={:.2}", 
            region, difficulty * 100.0, expected, score);
    }
}
