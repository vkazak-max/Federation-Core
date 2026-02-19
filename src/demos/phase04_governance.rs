//! Phase 4: DAO Governance
//!
//! Obsahuje:
//! - Governance demo
//! - Idea Lab demo

/// Hlavní entry point pro Phase 4
pub async fn demo_phase4() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 4: DAO Governance Demo            ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_governance_demo().await;
    run_ideas_demo().await;
    
    println!("\n✅ Phase 4 Complete!");
}

pub async fn run_governance_demo() {
    crate::run_governance_demo().await;
}

pub async fn run_ideas_demo() {
    crate::run_ideas_demo().await;
}
