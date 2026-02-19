//! Phase 6: Reputation & Trust Graph
//!
//! Obsahuje:
//! - Reputation demo
//! - Trust Graph PageRank demo

/// Hlavní entry point pro Phase 6
pub async fn demo_phase6() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 6: Reputation & Trust Demo        ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_reputation_demo().await;
    run_trust_graph_demo().await;
    
    println!("\n✅ Phase 6 Complete!");
}

pub async fn run_reputation_demo() {
    crate::run_reputation_demo().await;
}

pub async fn run_trust_graph_demo() {
    crate::run_trust_graph_demo().await;
}
