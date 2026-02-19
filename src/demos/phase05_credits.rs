//! Phase 5: Credits & Eco Economy
//!
//! Obsahuje:
//! - Credits demo
//! - Eco bonuses demo
//! - Bandwidth market demo

/// Hlavní entry point pro Phase 5
pub async fn demo_phase5() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 5: Credits & Eco Economy Demo     ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_credits_demo().await;
    run_eco_demo().await;
    run_market_demo().await;
    
    println!("\n✅ Phase 5 Complete!");
}

pub async fn run_credits_demo() {
    crate::run_credits_demo().await;
}

pub async fn run_eco_demo() {
    crate::run_eco_demo().await;
}

pub async fn run_market_demo() {
    crate::run_market_demo().await;
}
