//! Phase 3: Ethics & Device Rights
//!
//! Obsahuje:
//! - Ethics Aiki demo
//! - Device Rights Codex demo

/// Hlavní entry point pro Phase 3
pub async fn demo_phase3() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 3: Ethics & Device Rights Demo    ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_ethics_aiki_demo().await;
    run_device_rights_demo().await;
    
    println!("\n✅ Phase 3 Complete!");
}

pub async fn run_ethics_aiki_demo() {
    crate::run_ethics_aiki_demo().await;
}

pub async fn run_device_rights_demo() {
    crate::run_device_rights_demo().await;
}
