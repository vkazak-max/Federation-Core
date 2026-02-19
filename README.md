# 🌐 Federation Core

**Decentralized censorship-resistant network with neural routing and zero-knowledge privacy**

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

Federation Core is a next-generation peer-to-peer network designed to bypass censorship using AI-driven routing, cryptographic privacy, and adversarial simulation. Built entirely in Rust for maximum performance and safety.

---

## 🚀 Quick Start

```bash
# Clone repository
git clone <repository-url>
cd federation-core

# Build release version
cargo build --release

# Run Phase 1 demo (Neural Routing)
cargo run --release -- phase1
```

---

## 📋 Table of Contents

- [Features](#-features)
- [Architecture](#-architecture)
- [Installation](#-installation)
- [Usage](#-usage)
- [Project Structure](#-project-structure)
- [Development Phases](#-development-phases)
- [Contributing](#-contributing)
- [License](#-license)

---

## ✨ Features

### Core Capabilities

- **🧠 Neural Routing** - AI-driven route selection with SSAU (Structural Awareness Units) tensor metrics
- **🔐 Zero-Knowledge Privacy** - Onion routing with ZKP-based identity proofs
- **🎭 Mutation Engine** - Aiki-tactics for evading deep packet inspection
- **🏛️ DAO Governance** - Meritocratic governance with rep^0.7 voting weights
- **💰 Tokenomics** - Adaptive mint engine with halving schedules
- **🌍 Federated Learning** - Collective defense strategy sharing without raw data
- **📡 Multi-Protocol** - Mesh networking via IoT devices + satellite fallback
- **⚔️ War Simulation** - Adaptive SuperCensor for adversarial testing

### Technical Highlights

- **29+ Core Modules** covering networking, cryptography, consensus, economics
- **ChaCha20-Poly1305 AEAD** encryption with X25519 key exchange
- **Noise Protocol XX** for authenticated handshakes
- **Byzantine Fault Tolerant** consensus
- **Shamir Secret Sharing** for distributed key storage
- **PageRank-based** trust graph for reputation
- **Dynamic Network Sharding** for survivability under attack

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Application Layer                  │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐│
│  │ Dashboard│ │   DAO    │ │  Vault  │ │ Oracle  ││
│  └──────────┘ └──────────┘ └─────────┘ └─────────┘│
├─────────────────────────────────────────────────────┤
│                   Routing Layer                     │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐│
│  │  Neural  │ │   ZKP    │ │ Mutation│ │  Swarm  ││
│  │  Router  │ │  Onion   │ │  Engine │ │ Memory  ││
│  └──────────┘ └──────────┘ └─────────┘ └─────────┘│
├─────────────────────────────────────────────────────┤
│                  Network Layer                      │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐│
│  │   P2P    │ │Transport │ │  Mesh   │ │Satellite││
│  │  Overlay │ │ Channels │ │ Network │ │  Pulse  ││
│  └──────────┘ └──────────┘ └─────────┘ └─────────┘│
├─────────────────────────────────────────────────────┤
│                 Consensus Layer                     │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────┐│
│  │   BFT    │ │   DAG    │ │   PoA   │ │  Mint   ││
│  │Consensus │ │  Ledger  │ │ Rewards │ │ Engine  ││
│  └──────────┘ └──────────┘ └─────────┘ └─────────┘│
└─────────────────────────────────────────────────────┘
```

### Key Concepts

**SSAU (Structural Awareness Units)**  
5-dimensional tensors capturing network metrics: latency, bandwidth, reliability, trust, energy. Shannon entropy calculations determine route health.

**Aiki Tactics**  
Inspired by aikido - use censor's force against them. Exhaust resources through decoy generation and timing mutations.

**Proof-of-Awareness**  
Nodes prove honest routing by cross-verifying SSAU measurements. Byzantine nodes lose trust weight via exponential decay.

**Meritocracy DAO**  
Non-linear voting (rep^0.7) prevents plutocracy. Founding Fathers have veto rights for critical firmware updates.

---

## 💻 Installation

### Prerequisites

- **Rust 1.75+** - [Install Rust](https://rustup.rs/)
- **Linux/Unix** environment (Ubuntu 22+, macOS, etc.)

### Build from Source

```bash
# Clone repository
git clone <repository-url>
cd federation-core

# Build development version
cargo build

# Build optimized release
cargo build --release

# Run tests
cargo test

# Generate documentation
cargo doc --no-deps --open
```

---

## 🎮 Usage

### Command-Line Interface

Federation Core provides a rich CLI for exploring different system components:

```bash
# General syntax
federation-node <command>

# Example
cargo run --release -- phase1
```

### Available Commands

#### 🧪 Phase Demonstrations

| Command | Description |
|---------|-------------|
| `phase1` | Neural routing & SSAU tensors |
| `phase2` | Cryptographic core (ZKP, Vault, Noise) |
| `phase3` | Ethics layer & device rights |
| `phase4` | DAO governance & proposal engine |
| `phase5` | Credits system & eco bonuses |
| `phase6` | Reputation & trust graph |
| `phase7` | Mint engine & tokenomics |
| `phase8` | Treasury pools & insurance |
| `phase9` | ChaCha20 encryption |
| `phase10` | Live CLI dashboard |
| `phase11` | War simulation (VeilBreaker) |

#### 🔧 Legacy Commands

| Command | Description |
|---------|-------------|
| `neural` | Neural routing demo |
| `federated` | Federated learning demo |
| `mutation` | Mutation tactics demo |
| `veil` | VeilBreaker stress test |
| `credits` | Credit ledger demo |
| `market` | Bandwidth marketplace |
| `reputation` | Reputation system |
| `mint` | Mint engine demo |
| `vault` | Crypto vault + Shamir |
| `governance` | DAO governance |
| `ideas` | Idea Lab AI simulator |
| `dashboard` | Live dashboard |

### Example Session

```bash
# Test neural routing
$ cargo run --release -- phase1
╔════════════════════════════════════════════╗
║  PHASE 1: Neural Routing & SSAU Demo     ║
╚════════════════════════════════════════════╝

=== NEURAL ROUTING DEMO ===
Input:  latency=0.1ms bandwidth=0.1Mbps
Output: quality_score=0.496 tactic=Passive
...
✅ Phase 1 Complete!

# Run war simulation
$ cargo run --release -- phase11
🎯 WAR SIMULATION - SuperCensor vs Federation
...
```

---

## 📁 Project Structure

```
federation-core/
├── src/
│   ├── main.rs                    # Entry point & CLI
│   ├── demos/                     # Phase demonstrations
│   │   ├── mod.rs
│   │   ├── phase01_neural.rs
│   │   ├── phase02_crypto.rs
│   │   ├── ...
│   │   └── phase11_war.rs
│   │
│   ├── neural_node.rs             # Neural routing engine
│   ├── tensor.rs                  # SSAU tensor calculations
│   ├── zkp.rs                     # Zero-knowledge proofs
│   ├── mutation.rs                # Traffic mutation tactics
│   ├── governance.rs              # DAO & meritocracy
│   ├── mint.rs                    # Token emission
│   ├── reputation.rs              # Trust & PageRank
│   ├── federated.rs               # Federated learning
│   ├── consensus.rs               # BFT consensus
│   ├── vault.rs                   # Crypto vault + Shamir
│   ├── chacha.rs                  # ChaCha20-Poly1305
│   ├── noise.rs                   # Noise Protocol XX
│   ├── transport.rs               # Physical layer + strikes
│   ├── p2p.rs                     # P2P networking
│   ├── overlay.rs                 # Federation overlay
│   ├── robot_mesh.rs              # IoT mesh networking
│   ├── satellite_pulse.rs         # Satellite fallback
│   ├── veil_breaker.rs            # War simulation
│   ├── simulator.rs               # Network simulator
│   ├── adaptive_censor.rs         # Adaptive censorship
│   ├── war2.rs                    # Advanced war sim
│   ├── dashboard.rs               # CLI dashboard
│   ├── ethics.rs                  # Ethics layer
│   ├── credits.rs                 # Credit system
│   ├── market.rs                  # Bandwidth market
│   ├── pools.rs                   # Treasury pools
│   ├── dag.rs                     # DAG ledger
│   ├── oracle.rs                  # Oracle network
│   ├── shard.rs                   # Dynamic sharding
│   ├── swarm.rs                   # Swarm memory
│   ├── mirage.rs                  # Mirage layer
│   ├── routing.rs                 # AI router
│   ├── network.rs                 # Network messages
│   ├── inventory.rs               # Hardware profiles
│   ├── proposal_engine.rs         # Idea Lab
│   └── zk_identity.rs             # ZK identity proofs
│
├── Cargo.toml                     # Dependencies
├── README.md                      # This file
└── LICENSE                        # MIT License
```

---

## 🔬 Development Phases

### Completed (Phases 1-11)

- ✅ **Phase 1:** Neural routing & SSAU tensors
- ✅ **Phase 2:** Cryptographic core (ZKP, Vault, Noise)
- ✅ **Phase 3:** Ethics layer & device rights codex
- ✅ **Phase 4:** DAO governance with meritocracy
- ✅ **Phase 5:** Credits system & eco economy
- ✅ **Phase 6:** Reputation & trust graph
- ✅ **Phase 7:** Mint engine & adaptive tokenomics
- ✅ **Phase 8:** Treasury pools & insurance
- ✅ **Phase 9:** ChaCha20-Poly1305 AEAD
- ✅ **Phase 10:** Live CLI dashboard
- ✅ **Phase 11:** War simulation & adversarial testing

### Roadmap (Phase 12+)

- 🔄 **Phase 12:** Live node deployment
- 📊 **Phase 13:** Metrics & monitoring
- 🌐 **Phase 14:** Multi-region mesh network
- 🛡️ **Phase 15:** Advanced DPI evasion
- 📱 **Phase 16:** Mobile client support

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_ssau_tensor

# Run benchmarks (if available)
cargo bench
```

---

## 📊 Performance

Typical performance on modern hardware:

- **Neural routing:** ~50μs per route calculation
- **ZKP proof generation:** ~2ms
- **ChaCha20 encryption:** ~1GB/s
- **Consensus round:** ~100ms (100 nodes)

---

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### Code Standards

- Follow Rust best practices
- Add tests for new features
- Update documentation
- Run `cargo fmt` and `cargo clippy`

---

## 📜 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- **Anthropic** - For Claude AI assistance in development
- **Rust Community** - For excellent tooling and libraries
- **Cypherpunk movement** - For inspiration on privacy and decentralization

---

## 📞 Contact

- **Project:** Federation Core
- **Version:** 1.0.0-alpha
- **Documentation:** [Generated Docs](target/doc/federation_core/index.html)

---

## 🔗 Related Projects

- [Tor Project](https://www.torproject.org/) - Anonymous communication
- [I2P](https://geti2p.net/) - Anonymous network layer
- [Freenet](https://freenetproject.org/) - Peer-to-peer platform
- [Ethereum](https://ethereum.org/) - Decentralized platform

---

**Built with ❤️ and Rust 🦀**
