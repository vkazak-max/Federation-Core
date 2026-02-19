//! Phase 11: War Simulation
//!
//! Obsahuje:
//! - VeilBreaker test
//! - War simulation demo

/// Hlavní entry point pro Phase 11
pub async fn demo_phase11() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 11: War Simulation Demo           ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_veil_breaker().await;
    run_war_demo().await;
    
    println!("\n✅ Phase 11 Complete!");
}

pub async fn run_veil_breaker() {
    crate::run_veil_breaker().await;
}

pub async fn run_war_demo() {
    crate::run_war_demo().await;
}
