//! Phase 2: Cryptographic Core
//!
//! Obsahuje:
//! - ZKP & Vault demo
//! - ChaCha20 crypto demo
//! - Noise Protocol demo

/// Hlavní entry point pro Phase 2
pub async fn demo_phase2() {
    println!("\n╔════════════════════════════════════════════╗");
    println!("║  PHASE 2: Cryptographic Core Demo        ║");
    println!("╚════════════════════════════════════════════╝\n");
    
    run_vault_demo().await;
    run_crypto_demo().await;
    run_noise_demo().await;
    
    println!("\n✅ Phase 2 Complete!");
}

// Tyto funkce zůstávají v main.rs - jsou příliš dlouhé
// Vytvoříme jen wrappery
pub async fn run_vault_demo() {
    crate::run_vault_demo().await;
}

pub async fn run_crypto_demo() {
    crate::run_crypto_demo().await;
}

pub async fn run_noise_demo() {
    crate::run_noise_demo().await;
}
