//! Phase 7: Mint & Tokenomics
//!
//! Obsahuje:
//! - Mint engine demo
//! - Adaptive mint demo

/// Hlavní entry point pro Phase 7
pub async fn demo_phase7() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 7: Mint & Tokenomics Demo         ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_mint_demo().await;
    run_adaptive_mint_demo().await;
    
    println!("\n✅ Phase 7 Complete!");
}

pub async fn run_mint_demo() {
    crate::run_mint_demo().await;
}

pub async fn run_adaptive_mint_demo() {
    crate::run_adaptive_mint_demo().await;
}
