//! Phase 8: Pools & Treasury
//!
//! Obsahuje:
//! - Swarm Treasury demo
//! - Insurance pools demo

/// Hlavní entry point pro Phase 8
pub async fn demo_phase8() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 8: Pools & Treasury Demo          ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_pools_demo().await;
    
    println!("\n✅ Phase 8 Complete!");
}

pub async fn run_pools_demo() {
    crate::run_pools_demo().await;
}
