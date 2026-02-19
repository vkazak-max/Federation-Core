//! Phase 10: CLI Dashboard
//!
//! Obsahuje:
//! - Live dashboard demo

/// Hlavní entry point pro Phase 10
pub async fn demo_phase10() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 10: CLI Dashboard Demo            ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_dashboard_demo().await;
    
    println!("\n✅ Phase 10 Complete!");
}

pub async fn run_dashboard_demo() {
    crate::run_dashboard_demo().await;
}
