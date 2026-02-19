//! Ontological constants for Tellium Federation Core
//!
//! These constants define the philosophical and naming foundation
//! of the Federation network.

/// The name of the decentralized world
pub const NETWORK_NAME: &str = "Tellium";

/// The cognitive fabric layer (SSAU, neural routing, swarm memory)
pub const FABRIC_LAYER: &str = "Tela";

/// The collective mind (federated learning, adaptive strategies)
pub const COLLECTIVE_MIND: &str = "Nous Federis";

/// Reputation token (non-transferable social capital)
pub const REPUTATION_TOKEN: &str = "Credentia";

/// Economic token (transferable currency)
pub const ECONOMIC_TOKEN: &str = "Meritum";

/// Full ontological statement
pub const ONTOLOGY: &str = "Federation exists as a Totum Organicum within Tellium, \
                            structured as Tela, governed by Nous Federis, \
                            valued through Credentia and Meritum.";

/// Network version
pub const VERSION: &str = "1.0.0-alpha";

/// Project tagline
pub const TAGLINE: &str = "Decentralized Censorship Resistance through Collective Intelligence";

/// Latin motto
pub const MOTTO_LATIN: &str = "In Tellium Veritas, In Tela Sapientia, In Nous Libertas";

/// Motto translation
pub const MOTTO_EN: &str = "In Tellium - Truth, In Tela - Wisdom, In Nous - Freedom";

// ═══════════════════════════════════════════════════════════════
// Domain structure
// ═══════════════════════════════════════════════════════════════

/// Base domain for the network
pub const BASE_DOMAIN: &str = "tellium.network";

/// Tela (cognitive fabric) subdomain
pub const TELA_DOMAIN: &str = "tela.tellium.network";

/// Nous Federis subdomain
pub const NOUS_DOMAIN: &str = "nous.tellium.network";

/// Credentia (reputation) explorer
pub const CREDENTIA_DOMAIN: &str = "credentia.tellium.network";

/// Meritum (economics) portal
pub const MERITUM_DOMAIN: &str = "meritum.tellium.network";

/// DAO governance portal
pub const DAO_DOMAIN: &str = "dao.tellium.network";

/// API endpoint
pub const API_DOMAIN: &str = "api.tellium.network";

/// Documentation site
pub const DOCS_DOMAIN: &str = "docs.tellium.network";

/// Network explorer
pub const EXPLORER_DOMAIN: &str = "explorer.tellium.network";

// ═══════════════════════════════════════════════════════════════
// Ontological role names
// ═══════════════════════════════════════════════════════════════

/// Founding Fathers - original creators with veto power
pub const ROLE_FOUNDING_FATHERS: &str = "Founding Fathers";

/// Elders - high Credentia advisory council
pub const ROLE_ELDERS: &str = "Elders";

/// Citizens - full voting rights
pub const ROLE_CITIZENS: &str = "Citizens";

/// Ghosts - limited rights, low Credentia
pub const ROLE_GHOSTS: &str = "Ghosts";

// ═══════════════════════════════════════════════════════════════
// Thresholds for roles
// ═══════════════════════════════════════════════════════════════

/// Minimum Credentia to be a Citizen
pub const CREDENTIA_CITIZEN_THRESHOLD: f64 = 10.0;

/// Minimum Credentia to be an Elder
pub const CREDENTIA_ELDER_THRESHOLD: f64 = 100.0;

/// Minimum Meritum stake for proposal submission
pub const MERITUM_PROPOSAL_STAKE: f64 = 10.0;

/// Minimum Meritum stake for voting
pub const MERITUM_VOTING_STAKE: f64 = 1.0;

// ═══════════════════════════════════════════════════════════════
// Display functions
// ═══════════════════════════════════════════════════════════════

/// Print the main Tellium banner
pub fn print_banner() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║              TELLIUM FEDERATION CORE                      ║");
    println!("║                                                            ║");
    println!("║  Totum Organicum • Tela • Nous Federis                    ║");
    println!("║  Credentia • Meritum                                      ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("  {}", TAGLINE);
    println!("  {}", MOTTO_EN);
    println!();
}

/// Print the ontological statement
pub fn print_ontology() {
    println!("{}", ONTOLOGY);
}

/// Get role name based on Credentia
pub fn get_role_by_credentia(credentia: f64) -> &'static str {
    if credentia >= CREDENTIA_ELDER_THRESHOLD {
        ROLE_ELDERS
    } else if credentia >= CREDENTIA_CITIZEN_THRESHOLD {
        ROLE_CITIZENS
    } else {
        ROLE_GHOSTS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_thresholds() {
        assert_eq!(get_role_by_credentia(5.0), ROLE_GHOSTS);
        assert_eq!(get_role_by_credentia(15.0), ROLE_CITIZENS);
        assert_eq!(get_role_by_credentia(150.0), ROLE_ELDERS);
    }

    #[test]
    fn test_domains() {
        assert!(TELA_DOMAIN.contains(BASE_DOMAIN));
        assert!(NOUS_DOMAIN.contains(BASE_DOMAIN));
        assert!(CREDENTIA_DOMAIN.contains(BASE_DOMAIN));
        assert!(MERITUM_DOMAIN.contains(BASE_DOMAIN));
    }
}
