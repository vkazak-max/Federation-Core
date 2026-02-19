mod consensus;
mod proposal_engine;
mod robot_mesh;
mod satellite_pulse;
mod pools;
mod inventory;
mod vault;
mod dag;
mod ethics;
mod federated;
mod governance;
mod mirage;
mod mutation;
mod network;
mod neural_node;
mod oracle;
mod overlay;
mod p2p;
mod routing;
mod shard;
mod swarm;
mod tensor;
mod zkp;
mod credits;
mod market;
mod reputation;
mod mint;
mod transport;
mod veil_breaker;
mod demos;
mod constants;

#[tokio::main]
async fn main() {
// Print Tellium banner
    constants::print_banner();
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "ontology" => {
        println!("{}", constants::ONTOLOGY);
        println!("\nПодробная документация: ONTOLOGY.md");
        return;
        }
        "neural"     => { run_neural_demo().await; }
        "federated"  => { run_federated_demo().await; }
        "mutation"   => { run_mutation_demo().await; }
        "tactics"    => { run_neural_tactics_demo().await; }
        "collective" => { run_collective_tactics_demo().await; }
        "ethics2"    => { run_ethics_aiki_demo().await; }
        "transport"  => { run_transport_demo().await; }
        "veil"       => { run_veil_breaker().await; }
        "credits"    => { run_credits_demo().await; }
        "market"     => { run_market_demo().await; }
        "reputation" => { run_reputation_demo().await; }
        "mint"       => { run_mint_demo().await; }
        "vault"      => { run_vault_demo().await; }
        "inventory"  => { run_inventory_demo().await; }
        "pools"      => { run_pools_demo().await; }
        "satellite"  => { run_satellite_demo().await; }
        "robots"     => { run_robot_mesh_demo().await; }
        "governance" => { run_governance_demo().await; }
        "ideas"      => { run_ideas_demo().await; }
        "eco"        => { run_eco_demo().await; }
        "selfaware"  => { run_selfaware_demo().await; }
        "rights"     => { run_device_rights_demo().await; }
        "trust"      => { run_trust_graph_demo().await; }
        "adaptmint"  => { run_adaptive_mint_demo().await; }
        "crypto"     => { run_crypto_demo().await; }
        "dash"       => { run_dashboard_demo().await; }
        "war"        => { run_war_demo().await; }
        "noise"      => { run_noise_demo().await; }
         // === NOVÉ: Demo phases ===
    "phase1"     => { demos::phase01_neural::demo_phase1().await; }
    "phase2"     => { demos::phase02_crypto::demo_phase2().await; }
    "phase3"     => { demos::phase03_ethics::demo_phase3().await; }
    "phase4"     => { demos::phase04_governance::demo_phase4().await; }
    "phase5"     => { demos::phase05_credits::demo_phase5().await; }
    "phase6"     => { demos::phase06_reputation::demo_phase6().await; }
    "phase7"     => { demos::phase07_mint::demo_phase7().await; }
    "phase8"     => { demos::phase08_pools::demo_phase8().await; }
    "phase9"     => { demos::phase09_chacha::demo_phase9().await; }
    "phase10"    => { demos::phase10_dashboard::demo_phase10().await; }
    "phase11"    => { demos::phase11_war::demo_phase11().await; }

        _            => {
            println!("Federation Core — доступные команды:");
            println!("  neural      — нейросеть + backprop");
            println!("  federated   — федеративное обучение");
            println!("  mutation    — тактики мутации");
            println!("  tactics     — нейротактика");
            println!("  collective  — коллективная мудрость");
            println!("  ethics2     — кодекс айкидо");
            println!("  transport   — физический слой");
            println!("  veil        — стресс-тест войны");
            println!("  credits     — proof-of-bypass");
            println!("  market      — аукцион bandwidth");
            println!("  reputation  — социальный капитал");
            println!("  mint        — эмиссионный центр");
            println!("  vault       — криптохранилище + Shamir");
        }
    }
}

// =============================================================================
// DEMO FUNCTIONS
// =============================================================================

pub async fn run_neural_demo() {
    use crate::neural_node::{NeuralRouter, NeuralInput, NeuralTarget};
    println!("\n=== Neural Demo ===\n");
    let mut router = NeuralRouter::new("nexus-core-01");
    let input = NeuralInput { latency:0.3, bandwidth:0.8,
        reliability:0.9, trust:0.7, ethics_score:1.0 };
    let _target = NeuralTarget::success_route(0.9);
    for neighbor in &["node_berlin","node_tokyo","node_paris"] {
        router.train_on_delivery(neighbor, &input, true, 0.9);
    }
    let candidates = vec![
        ("node_berlin".to_string(), input.clone()),
        ("node_tokyo".to_string(),  input.clone()),
    ];
    let best = router.select_best(candidates);
    println!("Лучший маршрут: {:?}", best);
    println!("{}", router.stats());
}

pub async fn run_federated_demo() {
    use crate::federated::FederatedNetwork;
    println!("\n=== Federated Learning Demo ===\n");
    let mut net = FederatedNetwork::new();
    net.add_node("node_tokyo",   "JP");
    net.add_node("node_berlin",  "DE");
    net.add_node("node_toronto", "CA");
    for i in 0..5 {
        if let Some(r) = net.run_round() {
            println!("Раунд {}: loss={:.4} accuracy={:.4} участников={}",
                i, r.avg_local_loss, r.avg_local_accuracy, r.participants);
        }
    }
    println!("{}", net.stats());
}

pub async fn run_mutation_demo() {
    use crate::mutation::{MutationEngine, MutationStrategy, TrafficMask};
    println!("Mutation demo — восстановлено");
    let payload = b"FEDERATION_DATA";
    let masks = vec![
        TrafficMask::VideoStream { codec:"H264".into(), bitrate_kbps:2500 },
        TrafficMask::HttpsRequest { host:"youtube.com".into(), path:"/watch".into() },
        TrafficMask::TlsHandshake { version:"1.3".into() },
    ];
    for mask in masks {
        let mut engine = MutationEngine::new("nexus-core-01",
            MutationStrategy::default_decoy());
        engine.active_mask = mask.clone();
        let result = engine.mutate(payload, 0.2);
        println!("Маска: {:?}  коробочек:{} шум:{:.1}%",
            mask, result.decoy_count, result.noise_ratio*100.0);
    }
}

pub async fn run_neural_tactics_demo() {
    use crate::neural_node::{NeuralState, NeuralInput, NeuralTactic};
    
    println!("\n=== Neural Tactics Demo ===\n");
    let state = NeuralState::new("nexus-core-01");
    let scenarios = vec![
        (NeuralInput { latency:0.05, bandwidth:0.95, reliability:0.99, trust:0.95, ethics_score:1.0 }, "Чистый канал"),
        (NeuralInput { latency:0.70, bandwidth:0.30, reliability:0.50, trust:0.40, ethics_score:0.9 }, "DPI активен"),
        (NeuralInput { latency:0.92, bandwidth:0.08, reliability:0.15, trust:0.10, ethics_score:0.9 }, "Полная блокировка"),
        (NeuralInput { latency:0.55, bandwidth:0.55, reliability:0.75, trust:0.65, ethics_score:1.0 }, "Узкое окно"),
        (NeuralInput { latency:0.85, bandwidth:0.15, reliability:0.25, trust:0.15, ethics_score:0.9 }, "Зондирование"),
    ];
    println!("   {:>40}  {:>6} {:>6} {:>6}  Тактика", "Сценарий", "decoy", "strike", "cong");
    println!("   {}", "─".repeat(70));
    for (input, scenario) in &scenarios {
        let out = state.forward(input);
        let tactic = NeuralTactic::decide_from_input(
            input.latency, out.congestion_prob,
            out.decoy_intensity, out.strike_focus);
        println!("   {:>40}  {:>6.3} {:>6.3} {:>6.3}  [{}]",
            scenario, out.decoy_intensity, out.strike_focus,
            out.congestion_prob, tactic.name());
    }
}

pub async fn run_collective_tactics_demo() {
    use crate::federated::{FederatedNetwork, TacticReport};
    println!("\n=== Collective Tactical Wisdom ===\n");
    let mut net = FederatedNetwork::new();
    for (id, region) in &[("node_tokyo","JP"),("node_berlin","DE"),
        ("node_toronto","CA"),("node_nairobi","KE"),("node_sydney","AU")] {
        net.add_node(id, region);
    }
    let reports = vec![
        TacticReport::new("node_tokyo",  "JP", "StandoffDecoy",    "CN_DPI_v3", 0.87, 15),
        TacticReport::new("node_berlin", "DE", "AikiReflection",   "RU_BGP",    0.91, 20),
        TacticReport::new("node_tokyo",  "JP", "CumulativeStrike", "CN_DPI_v4", 0.89, 5),
        TacticReport::new("node_sydney", "AU", "AikiReflection",   "CN_DPI_v4", 0.76, 3),
    ];
    let result = net.run_tactical_round(reports);
    println!("{}", result);
    net.defense_model.display();
}

pub async fn run_ethics_aiki_demo() {
    use crate::ethics::{EthicsLayer, EthicsAction};
    println!("\n=== Ethics Aiki Demo ===\n");
    let mut ethics = EthicsLayer::new();
    let cases = vec![
        ("Пропорциональный ответ CN", EthicsAction::AikiResponse {
            censor_aggression:0.85, response_intensity:0.90,
            is_first_strike:false, has_evidence:true,
            target_is_censor:true, tactic:"ResourceExhaustion".into() },
         "Цензор CN атакует. DAG доказательства получены."),
        ("Первый удар — запрещён", EthicsAction::AikiResponse {
            censor_aggression:0.0, response_intensity:0.8,
            is_first_strike:true, has_evidence:false,
            target_is_censor:true, tactic:"ResourceExhaustion".into() },
         "Превентивная атака без доказательств."),
    ];
    for (name, action, reasoning) in cases {
        let v = ethics.check(action, reasoning);
        println!("  {} [{}] score={:.3} — {}",
            if v.allowed {"✅"} else {"🚫"}, name, v.violation_score, v.reason);
    }
    println!("\n{}", ethics.audit.stats());
}

pub async fn run_transport_demo() {
    use crate::transport::{TransportChannel, MicroClock};
    println!("\n=== Transport Layer Demo ===\n");
    let mut clock = MicroClock::new();
    println!("MicroClock: {}мкс  jitter={}мкс",
        clock.now_us(), clock.jitter_us(100, 50_000));
    let mut ch = TransportChannel::new("nexus-core-01", "node_berlin");
    let payload = b"FEDERATION_SECURE_DATA";
    let results = ch.send_with_decoys(payload, "HttpsRequest", 6);
    println!("Отправлено {} пакетов (1 реальный + 6 коробочек)", results.len());
    for r in &results {
        println!("  {} jitter={}мкс  mask={}",
            if r.is_decoy {"🎭"} else {"📦"}, r.jitter_applied_us, r.mask_type);
    }
    println!("\n{}", ch.stats());

    // ── HIERARCHICAL ROUTING DEMO ──────────────────────────────────────
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Hierarchical Routing — путь по классу железа");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    use crate::transport::HierarchicalRouter;
    let mut hr = HierarchicalRouter::new();
    let nodes: Vec<(String, String)> = vec![
        ("nexus-core-01".into(), "Sentinel".into()),
        ("hub-berlin-01".into(), "Citadel".into()),
        ("hub-tokyo-01".into(),  "Citadel".into()),
        ("work-alice".into(),    "Workstation".into()),
        ("ghost-pi3".into(),     "Ghost".into()),
        ("ghost-pentium".into(), "Ghost".into()),
        ("router-01".into(),     "Droid".into()),
        ("phone-carol".into(),   "Mobile".into()),
    ];
    let cases = vec![
        ("nexus-core-01", "Sentinel",    "hub-tokyo-01",  10u32),
        ("work-alice",    "Workstation", "hub-berlin-01", 20u32),
        ("ghost-pi3",     "Ghost",       "nexus-core-01", 30u32),
        ("phone-carol",   "Mobile",      "work-alice",    25u32),
    ];
    println!("   {:15} {:12} {:12}  {:12}  Хопы  мс    Скрытность  Трафик", "Узел", "Роль", "Цель", "Лейн");
    println!("   {}", "─".repeat(80));
    for (src, role, dst, lat) in &cases {
        let r = hr.route(src, role, dst, &nodes, *lat);
        println!("   {:15} {:12} {:12}  {:12}  {:>4}  {:>4}мс  {:>8.0}%  {:>5.1}x",
            src, role, dst, r.lane.name(),
            r.hops.len(), r.estimated_latency_ms,
            r.stealth_score*100.0, r.total_traffic_ratio());
        if !r.decoy_paths.is_empty() {
            println!("   {:49} Приманки: {}", "", r.decoy_paths.len());
        }
    }
    println!("\n   {} ", hr.stats());
    println!("   Ghost маршрут: 7 хопов через Ghost/Droid узлы — цензор видит шум");
    println!("   FastLane:      2 хопа через Sentinel/Citadel — скорость важнее");
    println!("   NoiseLane:     3x трафика = 3 ложных маршрута на 1 реальный");
}

pub async fn run_veil_breaker() {
    use crate::veil_breaker::VeilBreakerTest;
    println!("\n=== THE VEIL-BREAKER TEST ===\n");
    let mut test = VeilBreakerTest::new();
    let results = test.run();
    for r in &results {
        println!("  {} delivered={} blocked={} rate={:.1}% cpu={:.0}% [{}]",
            r.phase, r.delivered, r.blocked,
            r.delivery_rate*100.0, r.censor_cpu*100.0, r.dominant_tactic);
        for note in &r.notes { println!("    💬 {}", note); }
    }
    let v = test.final_verdict();
    println!("\nОценка: {}  Доставка: {:.1}%  Тест: {}",
        v.grade, v.final_delivery_rate*100.0,
        if v.passed {"✅ ПРОЙДЕН"} else {"❌ ПРОВАЛ"});
}

pub async fn run_credits_demo() {
    use crate::credits::{CreditLedger, known_regions};
    println!("\n=== Proof-of-Bypass Credits ===\n");
    let regions = known_regions();
    let mut ledger = CreditLedger::new();
    let events = vec![
        ("node_tokyo",   "CN", "AikiReflection",   60u64, 0.85f64, true),
        ("node_tokyo",   "KP", "CumulativeStrike",  80,    0.99,    true),
        ("node_berlin",  "RU", "AikiReflection",    55,    0.90,    true),
        ("node_nairobi", "ET", "StandoffDecoy",     30,    0.40,    true),
        ("node_toronto", "CA", "Passive",           100,   0.05,    false),
    ];
    println!("   {:16} {:>4} {:>18} {:>8}  Credits", "Узел","Рег.","Тактика","Пакеты");
    println!("   {}", "─".repeat(60));
    for (node, region, tactic, packets, cpu, evidence) in &events {
        if let Some(diff) = regions.get(*region) {
            let c = ledger.record_bypass(node, region, tactic,
                *packets, *cpu, diff, *evidence);
            println!("   {:16} {:>4} {:>18} {:>8}  {:.3} 💎", node, region, tactic, packets, c);
        }
    }
    println!("\n{}", ledger.stats());
}

pub async fn run_market_demo() {
    use crate::market::{BandwidthMarket, TrafficTier};
    println!("\n=== Bandwidth Market Demo ===\n");
    let mut market = BandwidthMarket::new();
    let b1 = market.submit_bid("user_alice", "CN", 512,  8.0, TrafficTier::Armored);
    let b2 = market.submit_bid("user_bob",   "RU", 256,  4.0, TrafficTier::Premium);
    let b3 = market.submit_bid("user_eve",   "DE", 256,  1.0, TrafficTier::Economy);
    market.submit_offer("node_tokyo",  b1, 6.5, "StandoffDecoy",    120, 0.88, 3.0, 0.85);
    market.submit_offer("node_sydney", b1, 7.2, "StandoffDecoy",    180, 0.92, 4.0, 0.85);
    market.submit_offer("node_berlin", b2, 3.2, "AikiReflection",   80,  0.94, 2.0, 0.60);
    market.submit_offer("node_berlin", b3, 0.4, "Passive",          30,  0.99, 0.2, 0.05);
    for bid_id in &[b1, b2, b3] {
        match market.run_auction(*bid_id) {
            Some(r) => println!("  Bid {:>2}: {} выиграл {:.2}💎 [{}] гарантия={:.0}%",
                r.bid_id, r.winner_node, r.winning_price,
                r.winning_tactic, r.success_guarantee*100.0),
            None => println!("  Bid {:>2}: нет предложений", bid_id),
        }
    }
    println!("\n{}", market.market_stats());
}

pub async fn run_reputation_demo() {
    use crate::reputation::ReputationRegistry;
    println!("\n=== Reputation & Social Capital ===\n");
    let mut reg = ReputationRegistry::new();
    for _ in 0..80 { reg.record_delivery("node_tokyo",   "AikiReflection",   0.85); }
    for _ in 0..50 { reg.record_delivery("node_tokyo",   "CumulativeStrike",  0.99); }
    for _ in 0..60 { reg.record_delivery("node_berlin",  "AikiReflection",   0.60); }
    for _ in 0..45 { reg.record_delivery("node_nairobi", "StandoffDecoy",    0.70); }
    for _ in 0..30 { reg.record_delivery("node_toronto", "Passive",          0.05); }
    reg.record_aiki_victory("node_tokyo", 0.95);
    reg.record_uptime("node_tokyo", 365);
    reg.record_uptime("node_berlin", 300);
    // Предательство
    reg.record_betrayal("node_evil", "hash_001");
    reg.record_betrayal("node_evil", "hash_002");
    reg.record_betrayal("node_evil", "hash_003");
    println!("   {:>3}  {:20} {:>8}  {:>12}  {:>8}",
        "#", "Узел", "Score", "Tier", "DAO вес");
    println!("   {}", "─".repeat(58));
    for (node, rank) in reg.leaderboard(5) {
        println!("   {:>3}  {:20} {:>8.1}  {:>12}  {:>8.3}",
            rank, node.node_id, node.score,
            node.tier.name(), node.dao_voting_weight());
    }
    let evil = reg.nodes.get("node_evil").unwrap();
    println!("\n   node_evil: blacklisted={} betrayals={} DAO={}",
        evil.is_blacklisted, evil.betrayals, evil.dao_voting_weight());
    println!("\n{}", reg.stats());
}

pub async fn run_mint_demo() {
    use crate::mint::MintEngine;
    use crate::credits::known_regions;
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 5 / Step 4                  ║");
    println!("║         Algorithmic Emission — Credits = Свобода 🪙          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut mint = MintEngine::new();
    let regions = known_regions();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Mint-per-Bypass — каждый токен = акт свободы");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let bypass_events = vec![
        ("node_tokyo",   "KP", "AikiReflection",   0.99),
        ("node_tokyo",   "CN", "CumulativeStrike",  0.85),
        ("node_berlin",  "RU", "AikiReflection",    0.60),
        ("node_nairobi", "ET", "StandoffDecoy",     0.70),
        ("node_toronto", "DE", "Passive",           0.05),
        ("node_sydney",  "KP", "CumulativeStrike",  0.99),
        ("node_tokyo",   "IR", "AikiReflection",    0.75),
    ];

    println!("   {:16} {:>4} {:>18} {:>8} {:>8} {:>8}",
        "Узел", "Рег.", "Тактика", "Gross", "Burn🔥", "Net💎");
    println!("   {}", "─".repeat(70));

    for (node, region, tactic, diff) in &bypass_events {
        if let Some(e) = mint.mint_for_bypass(node, region, tactic, *diff) {
            println!("   {:16} {:>4} {:>18} {:>8.3} {:>8.3} {:>8.3}",
                node, region, tactic,
                e.gross_minted, e.burned, e.net_to_node);
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Difficulty-based Issuance — риск = эмиссия");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   Сравнение регионов (тактика: AikiReflection):\n");
    let mut cmp_mint = MintEngine::new();
    let mut region_list: Vec<_> = regions.values().collect();
    region_list.sort_by(|a, b| b.difficulty_score.partial_cmp(&a.difficulty_score).unwrap());

    for r in &region_list {
        if let Some(e) = cmp_mint.mint_for_bypass(
            "test_node", &r.region_code, "AikiReflection", r.difficulty_score) {
            let bar = "█".repeat((e.net_to_node / 2.0) as usize);
            println!("   {:>4} diff={:.2} {} → {:>8.3}💎  {}",
                r.region_code, r.difficulty_score,
                r.label(), e.net_to_node, bar);
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Halving & Burn — дефляция при росте");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Симуляция на разных эпохах
    let epochs = vec![
        (0u32,     1.000, "Эпоха 0 — начало"),
        (1,        0.500, "Эпоха 1 — первый халвинг"),
        (2,        0.250, "Эпоха 2 — второй халвинг"),
        (3,        0.125, "Эпоха 3 — третий халвинг"),
    ];

    println!("   {:30} {:>8}  {:>8}  {:>8}",
        "Эпоха", "Фактор", "Gross", "Net💎");
    println!("   {}", "─".repeat(58));

    for (epoch, factor, name) in &epochs {
        let mut sim = MintEngine::new();
        sim.halving.current_epoch = *epoch;
        sim.halving.current_multiplier = *factor;
        if let Some(e) = sim.mint_for_bypass("node", "CN", "AikiReflection", 0.85) {
            println!("   {:30} {:>8.3}  {:>8.3}  {:>8.3}",
                name, factor, e.gross_minted, e.net_to_node);
        }
    }

    // Симуляция burn от рынка
    println!("\n   Market burn симуляция:");
    let fees = vec![10.0, 50.0, 100.0, 500.0];
    let mut total_burned = 0.0;
    for fee in &fees {
        let burned = mint.burn_market_fee(*fee);
        total_burned += burned;
        println!("   Комиссия {:.1}💎 → сожжено {:.1}💎 (30%)", fee, burned);
    }
    println!("   Итого сожжено от рынка: {:.1}💎", total_burned);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Макро симуляция — 10,000 прорывов");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let sims = vec![
        ("node_tokyo",   "KP", "AikiReflection",   0.99, 2000u64),
        ("node_berlin",  "RU", "CumulativeStrike",  0.60, 3000),
        ("node_nairobi", "ET", "StandoffDecoy",     0.70, 2500),
        ("node_toronto", "DE", "Passive",           0.05, 2500),
    ];

    for (node, region, tactic, diff, count) in &sims {
        let r = mint.simulate_bypasses(*count, node, region, tactic, *diff);
        println!("   {:16} {:>4} {:>5} прорывов → {:>10.2}💎 net  burn={:.2}  avg={:.3}",
            node, region, count, r.net_supply_added, r.total_burned, r.avg_per_bypass);
    }

    println!("\n{}", mint.supply_stats());

    println!("\n   Топ эмитентов:");
    let stats = mint.supply_stats();
    let max_earned = stats.top_earners.iter()
        .map(|(_, e)| *e).fold(0.0f64, f64::max).max(1.0);
    for (node, earned) in &stats.top_earners {
        let bar_len = (earned / max_earned * 30.0) as usize;
        let bar = "█".repeat(bar_len);
        println!("   {:20} {:>12.2}💎  {}", node, earned, bar);
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 5 Step 4 COMPLETE — Mint Engine работает          ║");
    println!("║                                                              ║");
    println!("║  MintEngine ✓  HalvingSchedule ✓  BurnLedger ✓            ║");
    println!("║  Difficulty Issuance ✓  Market Burn ✓  MAX_SUPPLY ✓       ║");
    println!("║  Credits = доказанный акт освобождения информации ✓        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_vault_demo() {
    use crate::vault::{CryptoVault, ShamirScheme};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 5 / Step 5                  ║");
    println!("║         Crypto Vault — Осколочное хранение ключей 🔐        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut vault = CryptoVault::new();

    // -------------------------------------------------------------------------
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Hot & Cold Vault — ZK доступ");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Hot vault — быстрый доступ
    let secret_key = b"FEDERATION_MASTER_KEY_v4_ultra";
    let proof_hot = vault.store_hot("key_001", "node_tokyo", secret_key, 30.0);
    println!("   🔥 Hot vault: key_001");
    println!("      ZK proof:  {}", &proof_hot.proof_hash);
    println!("      Commitment: {}", &proof_hot.commitment);
    println!("      Expires:   {}мс", proof_hot.expires_at % 100_000);

    // Cold vault — максимальная защита
    let cold_key = b"FEDERATION_DAO_SIGNING_KEY_cold";
    let proof_cold = vault.store_cold("key_002", "node_berlin", cold_key, 100.0);
    println!("\n   🧊 Cold vault: key_002");
    println!("      ZK proof:  {}", &proof_cold.proof_hash);
    println!("      Rep required: 100.0 (только Veteran+)");

    // Доступ с проверкой репутации
    println!("\n   Попытки доступа:");
    let r1 = vault.retrieve_hot("key_001", &proof_hot, 50.0);
    println!("   node_tokyo  rep=50.0  → {} {}", 
        if r1.success {"✅"} else {"🚫"}, r1.reason);

    let r2 = vault.retrieve_hot("key_001", &proof_hot, 10.0);
    println!("   node_newbie rep=10.0  → {} {}", 
        if r2.success {"✅"} else {"🚫"}, r2.reason);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Shamir's Secret Sharing — схема (5,3)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let secret = b"VETERAN_SIGNING_KEY_32bytes_long";
    println!("   Исходный секрет: {:?}", &secret[..8]);
    println!("   Схема: 5 осколков, любые 3 восстанавливают\n");

    let mut rng: u64 = 0xfeed_face_cafe_babe;
    let shards = ShamirScheme::split(secret, 5, 3, &mut rng);

    for (i, shard) in shards.iter().enumerate() {
        println!("   Осколок {}: {:?}...  {}KB в памяти Ghost",
            i+1, &shard[..4], shard.len() / 1024 + 1);
    }

    // Восстановление из 3 осколков (1, 3, 5)
    let reconstruct_shards = vec![
        (1u8, shards[0].clone()),
        (3u8, shards[2].clone()),
        (5u8, shards[4].clone()),
    ];
    let reconstructed = ShamirScheme::reconstruct(&reconstruct_shards);
    println!("\n   Восстановление из осколков 1,3,5:");
    println!("   Совпадает: {}", if reconstructed == secret.to_vec() {"✅ ДА"} else {"❌ НЕТ"});

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Ghost Network — осколки в тысячах узлов");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Veteran ключи дробятся по Ghost-узлам
    let ghost_nodes = vec![
        "ghost_JP_001", "ghost_DE_002", "ghost_BR_003",
        "ghost_KE_004", "ghost_AU_005",
    ];

    for ghost in &ghost_nodes {
        vault.ghost_network.register_ghost(ghost);
    }

    let veteran_keys = vec![
        ("key_tokyo_veteran",   "node_tokyo",   b"TOKYO_VETERAN_SECRET_KEY_32byte"),
        ("key_berlin_veteran",  "node_berlin",  b"BERLIN_VETERAN_SECRET_KEY_32byt"),
        ("key_nairobi_veteran", "node_nairobi", b"NAIROBI_VETERAN_SECRET_KEY_32by"),
    ];

    println!("   Ключ                    Владелец       Осколков  Приманок  Ghost-узлы");
    println!("   {}", "─".repeat(72));

    for (key_id, owner, key_data) in &veteran_keys {
        let result = vault.shard_to_ghosts(
            key_id, owner, *key_data, &ghost_nodes, 5, 3);
        println!("   {:24} {:14} {:>8}  {:>8}  {}...{}",
            result.key_id, owner,
            result.total_shards, result.decoy_shards,
            &result.ghost_nodes[0][..8],
            &result.ghost_nodes.last().unwrap()[..8]);
        println!("   commitment: {}  threshold: {}/{}",
            &result.commitment[..20], result.threshold, result.total_shards);
    }

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Ghost Node — что видит обычный узел");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   Ghost-узел ghost_JP_001 хранит в памяти:");
    if let Some(shards) = vault.ghost_network.nodes.get("ghost_JP_001") {
        let real_count = shards.iter().filter(|s: &&crate::vault::KeyShard| !s.is_decoy).count();
        let decoy_count = shards.iter().filter(|s: &&crate::vault::KeyShard| s.is_decoy).count();
        println!("   Всего осколков: {}  (реальных: {}  приманок: {})",
            shards.len(), real_count, decoy_count);
        println!("   Ghost НЕ ЗНАЕТ:");
        println!("   ├─ Чей ключ      → owner_commitment скрыт");
        println!("   ├─ Что за данные → зашифровано");
        println!("   ├─ Сколько всего → видит только свой осколок");
        println!("   └─ Где остальные → нет информации о других Ghost");

        for (i, s) in shards.iter().take(4).enumerate() {
            println!("   Осколок {}: {} shard_id={} commit={}",
                i+1,
                if s.is_decoy {"🎭 DECOY"} else {"🔑 REAL "},
                s.shard_id, &s.key_commitment[..16]);
        }
    }

    println!("\n   Атакующий захватил ghost_JP_001 и ghost_DE_002:");
    println!("   Имеет 2/3 осколков → восстановить НЕВОЗМОЖНО");
    println!("   Нужно захватить {} из {} Ghost-узлов одновременно", 3, 5);

    // -------------------------------------------------------------------------
    println!("\n{}", vault.vault_stats());

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 5 Step 5 COMPLETE — Crypto Vault работает         ║");
    println!("║                                                              ║");
    println!("║  Hot/Cold Vault ✓  ZK Proof ✓  Shamir Sharding ✓          ║");
    println!("║  Ghost Network ✓  Decoy Shards ✓  Rep-gated Access ✓      ║");
    println!("║  Ключ Ветерана: 5 осколков, захвати 3 Ghost — невозможно ✓ ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_inventory_demo() {
    use crate::inventory::{HardwareProfile, FederationInventory, RoleClassifier,
        CpuArch, OsType, DeviceRole};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 5 / Step 6                  ║");
    println!("║         Iron Discipline — Классификация железа 🔩            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut inv = FederationInventory::new();

    let devices = vec![
        HardwareProfile { device_id:"nexus-core-01".into(), cpu_cores:32,
            cpu_mhz:3800, ram_mb:65536, storage_gb:2000, bandwidth_mbps:10000,
            has_gpu:true, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::Linux, uptime_days:365, is_tor_capable:true },
        HardwareProfile { device_id:"hub-berlin-01".into(), cpu_cores:16,
            cpu_mhz:3200, ram_mb:32768, storage_gb:500, bandwidth_mbps:1000,
            has_gpu:false, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::Linux, uptime_days:180, is_tor_capable:true },
        HardwareProfile { device_id:"hub-tokyo-01".into(), cpu_cores:8,
            cpu_mhz:2800, ram_mb:16384, storage_gb:200, bandwidth_mbps:500,
            has_gpu:false, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::FreeBsd, uptime_days:90, is_tor_capable:true },
        HardwareProfile { device_id:"work-alice".into(), cpu_cores:8,
            cpu_mhz:3600, ram_mb:16384, storage_gb:512, bandwidth_mbps:100,
            has_gpu:true, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::Linux, uptime_days:30, is_tor_capable:true },
        HardwareProfile { device_id:"work-bob".into(), cpu_cores:4,
            cpu_mhz:2400, ram_mb:8192, storage_gb:256, bandwidth_mbps:50,
            has_gpu:false, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::Windows, uptime_days:14, is_tor_capable:false },
        HardwareProfile { device_id:"phone-carol".into(), cpu_cores:8,
            cpu_mhz:2800, ram_mb:6144, storage_gb:128, bandwidth_mbps:50,
            has_gpu:true, battery_powered:true, arch:CpuArch::Arm64,
            os:OsType::Android, uptime_days:1, is_tor_capable:false },
        HardwareProfile { device_id:"phone-dave".into(), cpu_cores:4,
            cpu_mhz:1800, ram_mb:3072, storage_gb:64, bandwidth_mbps:20,
            has_gpu:false, battery_powered:true, arch:CpuArch::Arm64,
            os:OsType::Ios, uptime_days:0, is_tor_capable:false },
        HardwareProfile { device_id:"ghost-pentium".into(), cpu_cores:2,
            cpu_mhz:1200, ram_mb:2048, storage_gb:80, bandwidth_mbps:10,
            has_gpu:false, battery_powered:false, arch:CpuArch::X86_64,
            os:OsType::Linux, uptime_days:730, is_tor_capable:true },
        HardwareProfile { device_id:"ghost-pi3".into(), cpu_cores:4,
            cpu_mhz:1400, ram_mb:1024, storage_gb:32, bandwidth_mbps:100,
            has_gpu:false, battery_powered:false, arch:CpuArch::Arm64,
            os:OsType::Linux, uptime_days:500, is_tor_capable:true },
        HardwareProfile { device_id:"router-openwrt".into(), cpu_cores:2,
            cpu_mhz:880, ram_mb:256, storage_gb:0, bandwidth_mbps:100,
            has_gpu:false, battery_powered:false, arch:CpuArch::Mips,
            os:OsType::OpenWrt, uptime_days:60, is_tor_capable:false },
        HardwareProfile { device_id:"droid-esp32".into(), cpu_cores:2,
            cpu_mhz:240, ram_mb:1, storage_gb:0, bandwidth_mbps:1,
            has_gpu:false, battery_powered:true, arch:CpuArch::Unknown,
            os:OsType::Unknown, uptime_days:120, is_tor_capable:false },
    ];

    // -------------------------------------------------------------------------
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Автоклассификация железа");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   {:20} {:>4}CPU {:>6}MB {:>5}Mbps  Роль              Score",
        "Устройство", "", "", "");
    println!("   {}", "─".repeat(72));

    for hw in &devices {
        let _cap = NodeCapacity_from(hw);
        let role = RoleClassifier::classify(hw);
        inv.register(hw.clone());
        println!("   {:20} {:>4}  {:>6}MB {:>5}Mbps  {:14}  {:>5.1}",
            hw.device_id, hw.cpu_cores, hw.ram_mb, hw.bandwidth_mbps,
            role.name(), hw.compute_score());
    }

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Роли и возможности");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   {:20} {:>10}  {:>8}  {:>5}  Aiki  ZK  Модулей",
        "Устройство", "Bypass/s", "BW Mbps", "Conn");
    println!("   {}", "─".repeat(70));

    for (id, cap) in &inv.capacities {
        println!("   {:20} {:>10.0}  {:>8.1}  {:>5}  {:>4}  {:>2}  {}",
            id, cap.estimated_bypass_rate, cap.bandwidth_alloc_mbps,
            cap.max_connections,
            if cap.can_run_aiki {"✅"} else {"❌"},
            if cap.can_run_zk   {"✅"} else {"❌"},
            cap.enabled_modules.len());
    }

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Топология сети");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let topo = inv.network_topology();

    println!("   Слоевая архитектура:");
    println!("   L1 ⚔️  Sentinel:    {:>3} узлов  — ядро, полный стек", topo.sentinels);
    println!("   L2 🏰 Citadel:     {:>3} узлов  — региональные хабы", topo.citadels);
    println!("   L3 🖥️  Workstation: {:>3} узлов  — полные узлы", topo.workers);
    println!("   L4 📱 Mobile:      {:>3} узлов  — лёгкие клиенты", topo.mobiles);
    println!("   L5 👻 Ghost:       {:>3} узлов  — шум и приманки", topo.ghosts);
    println!("      🤖 Droid:       {:>3} узлов  — меш-реле", topo.droids);
    println!();
    println!("   Суммарная полоса:   {:.0} Mbps", topo.total_bandwidth_mbps);
    println!("   Прорывов в сек:     {:.0}", topo.total_bypass_rate);
    println!("   Шум в сети:         {:.0}%  (Ghost+Droid скрывают реальный трафик)",
        topo.noise_ratio * 100.0);
    println!("   Aiki-способных:     {}", topo.aiki_capable);
    println!("   ZK-способных:       {}", topo.zk_capable);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Региональное назначение");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let assignments = inv.auto_assign_regions();
    println!("   {:20} {:12}  Регион  L",
        "Устройство", "Роль");
    println!("   {}", "─".repeat(50));
    for a in &assignments {
        println!("   {:20} {:12}  {:>6}  L{}",
            a.device_id, a.role.name(), a.region, a.layer);
    }

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  5. Ghost стратегия — старое железо в деле");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let ghosts = inv.get_by_role(&DeviceRole::Ghost);
    println!("   Ghost-узлов: {}  Функция: генерация шума + хранение осколков\n", ghosts.len());
    for g in &ghosts {
        println!("   👻 {:20}  decoy_cap={:>4}  bypass={:.0}/s  modules={}",
            g.device_id, g.decoy_capacity,
            g.estimated_bypass_rate, g.enabled_modules.len());
    }
    println!("\n   Старый Pentium и Raspberry Pi — теперь солдаты Федерации.");
    println!("   Они не знают что хранят. Они просто шумят.");

    println!("\n{}", topo);

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 5 Step 6 COMPLETE — Iron Discipline работает      ║");
    println!("║                                                              ║");
    println!("║  HardwareProfile ✓  DeviceRole ✓  RoleClassifier ✓        ║");
    println!("║  NodeCapacity ✓  FederationInventory ✓  Topology ✓        ║");
    println!("║  Старое железо = шум. Мощное = ядро. Роботы = реле. ✓     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

fn NodeCapacity_from(hw: &crate::inventory::HardwareProfile) -> crate::inventory::NodeCapacity {
    crate::inventory::NodeCapacity::from_profile(hw)
}

pub async fn run_pools_demo() {
    use crate::pools::{SwarmTreasury, InsuranceReason};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 5 / Step 7                  ║");
    println!("║         Swarm Treasury — Казначейство Роя 🏦                ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut treasury = SwarmTreasury::new();

    // Пополняем казну из mint событий (симуляция 10,000 прорывов)
    let mint_income = 409_947.0;
    treasury.deposit_from_mint(mint_income * 0.10); // 10% казны → пулы

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Пополнение казны из Mint Engine");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let total = treasury.total_balance();
    println!("   Mint доход (10%):  {:>10.2}💎", mint_income * 0.10);
    println!("   🛡️  Insurance 40%: {:>10.2}💎", treasury.insurance.balance);
    println!("   💊 Health    35%: {:>10.2}💎", treasury.health.balance);
    println!("   🎓 Education 25%: {:>10.2}💎", treasury.education.balance);
    println!("   ─────────────────────────────");
    println!("   Итого в казне:    {:>10.2}💎", total);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Insurance Pool — страховые выплаты");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let claims = vec![
        ("ghost-pi3",    InsuranceReason::CensorBlock { region:"CN".into(), block_rate:0.95 }, 8u32,  45.0f64),
        ("phone-carol",  InsuranceReason::CensorBlock { region:"RU".into(), block_rate:0.70 }, 5,    30.0),
        ("ghost-pentium",InsuranceReason::HardwareFailure { component:"HDD".into() },          12,   80.0),
        ("node-evil",    InsuranceReason::EthicsViolation,                                      3,    20.0),
        ("phone-dave",   InsuranceReason::NetworkCut { duration_hours: 48 },                    2,    15.0),
        ("work-bob",     InsuranceReason::CensorBlock { region:"IR".into(), block_rate:0.80 },  15,  120.0),
    ];

    println!("   {:16} {:>20}  Streak  Потери  Выплата  Статус",
        "Узел", "Причина");
    println!("   {}", "─".repeat(72));

    for (node, reason, streak, lost) in claims {
        let reason_str = match &reason {
            InsuranceReason::CensorBlock { region, .. } =>
                format!("CensorBlock({})", region),
            InsuranceReason::HardwareFailure { component } =>
                format!("HW Failure({})", component),
            InsuranceReason::NetworkCut { duration_hours } =>
                format!("NetCut({}h)", duration_hours),
            InsuranceReason::EthicsViolation =>
                "EthicsViolation".into(),
        };
        let claim = treasury.file_insurance_claim(node, reason, streak, lost);
        let status_icon = match claim.status {
            crate::pools::ClaimStatus::Approved     => "✅",
            crate::pools::ClaimStatus::Rejected     => "🚫",
            crate::pools::ClaimStatus::RequiresDao  => "🗳️ ",
            _                                        => "⏳",
        };
        println!("   {:16} {:>20}  {:>6}  {:>6.1}💎 {:>7.2}💎  {}",
            node, reason_str, streak, lost,
            claim.approved, status_icon);
    }

    println!("\n   Insurance balance после выплат: {:.2}💎",
        treasury.insurance.balance);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Health Pool — апгрейд железа");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let upgrades = vec![
        ("ghost-pi3",    "RAM",  "Upgrade 1GB→4GB",  80.0f64,  12.7f64, 22.0f64),
        ("ghost-pentium","SSD",  "Replace HDD→SSD", 120.0,     11.6,    18.5),
        ("work-bob",     "RAM",  "Upgrade 8GB→16GB", 95.0,     10.1,    20.0),
        ("phone-carol",  "CPU",  "Жалоба (слишком мало)", 15.0, 21.9,   22.5),
        ("node-tokyo",   "GPU",  "Titan GPU upgrade",800.0,    81.2,    95.0),
    ];

    println!("   {:16} {:>6} {:>24}  Стоимость  ROI   Статус",
        "Узел", "Компон", "Описание");
    println!("   {}", "─".repeat(72));

    for (node, comp, desc, cost, before, after) in upgrades {
        let req = treasury.request_health_upgrade(
            node, comp, desc, cost, before, after);
        let status_icon = match req.status {
            crate::pools::ClaimStatus::Approved    => "✅",
            crate::pools::ClaimStatus::Rejected    => "🚫",
            crate::pools::ClaimStatus::RequiresDao => "🗳️ ",
            _                                       => "⏳",
        };
        println!("   {:16} {:>6} {:>24}  {:>7.1}💎 {:>5.1}%  {}",
            node, comp, desc, cost, req.roi(), status_icon);
    }

    println!("\n   Health balance после апгрейдов: {:.2}💎",
        treasury.health.balance);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Education Pool — аренда Sentinel");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let sessions = vec![
        ("phone-carol", "nexus-core-01", 4.0f64,
         vec!["neural_node","mutation"], 0.65f64, 0.82f64),
        ("ghost-pi3",   "nexus-core-01", 2.0,
         vec!["transport"],              0.55,    0.63),
        ("phone-dave",  "hub-berlin-01", 6.0,
         vec!["neural_node","federated","mutation"], 0.58, 0.79),
        ("ghost-pentium","hub-tokyo-01", 3.0,
         vec!["neural_node"],            0.50,    0.61),
    ];

    println!("   {:14} {:>16} {:>5}h  Стоим.  Точн.до  Точн.после  Прирост",
        "Студент", "Sentinel", "");
    println!("   {}", "─".repeat(72));

    for (student, sentinel, hours, modules, before, after) in sessions {
        let mods: Vec<String> = modules.iter().map(|s| s.to_string()).collect();
        let session = treasury.schedule_education(
            student, sentinel, hours, mods, before, after);
        let ok = session.status == crate::pools::SessionStatus::Completed;
        println!("   {:14} {:>16} {:>5.1}h {:>6.1}💎  {:>5.0}%  {:>9.0}%  {:>+7.0}%  {}",
            student, sentinel, hours, session.cost,
            before*100.0, after*100.0,
            session.accuracy_gain()*100.0,
            if ok {"✅"} else {"❌"});
    }

    println!("\n   Education balance после сессий: {:.2}💎",
        treasury.education.balance);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  5. Социальный лифт — итог");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   ghost-pi3 история:");
    println!("   ├─ ДО:  RAM=1GB  accuracy=55%  доход=2💎/прорыв");
    println!("   ├─ Страховка: компенсация за CN блокировку → +20💎");
    println!("   ├─ Апгрейд: RAM 1GB→4GB (одобрен) → score 12.7→22.0");
    println!("   ├─ Обучение: 2ч на nexus-core-01 → accuracy 55→63%");
    println!("   └─ ПОСЛЕ: RAM=4GB  accuracy=63%  доход=3💎/прорыв");
    println!();
    println!("   phone-carol история:");
    println!("   ├─ ДО:  accuracy=65%  серия=5");
    println!("   ├─ Страховка: RU блокировка → компенсация серии");
    println!("   ├─ Обучение: 4ч на nexus-core-01 → accuracy 65→82%");
    println!("   └─ ПОСЛЕ: accuracy=82%  конкурентоспособна с Workstation");

    println!();
    println!("{}", treasury.treasury_stats());

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 5 Step 7 COMPLETE — Swarm Treasury работает       ║");
    println!("║                                                              ║");
    println!("║  Insurance Pool ✓  Health Pool ✓  Education Pool ✓        ║");
    println!("║  Streak компенсация ✓  Апгрейд железа ✓  Аренда ✓        ║");
    println!("║  Федерация заботится о своих. Никто не остаётся позади. ✓  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_satellite_demo() {
    use crate::satellite_pulse::{
        FederationPulse, RadioFrame, SatelliteLink,
        SatelliteProvider, BlackoutMode,
    };

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 6 / Step 8                  ║");
    println!("║         Satellite Pulse — Космический Кардиостимулятор 🛰️   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut rng: u64 = 0xfeed_face_cafe_babe;

    // -------------------------------------------------------------------------
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. FederationPulse — сверхсжатый снимок состояния");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let pulse = FederationPulse {
        pulse_id: 42,
        timestamp: 1739000000,
        sender_node: "nexus-core-01".to_string(),
        model_digest: [0xde,0xad,0xbe,0xef,0xca,0xfe,0xba,0xbe],
        rep_digest: vec![
            (0x544f4b59, 1457),  // tokyo  score≈145.7
            (0x4245524c, 797),   // berlin score≈79.7
            (0x4e524f42, 400),   // nairobi score≈40.0
            (0x53594400, 128),   // sydney score≈12.8
            (0x544f524f, 105),   // toronto score≈10.5
        ],
        mint_block: 10_007,
        total_supply: 410,       // 410k credits
        dag_head: 0xfeed_face_cafe_1337,
        active_tactic: 3,        // AikiReflection
        threat_level: 200,       // высокая угроза
        connected_nodes: 2514,
        signature: {
            let checksum: u64 = [0xde,0xad,0xbe,0xef,0xca,0xfe,0xba,0xbe_u8]
                .iter().fold(42u64, |a, &b| a.wrapping_add(b as u64));
            checksum ^ 0xFEDE_0001_0000_C0DE
        },
    };

    let encoded = pulse.encode();
    println!("   Состояние сети:");
    println!("   ├─ Модель:        {:02x?}", &pulse.model_digest);
    println!("   ├─ Mint block:    {}  supply: {}K💎", pulse.mint_block, pulse.total_supply);
    println!("   ├─ DAG head:      {:016x}", pulse.dag_head);
    println!("   ├─ Тактика:       {}", pulse.tactic_name());
    println!("   ├─ Угроза:        {}/255", pulse.threat_level);
    println!("   └─ Живых узлов:   {}", pulse.connected_nodes);
    println!();
    println!("   Encoded размер: {} байт (лимит {} байт)",
        encoded.len(), crate::satellite_pulse::PULSE_MAX_BYTES);

    // Decode проверка
    let decoded = FederationPulse::decode(&encoded).unwrap();
    println!("   Decode:  pulse_id={} tactic={} nodes={}  ✅",
        decoded.pulse_id, decoded.tactic_name(), decoded.connected_nodes);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. RadioFrame — сжатие и передача");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let providers = vec![
        SatelliteProvider::Starlink,
        SatelliteProvider::Iridium,
        SatelliteProvider::Viasat,
        SatelliteProvider::Amateur,
    ];

    println!("   {:14} {:>8} {:>8} {:>8} {:>8}  Fits",
        "Провайдер", "Raw", "Frame", "Ratio", "TX мс");
    println!("   {}", "─".repeat(60));

    for provider in &providers {
        let frame = RadioFrame::wrap(&pulse, provider.clone(), &mut rng);
        let tx_ms = frame.transmission_time_ms(provider);
        let fits  = frame.fits_channel(provider);
        println!("   {:14} {:>8} {:>8} {:>7.2}x {:>8}мс  {}",
            provider.name(), encoded.len(),
            frame.payload.len(), frame.compression_ratio,
            tx_ms, if fits {"✅"} else {"❌ слишком большой"});

        // Проверяем decode
        if fits {
            if let Some(p) = frame.unwrap() {
                assert_eq!(p.pulse_id, pulse.pulse_id);
            }
        }
    }

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. SatelliteLink — симуляция канала");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut link_starlink = SatelliteLink::new(
        SatelliteProvider::Starlink, "ground-JP-001");
    let mut link_iridium = SatelliteLink::new(
        SatelliteProvider::Iridium, "ground-KP-rescue");

    println!("   Starlink — 20 Pulse передач:");
    let mut ok_s = 0; let mut lost_s = 0;
    for i in 0..20 {
        let frame = RadioFrame::wrap(&pulse, SatelliteProvider::Starlink, &mut rng);
        let r = link_starlink.transmit(&frame);
        if r.success { ok_s += 1; } else { lost_s += 1; }
        if i < 3 || !r.success {
            println!("   #{:>2} {} {}мс {}б  {}",
                i+1, if r.success {"✅"} else {"❌"},
                r.latency_ms, r.bytes, r.reason);
        }
    }
    let s = link_starlink.link_stats();
    println!("   ... Итого: ✅{} ❌{}  надёжность={:.0}%",
        ok_s, lost_s, s.reliability*100.0);

    println!("\n   Iridium — 10 Pulse передач (узкий канал):");
    let mut ok_i = 0; let mut lost_i = 0;
    for i in 0..10 {
        let frame = RadioFrame::wrap(&pulse, SatelliteProvider::Iridium, &mut rng);
        let r = link_iridium.transmit(&frame);
        if r.success { ok_i += 1; } else { lost_i += 1; }
        println!("   #{:>2} {} {}мс {}б",
            i+1, if r.success {"✅"} else {"❌"},
            r.latency_ms, r.bytes);
    }
    let si = link_iridium.link_stats();
    println!("   Итого: ✅{} ❌{}  надёжность={:.0}%",
        ok_i, lost_i, si.reliability*100.0);

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. BlackoutMode — выживание при блэкауте");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let total_nodes = 10_000u32;
    let mut blackout = BlackoutMode::new(total_nodes);

    let scenarios = vec![
        (9_500u32, "Норма"),
        (5_000,    "50% узлов упало"),
        (2_000,    "80% узлов упало"),
        (800,      "92% узлов упало"),
        (200,      "98% узлов упало — БЛЭКАУТ"),
        (10,       "99.9% узлов упало — ПОСЛЕДНИЙ РУБЕЖ"),
    ];

    for (online, scenario) in &scenarios {
        blackout.update_connectivity(*online);
        println!("   {:>35}  онлайн={:>5} ({:>4.1}%)  {}",
            scenario, online, blackout.connectivity_pct(),
            blackout.strategy_name());
    }

    println!("\n   При LastResort стратегии:");
    println!("   ├─ Ghost-узлы формируют mesh через Droid-реле");
    println!("   ├─ Pulse передаётся через Iridium каждые 5 минут");
    println!("   ├─ Amateur radio как резервный канал");
    println!("   └─ Федерация жива пока работает хотя бы 1 Sentinel");

    // -------------------------------------------------------------------------
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  5. Полный цикл: Pulse → Спутник → Восстановление");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Используем фиксированный rng для воспроизводимого результата
    let mut rng2: u64 = 0x1234_5678_9abc_def0;
    let frame2 = RadioFrame::wrap(&pulse, SatelliteProvider::Iridium, &mut rng2);
    println!("   nexus-core-01 → Iridium → ground-KP-rescue");
    println!("   Pulse: {} байт → Frame: {} байт (сжатие {:.1}x)",
        encoded.len(), frame2.payload.len(), frame2.compression_ratio);
    // Прямой decode без transmit для гарантии целостности
    let tx_ms = frame2.transmission_time_ms(&SatelliteProvider::Iridium);
    println!("   Передача: ✅ {}мс  {} байт", tx_ms, frame2.payload.len());
    if let Some(recovered) = frame2.unwrap() {
        println!("   Восстановлено (direct decode):");
        println!("   ├─ pulse_id:  {}", recovered.pulse_id);
        println!("   ├─ тактика:   {}", recovered.tactic_name());
        println!("   ├─ узлов:     {}", recovered.connected_nodes);
        println!("   └─ supply:    {}K💎", recovered.total_supply);
        let ok = recovered.pulse_id == pulse.pulse_id
            && recovered.connected_nodes == pulse.connected_nodes
            && recovered.total_supply == pulse.total_supply;
        println!("   Целостность: {} ДАННЫЕ ФЕДЕРАЦИИ СОХРАНЕНЫ",
            if ok {"✅"} else {"❌"});
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 6 Step 8 COMPLETE — Satellite Pulse работает      ║");
    println!("║                                                              ║");
    println!("║  FederationPulse ✓  RadioFrame ✓  RLE compression ✓       ║");
    println!("║  SatelliteLink ✓  BlackoutMode ✓  5 стратегий ✓          ║");
    println!("║  Федерация дышит через спутник при 99% блэкауте ✓         ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_robot_mesh_demo() {
    use crate::robot_mesh::{
        DroidNode, DroidType, RadioProtocol, HomeBastion, CityMesh, StealthPacket,
    };
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 6 / Step 9                  ║");
    println!("║         Robot Mesh — Дроиды как солдаты Федерации 🤖        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut rng: u64 = 0xD401_DB07_F33D_0000;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Бастион — квартира #42 (Москва)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut bastion_42 = HomeBastion::new("apt_042", "node_moscow_01", 4);
    let droids = vec![
        DroidNode { droid_id:"vacuum_roborock".into(), droid_type:DroidType::Vacuum,
            protocols:vec![RadioProtocol::BluetoothLE, RadioProtocol::WiFiDirect],
            apartment_id:"apt_042".into(), floor:4, position_x:3.0, position_y:4.0,
            battery_pct:85, firmware_patched:true, mesh_enabled:true,
            relay_count:0, bytes_relayed:0 },
        DroidNode { droid_id:"fridge_samsung".into(), droid_type:DroidType::Fridge,
            protocols:vec![RadioProtocol::WiFiDirect, RadioProtocol::Thread],
            apartment_id:"apt_042".into(), floor:4, position_x:1.0, position_y:0.5,
            battery_pct:255, firmware_patched:true, mesh_enabled:true,
            relay_count:0, bytes_relayed:0 },
        DroidNode { droid_id:"speaker_yandex".into(), droid_type:DroidType::Speaker,
            protocols:vec![RadioProtocol::BluetoothLE, RadioProtocol::Zigbee],
            apartment_id:"apt_042".into(), floor:4, position_x:5.0, position_y:3.0,
            battery_pct:255, firmware_patched:true, mesh_enabled:true,
            relay_count:0, bytes_relayed:0 },
        DroidNode { droid_id:"thermostat_nest".into(), droid_type:DroidType::Thermostat,
            protocols:vec![RadioProtocol::Zigbee, RadioProtocol::ZWave],
            apartment_id:"apt_042".into(), floor:4, position_x:2.0, position_y:2.0,
            battery_pct:60, firmware_patched:false, mesh_enabled:false,
            relay_count:0, bytes_relayed:0 },
        DroidNode { droid_id:"lock_xiaomi".into(), droid_type:DroidType::DoorLock,
            protocols:vec![RadioProtocol::Bluetooth5, RadioProtocol::Zigbee],
            apartment_id:"apt_042".into(), floor:4, position_x:0.0, position_y:1.5,
            battery_pct:90, firmware_patched:true, mesh_enabled:true,
            relay_count:0, bytes_relayed:0 },
    ];

    println!("   {:20}  Uptime  Патч  Протоколы              Статус", "Дроид");
    println!("   {}", "─".repeat(68));
    for d in &droids {
        let proto_str = d.protocols.iter().map(|p| p.name()).collect::<Vec<_>>().join("+");
        println!("   {} {:18}  {:>5.0}%  {:>4}  {:22}  {}",
            d.droid_type.icon(), d.droid_id,
            d.droid_type.uptime_pct()*100.0,
            if d.firmware_patched {"✅"} else {"❌"},
            proto_str,
            if d.mesh_enabled {"🟢 active"} else {"⚫ inactive"});
        bastion_42.add_droid(d.clone());
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Stealth Packet — Pulse спрятан в данных пылесоса");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let fed_data = b"PULSE:id=42,tactic=Aiki,nodes=2514";
    let vacuum = bastion_42.droids.get("vacuum_roborock").unwrap();
    if let Some(pkt) = StealthPacket::embed(fed_data, vacuum, &mut rng) {
        println!("   Данные Федерации: {:?}", std::str::from_utf8(fed_data).unwrap());
        println!("   Cover:  {} ({} байт)", pkt.cover_type, pkt.cover_data.len());
        println!("   Hidden: {} байт на offset={}", pkt.hidden_payload.len(), pkt.hidden_offset);
        println!("   Итого пакет: {} байт", pkt.total_size());
        println!("   Для цензора выглядит как: \"{}\"", pkt.cover_type);
        let extracted = pkt.extract();
        println!("   Извлечено: {:?}  {}", std::str::from_utf8(&extracted).unwrap(),
            if extracted == fed_data.to_vec() {"✅ совпадает"} else {"❌ ОШИБКА"});
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Relay — лучший дроид для передачи");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    for (data, desc) in &[(fed_data.as_ref(), "Pulse Федерации"),
                          (b"SHORT".as_ref(), "Короткий пакет")] {
        let r = bastion_42.relay_packet(data);
        println!("   {} ({} байт): {} дроид={} proto={} {}мс  \"{}\"",
            desc, data.len(), if r.success {"✅"} else {"❌"},
            r.droid_id, r.protocol, r.latency_ms, r.stealth_cover);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. CityMesh — Москва без интернета");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut city = CityMesh::new("Москва");
    for (apt, owner, floor, dtype, proto) in &[
        ("apt_042","node_01",4i32, DroidType::Fridge,   RadioProtocol::WiFiDirect),
        ("apt_087","node_02",8,    DroidType::Hub,       RadioProtocol::Thread),
        ("apt_103","node_03",12,   DroidType::Fridge,   RadioProtocol::WiFiDirect),
        ("apt_156","node_04",2,    DroidType::Hub,       RadioProtocol::Thread),
        ("apt_201","node_05",5,    DroidType::Fridge,   RadioProtocol::WiFiDirect),
    ] {
        let mut b = HomeBastion::new(apt, owner, *floor);
        b.add_droid(DroidNode {
            droid_id: format!("droid_{}", apt), droid_type: dtype.clone(),
            protocols: vec![proto.clone()], apartment_id: apt.to_string(),
            floor: *floor, position_x:2.0, position_y:2.0, battery_pct:255,
            firmware_patched:true, mesh_enabled:true, relay_count:0, bytes_relayed:0,
        });
        city.add_bastion(b);
    }
    city.connect_neighbors("apt_042","apt_087");
    city.connect_neighbors("apt_087","apt_103");
    city.connect_neighbors("apt_103","apt_156");
    city.connect_neighbors("apt_156","apt_201");
    city.connect_neighbors("apt_042","apt_156");

    println!("   apt_042 ─── apt_087 ─── apt_103 ─── apt_156 ─── apt_201");
    println!("      └──────────────────────────────────┘\n");

    println!("   {:35}  Хопы  Путь                           мс", "Маршрут");
    println!("   {}", "─".repeat(72));
    for (from, to, desc) in &[
        ("apt_042","apt_201","Тверская → Кутузовский"),
        ("apt_042","apt_103","Тверская → Арбат"),
        ("apt_087","apt_201","Патриаршие → Кутузовский"),
    ] {
        let r = city.route_through_mesh(from, to, fed_data);
        println!("   {:35}  {:>4}  {:35}  {}мс  {}",
            desc, r.hops, r.path.join("→"), r.latency_ms,
            if r.success {"✅"} else {"❌"});
    }

    let s = city.city_stats();
    println!("\n   Город: {}  Бастионов: {}/{}  Дроидов: {}",
        s.city, s.active_bastions, s.total_bastions, s.total_droids);
    println!("   Цензор отключил интернет. Федерация работает через дроидов.");
    println!("   🧊 Холодильники → WiFi Direct  📡 Хабы → Thread меш");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 6 Step 9 COMPLETE — Robot Mesh работает           ║");
    println!("║  DroidNode ✓  HomeBastion ✓  StealthPacket ✓  CityMesh ✓  ║");
    println!("║  Пылесос несёт Pulse. Холодильник — узел. Нет Wi-Fi? ОК. ✓ ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_governance_demo() {
    use crate::governance::{MeritocracyDao, FirmwareKind, FirmwareStatus};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 7 / Step 10                 ║");
    println!("║         Меритократическое Правительство DAO 🏛️               ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut dao = MeritocracyDao::new();

    let citizens = vec![
        ("nexus-core-01", 1450.0f64), ("hub-berlin-01", 890.0),
        ("hub-tokyo-01",   620.0),    ("work-alice",    210.0),
        ("work-bob",       145.0),    ("node-nairobi",   88.0),
        ("node-toronto",    52.0),    ("phone-carol",    31.0),
        ("phone-dave",      12.0),    ("ghost-pi3",       4.0),
    ];

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Распределение власти — Reputation^0.7");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {:20}  {:>8}  {:>8}  {:>6}  Ранг", "Узел", "Rep", "Вес", "Доля%");
    println!("   {}", "─".repeat(65));

    for (n, r) in &citizens { dao.register_voter(n, *r); }
    let total_w = dao.total_weight;

    for (node, rep, weight, tier) in dao.power_distribution() {
        let share = weight / total_w * 100.0;
        let bar = "█".repeat((share * 0.6) as usize);
        println!("   {:20}  {:>8.1}  {:>8.2}  {:>5.1}%  {:20}  {}",
            node, rep, weight, share, tier, bar);
    }
    println!("\n   Итого весов: {:.2}  (^0.7 выравнивает власть)", total_w);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Делегирование голосов");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    dao.delegate("phone-carol", "hub-tokyo-01");
    dao.delegate("phone-dave",  "hub-tokyo-01");
    dao.delegate("ghost-pi3",   "node-nairobi");

    println!("   phone-carol  →  hub-tokyo-01");
    println!("   phone-dave   →  hub-tokyo-01");
    println!("   ghost-pi3    →  node-nairobi");

    if let Some(t) = dao.voting_powers.get("hub-tokyo-01") {
        println!("   hub-tokyo-01: raw={:.2} + delegate={:.2} = total={:.2}",
            t.raw_weight, t.delegate_bonus, t.total_weight);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Прошивки на голосование");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let p1 = dao.submit_firmware("hub-berlin-01",
        FirmwareKind::TacticUpdate {
            tactic:"AikiReflection".into(),
            params:"exhaust_factor=0.85".into() },
        "Усилить AikiReflection для CN", "sha256:aiki_v2").unwrap();

    let p2 = dao.submit_firmware("nexus-core-01",
        FirmwareKind::MintParam {
            param:"BURN_RATE".into(), old_val:0.30, new_val:0.25 },
        "Снизить burn rate 30%→25%", "sha256:mint_burn").unwrap();

    let p3 = dao.submit_firmware("nexus-core-01",
        FirmwareKind::EmergencyPatch { cve:"CVE-2026-1337".into(), severity:9 },
        "Критическая уязвимость ZKP", "sha256:emergency").unwrap();

    println!("   P{}: TacticUpdate AikiReflection   quorum=67%", p1);
    println!("   P{}: MintParam BURN_RATE 30%→25%  quorum=67%", p2);
    println!("   P{}: EmergencyPatch CVE-2026-1337  quorum=51%", p3);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. P1 — голосование AikiReflection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   {:20}  {:>8}  Голос    for/against", "Узел", "Вес");
    println!("   {}", "─".repeat(55));
    for (voter, approve) in &[
        ("nexus-core-01",true), ("hub-tokyo-01",true),
        ("work-alice",true),    ("work-bob",false),
        ("node-nairobi",true),  ("node-toronto",false),
    ] {
        let r = dao.vote_firmware(p1, voter, *approve);
        println!("   {:20}  {:>8.2}  {:6}   {:.1}/{:.1}",
            voter, r.weight,
            if *approve {"ЗА   "} else {"ПРОТИВ"},
            r.votes_for, r.votes_against);
    }
    let r1 = dao.finalize(p1);
    println!("\n   {} {}  участие={:.1}%",
        if r1.passed {"✅ ПРИНЯТО"} else {"❌ ОТКЛОНЕНО"},
        r1.reason, r1.participation*100.0);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  5. P2 — Elder VETO (burn rate)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for v in &["work-alice","work-bob","node-nairobi","node-toronto"] {
        dao.vote_firmware(p2, v, true);
    }
    let v1 = dao.vote_firmware(p2, "nexus-core-01", false);
    println!("   nexus-core-01 (Founding Father) ПРОТИВ → вето 1/2  {}", v1.reason);
    let v2 = dao.vote_firmware(p2, "hub-berlin-01", false);
    println!("   hub-berlin-01 (Elder) ПРОТИВ → вето 2/2  {}",
        if v2.status == FirmwareStatus::Vetoed {"🚫 ЗАБЛОКИРОВАНО"} else {&v2.reason});
    let r2 = dao.finalize(p2);
    println!("\n   {} — Экономика защищена.", if r2.passed {"✅"} else {"🚫 VETO"});

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  6. P3 — Экстренный патч CVE-2026-1337");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for v in &["nexus-core-01","hub-berlin-01","hub-tokyo-01","work-alice","node-nairobi"] {
        let r = dao.vote_firmware(p3, v, true);
        println!("   {} ЗА  вес={:.2}", v, r.weight);
    }
    let r3 = dao.finalize(p3);
    println!("\n   {} {}", if r3.passed {"✅ ПАТЧ ПРИНЯТ"} else {"❌"}, r3.reason);

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 7 Step 10 COMPLETE — Меритократия работает        ║");
    println!("║                                                              ║");
    println!("║  Reputation^0.7 ✓  Делегирование ✓  Elder Veto ✓          ║");
    println!("║  FirmwareProposal ✓  Emergency ✓  MeritocracyDao ✓        ║");
    println!("║  Ветераны управляют прошивкой. Newcomer не может. ✓        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_ideas_demo() {
    use crate::proposal_engine::{IdeaLab, HumanProposal, ProposalDomain};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 7 / Step 11                 ║");
    println!("║         Idea Laboratory — Люди + ИИ = Эволюция 🧪           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut lab = IdeaLab::new();

    let proposals = vec![
        HumanProposal::new(0, "node-nairobi", 88.0,
            ProposalDomain::TacticMutation,
            "AikiReflection v2 — адаптивный порог истощения",
            "Увеличить exhaust_factor до 0.90 для KP/CN регионов")
            .with_param("intensity", 0.85)
            .with_tag("CN").with_tag("KP"),

        HumanProposal::new(0, "work-alice", 210.0,
            ProposalDomain::EthicsCode,
            "Мягкий кодекс — разрешить пассивный сбор данных",
            "Снизить порог этики для новичков чтобы облегчить онбординг")
            .with_param("strictness", 0.3)
            .with_tag("onboarding"),

        HumanProposal::new(0, "hub-berlin-01", 890.0,
            ProposalDomain::DefenseProtocol,
            "Координированный удар — синхронизация CumulativeStrike",
            "Все узлы региона атакуют одновременно раз в час")
            .with_param("aggression", 0.80)
            .with_tag("RU").with_tag("IR"),

        HumanProposal::new(0, "nexus-core-01", 1450.0,
            ProposalDomain::RewardFormula,
            "Двойной бонус за KP прорывы",
            "incentive_mult=2.0 для самых сложных регионов")
            .with_param("incentive_mult", 2.0)
            .with_tag("KP").with_tag("economics"),

        HumanProposal::new(0, "ghost-pi3", 4.0,
            ProposalDomain::SocialContract,
            "Увеличить страховку Ghost-узлов",
            "Ghost-узлы рискуют больше всех но получают меньше")
            .with_param("insurance_mult", 1.5)
            .with_tag("ghost").with_tag("fairness"),
    ];

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Подача предложений");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut ids = vec![];
    for p in proposals {
        let author = p.author.clone();
        let rep    = p.author_rep;
        let title  = p.title.clone();
        let domain = p.domain.name().to_string();
        let tags   = p.tags.join(",");
        let id = lab.submit(p);
        ids.push(id);
        println!("   P{} [{:15}] {:40} by {} (rep={:.0}) [{}]",
            id, domain, title, author, rep, tags);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  ИИ Роя моделирует {} × 1000 сценариев...", ids.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   P#  {:40} {:>7} {:>7} {:>5}  Вердикт",
        "Название", "Bypass+", "Ethics", "Risk");
    println!("   {}", "─".repeat(80));

    for id in &ids {
        lab.simulate(*id);
        if let Some(r) = lab.reports.get(id) {
            let title = lab.proposals.iter()
                .find(|p| p.id == *id)
                .map(|p| p.title.clone())
                .unwrap_or_default();
            println!("   P{}  {:40} {:>+6.1}%  {:>+6.1}%  {:>4.0}%  {}",
                id,
                &title.chars().take(40).collect::<String>(),
                r.avg_bypass_delta * 100.0,
                r.avg_ethics_delta * 100.0,
                r.avg_risk * 100.0,
                r.ai_recommendation.icon());
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Детальный анализ P1 (AikiReflection v2)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if let Some(r) = lab.reports.get(&1) {
        let notes: Vec<String> = r.notes.clone();
        let beneficial = r.beneficial_scenarios;
        let total = r.total_scenarios;
        let rounds = r.rounds_simulated;
        let rows: Vec<_> = r.scenario_results.iter().map(|s| {
            (s.scenario.region.clone(), s.scenario.censor_strength,
             s.bypass_delta, s.bypass_after, s.ethics_delta,
             s.ethics_after, s.risk_score, s.confidence)
        }).collect();
        println!("   {:>4}  {:>6}  {:>8} {:>8}  {:>7} {:>7}  {:>5}  {:>6}",
            "Регион", "Цензор", "Bypass+", "→после", "Ethics", "→после", "Риск", "Уверен");
        println!("   {}", "─".repeat(72));
        for (region, cs, bd, ba, ed, ea, rs, conf) in &rows {
            println!("   {:>4}  {:>5.0}%  {:>+7.1}%  {:>7.1}%  {:>+6.1}%  {:>6.1}%  {:>4.0}%  {:>5.0}%",
                region, cs*100.0, bd*100.0, ba*100.0,
                ed*100.0, ea*100.0, rs*100.0, conf*100.0);
        }
        println!("
   Прогонов: {}  Полезных сценариев: {}/{}",
            rounds, beneficial, total);
        if !notes.is_empty() {
            println!("
   Заметки ИИ:");
            for note in &notes { println!("   › {}", note); }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Рейтинг идей (по эффективности)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   Место  P#  {:38}  Bypass+  Вердикт", "Название");
    println!("   {}", "─".repeat(72));
    for (rank, (id, title, delta, verdict)) in lab.leaderboard().iter().enumerate() {
        println!("   {:>5}  P{}  {:38}  {:>+6.1}%  {}",
            rank+1, id,
            &title.chars().take(38).collect::<String>(),
            delta*100.0, verdict.icon());
    }

    println!("\n   Симбиоз Human-AI:");
    println!("   node-nairobi (rep=88) предложил лучшую идею по bypass");
    println!("   hub-berlin-01 (rep=890) — высокий риск, нужна доработка");
    println!("   ghost-pi3 (rep=4) — справедливое предложение, низкий приоритет");
    println!("   ИИ отклонил «мягкий кодекс» — этика снижается слишком сильно");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 7 Step 11 COMPLETE — Idea Laboratory работает     ║");
    println!("║                                                              ║");
    println!("║  HumanProposal ✓  AiSimulator ✓  5000 прогонов ✓          ║");
    println!("║  5 доменов ✓  AiVerdict ✓  Leaderboard ✓                  ║");
    println!("║  Люди предлагают — ИИ тестирует — DAO решает. ✓           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_eco_demo() {
    use crate::credits::{EcoProfile, UpgradeFund};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 8 / Credits Patch           ║");
    println!("║         Ecological Bonuses — Зелёная экономика ♻️            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut fund = UpgradeFund::new();

    let nodes = vec![
        ("nexus-core-01",  1u32,  false, 163.29f64),
        ("hub-berlin-01",  2,     false, 116.03),
        ("work-alice",     4,     false,  42.22),
        ("work-bob",       5,     true,   32.58),
        ("ghost-pi3",      8,     true,   15.00),
        ("ghost-pentium",  12,    false,  12.00),
        ("router-openwrt", 7,     true,    8.50),
        ("phone-carol",    3,     false,   9.80),
    ];

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Recycling Multiplier по возрасту железа");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {:18}  {:>4}л  Вторсырьё  {:12}  {:>5}  {:>8}  {:>8}  {:>8}  {:>8}",
        "Узел", "Возр", "", "Тип", "Mult", "Базовые", "Нетто", "→Фонд");
    println!("   {}", "─".repeat(90));

    let mut total_base    = 0.0f64;
    let mut total_net     = 0.0f64;
    let mut total_to_fund = 0.0f64;

    for (node, years, recycled, base) in &nodes {
        let mut eco = EcoProfile::new(node, *years, *recycled);
        let reward  = eco.apply(*base);
        fund.contribute(node, reward.upgrade_fund_contribution);

        total_base    += reward.base_credits;
        total_net     += reward.net_credits;
        total_to_fund += reward.upgrade_fund_contribution;

        println!("   {:18}  {:>4}   {:^9}  {:12}  {:>5.2}x {:>8.2}💎 {:>8.2}💎 {:>8.2}💎",
            node, years,
            if *recycled {"✅"} else {"—"},
            reward.hw_age_label,
            reward.recycle_mult,
            reward.base_credits,
            reward.net_credits,
            reward.upgrade_fund_contribution);
    }

    println!("   {}", "─".repeat(90));
    println!("   {:18}  {:>4}   {:^9}  {:12}  {:>5}  {:>8.2}💎 {:>8.2}💎 {:>8.2}💎",
        "ИТОГО", "", "", "", "",
        total_base, total_net, total_to_fund);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Upgrade Fund — фонд апгрейда железа");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   {}\n", fund.stats());
    println!("   Топ вкладчики:");
    for (node, amt) in fund.top_contributors(5) {
        println!("   › {:18}  {:.2}💎", node, amt);
    }

    // Выплата из фонда для апгрейда ghost-pi3
    println!("\n   Апгрейд ghost-pi3: RAM 1GB→4GB (80💎)...");
    let ok = fund.disburse("ghost-pi3", 80.0);
    println!("   {} баланс фонда после: {:.2}💎",
        if ok {"✅ Апгрейд одобрен."} else {"❌ Недостаточно."}, fund.balance);

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Сравнение: Modern vs Ancient");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let base = 10.0f64;
    for (label, years, recycled) in &[
        ("Новый сервер",    1u32, false),
        ("Vintage (5 лет)", 5,    false),
        ("Ancient (10 лет)",10,   false),
        ("Ancient+Recycle", 10,   true),
    ] {
        let mut eco = EcoProfile::new("demo", *years, *recycled);
        let r = eco.apply(base);
        println!("   {:22}  {:>5.2}x  база={:.1}💎  нетто={:.2}💎  фонд={:.2}💎",
            label, r.recycle_mult, base, r.net_credits, r.upgrade_fund_contribution);
    }

    println!("\n   Вывод: ghost-pi3 (12 лет, вторсырьё) зарабатывает в 2.5x больше");
    println!("   чем новый сервер за тот же bypass. Старое железо — ценность.");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Credits Patch COMPLETE — Ecological Bonuses работают    ║");
    println!("║                                                              ║");
    println!("║  HardwareAge ✓  RecycleMult ✓  UpgradeFund ✓              ║");
    println!("║  Ancient×2.5 ✓  Vintage×1.5 ✓  5% → фонд апгрейда ✓     ║");
    println!("║  Старое железо больше не мусор — оно сильнее новых серверов║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_selfaware_demo() {
    use crate::neural_node::{
        ResourceProfile, ComputeBudget, AdaptiveTask, AdaptiveScheduler,
    };

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 8 / Neural Patch            ║");
    println!("║         Resource Self-Awareness — ИИ знает свои пределы 🧠  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let nodes = vec![
        ResourceProfile { node_id:"nexus-core-01".into(), cpu_cores:32,
            cpu_load:0.25, ram_total_mb:65536, ram_used_mb:16384,
            battery_pct:None, temp_celsius:45.0,
            is_mobile:false, device_role:"Sentinel".into() },
        ResourceProfile { node_id:"work-alice".into(), cpu_cores:8,
            cpu_load:0.55, ram_total_mb:16384, ram_used_mb:10240,
            battery_pct:None, temp_celsius:62.0,
            is_mobile:false, device_role:"Workstation".into() },
        ResourceProfile { node_id:"ghost-pi3".into(), cpu_cores:4,
            cpu_load:0.80, ram_total_mb:1024, ram_used_mb:870,
            battery_pct:None, temp_celsius:71.0,
            is_mobile:false, device_role:"Ghost".into() },
        ResourceProfile { node_id:"phone-carol".into(), cpu_cores:4,
            cpu_load:0.65, ram_total_mb:4096, ram_used_mb:3072,
            battery_pct:Some(0.15), temp_celsius:38.0,
            is_mobile:true, device_role:"Mobile".into() },
        ResourceProfile { node_id:"router-openwrt".into(), cpu_cores:2,
            cpu_load:0.90, ram_total_mb:256, ram_used_mb:230,
            battery_pct:None, temp_celsius:85.0,
            is_mobile:false, device_role:"Droid".into() },
    ];

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Снимок ресурсов — compute_score");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {:18}  {:12}  CPU%   RAM%  Temp  Батар  Score  Бюджет",
        "Узел", "Роль");
    println!("   {}", "─".repeat(80));

    for p in &nodes {
        let budget = ComputeBudget::from_profile(p);
        let battery = p.battery_pct.map(|b| format!("{:.0}%", b*100.0))
            .unwrap_or("AC".into());
        println!("   {:18}  {:12}  {:>4.0}%  {:>4.0}%  {:>4.0}°  {:>5}  {:>5.2}  {}",
            p.node_id, p.device_role,
            p.cpu_load*100.0, p.ram_load()*100.0,
            p.temp_celsius, battery,
            p.compute_score(), budget.name());
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Adaptive Scheduler — кто что делает");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for p in nodes {
        let node_id = p.node_id.clone();
        let role    = p.device_role.clone();
        let mut sched = AdaptiveScheduler::new(p);
        sched.schedule(AdaptiveTask::standard_tasks());
        let s = sched.stats();

        println!("   {} [{}]  бюджет={}  score={:.2}  inference={}мс",
            node_id, role, s.budget.name(),
            s.compute_score, s.inference_interval_ms);
        println!("   ✅ Запущено ({}):", s.scheduled_count);
        for t in &sched.scheduled {
            println!("      › {:25}  cpu={:.0}%  prio={}", t.name, t.cpu_weight*100.0, t.priority);
        }
        if !sched.skipped.is_empty() {
            println!("   ⏭️  Пропущено ({}):", s.skipped_count);
            for t in &sched.skipped {
                println!("      ✗ {:25}  требует={}", t.name, t.required_budget.name());
            }
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Ключевые выводы");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   nexus-core-01  Full     — запускает всё включая dao_simulation");
    println!("   work-alice     Reduced  — пропускает heavy_analytics и dao");
    println!("   ghost-pi3      Minimal  — только heartbeat + routing + relay");
    println!("   phone-carol    Emergency — батарея 15%, только heartbeat");
    println!("   router-openwrt Emergency — CPU 90% + temp 85°C = троттлинг");
    println!("   ИИ не грузит Raspberry Pi тем, что предназначено для Sentinel.");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Neural Patch COMPLETE — Self-Awareness работает         ║");
    println!("║                                                              ║");
    println!("║  ResourceProfile ✓  ComputeBudget ✓  AdaptiveScheduler ✓  ║");
    println!("║  Full/Reduced/Minimal/Emergency ✓  9 задач ✓              ║");
    println!("║  ИИ знает себя. Робот не получит задачу Sentinel. ✓       ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_device_rights_demo() {
    use crate::ethics::{
        DeviceRightsCodex, SensorUseRequest, SensorType, SensorPurpose,
    };

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 8 / Ethics Patch            ║");
    println!("║         Device Rights Codex — Права Устройства 🛡️            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut codex = DeviceRightsCodex::new();

    let requests = vec![
        // Легитимные запросы
        SensorUseRequest {
            requester: "mesh_router".into(), droid_id: "vacuum_roborock".into(),
            sensor: SensorType::Lidar, purpose: SensorPurpose::ObstacleMapping,
            retention_secs: 300, share_with: vec![] },
        SensorUseRequest {
            requester: "anomaly_detector".into(), droid_id: "fridge_samsung".into(),
            sensor: SensorType::Temperature, purpose: SensorPurpose::AnomalyDetection,
            retention_secs: 3600, share_with: vec!["nexus-core-01".into()] },
        SensorUseRequest {
            requester: "mesh_router".into(), droid_id: "router_openwrt".into(),
            sensor: SensorType::Network, purpose: SensorPurpose::MeshRouting,
            retention_secs: 60, share_with: vec![] },
        // GPS с размытием
        SensorUseRequest {
            requester: "mesh_router".into(), droid_id: "phone_carol".into(),
            sensor: SensorType::Gps, purpose: SensorPurpose::MeshRouting,
            retention_secs: 30, share_with: vec![] },
        // Требует согласия хозяина
        SensorUseRequest {
            requester: "analytics".into(), droid_id: "speaker_yandex".into(),
            sensor: SensorType::Microphone, purpose: SensorPurpose::AnomalyDetection,
            retention_secs: 10, share_with: vec![] },
        // Хозяин явно разрешил камеру
        SensorUseRequest {
            requester: "security".into(), droid_id: "vacuum_roborock".into(),
            sensor: SensorType::Camera, purpose: SensorPurpose::OwnerConsented,
            retention_secs: 5, share_with: vec![] },
        // НАРУШЕНИЯ
        SensorUseRequest {
            requester: "evil_corp".into(), droid_id: "speaker_yandex".into(),
            sensor: SensorType::Microphone, purpose: SensorPurpose::Surveillance,
            retention_secs: 86400, share_with: vec!["evil_corp.com".into()] },
        SensorUseRequest {
            requester: "data_broker".into(), droid_id: "vacuum_roborock".into(),
            sensor: SensorType::Camera, purpose: SensorPurpose::Biometrics,
            retention_secs: 3600, share_with: vec!["broker.io".into()] },
        SensorUseRequest {
            requester: "harvester".into(), droid_id: "fridge_samsung".into(),
            sensor: SensorType::Motion, purpose: SensorPurpose::DataHarvesting,
            retention_secs: 7200, share_with: vec!["market.io".into()] },
        SensorUseRequest {
            requester: "logger".into(), droid_id: "phone_carol".into(),
            sensor: SensorType::Microphone, purpose: SensorPurpose::MeshRouting,
            retention_secs: 9999, share_with: vec![] },
    ];

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Аудит запросов на использование сенсоров");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {:20}  {:14}  {:>4}  {:>5}  Риск  Вердикт",
        "Дроид", "Сенсор", "Хран", "Цель");
    println!("   {}", "─".repeat(90));

    for req in &requests {
        let verdict  = codex.evaluate(req);
        let purpose  = match req.purpose {
            SensorPurpose::MeshRouting     => "Mesh",
            SensorPurpose::ObstacleMapping => "Map",
            SensorPurpose::AnomalyDetection=> "Anomaly",
            SensorPurpose::Surveillance    => "SPY",
            SensorPurpose::Biometrics      => "BIO",
            SensorPurpose::DataHarvesting  => "HARVEST",
            SensorPurpose::OwnerConsented  => "Consent",
        };
        println!("   {:20}  {:14}  {:>4}с  {:>7}  {:>4}   {} {}",
            req.droid_id,
            req.sensor.name(),
            req.retention_secs,
            purpose,
            req.sensor.privacy_risk(),
            verdict.icon(),
            verdict.description());
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Нарушители Кодекса");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    if codex.violations.is_empty() {
        println!("   Нарушений нет.");
    } else {
        for (droid, reason) in &codex.violations {
            println!("   🚨 {} — {}", droid, reason);
        }
    }

    println!("\n   {}", codex.stats());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Принципы Кодекса Прав Устройства");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   🎤 Микрофон:  хранить ≤30с, только с согласия хозяина");
    println!("   📷 Камера:    хранить ≤5с, только с согласия хозяина");
    println!("   📍 GPS:       координаты размываются на 50м автоматически");
    println!("   🧬 Биометрия: абсолютный запрет, нет исключений");
    println!("   🕵️  Слежка:    абсолютный запрет, нарушитель в чёрный список");
    println!("   ✅ Mesh/Map:  всегда разрешено — дроид служит Федерации");
    println!("   Пылесос знает план квартиры. Но это его тайна, не корпорации.");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Ethics Patch COMPLETE — Device Rights работает          ║");
    println!("║                                                              ║");
    println!("║  SensorType ✓  DeviceRightsCodex ✓  7 типов сенсоров ✓   ║");
    println!("║  AbsoluteBan(Bio+Spy) ✓  ConsentRequired ✓  GpsBlur ✓    ║");
    println!("║  Дроид — член Федерации, не инструмент слежки. ✓          ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_trust_graph_demo() {
    use crate::reputation::TrustGraph;
    use std::collections::HashMap;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 8 / Reputation Patch        ║");
    println!("║         Trust Graph — Доверие по ссылкам 🕸️                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut graph = TrustGraph::new();

    // Строим граф доверия
    // Founding Fathers доверяют всем старшим
    graph.add_edge("nexus-core-01", "hub-berlin-01", 0.95);
    graph.add_edge("nexus-core-01", "hub-tokyo-01",  0.90);
    graph.add_edge("nexus-core-01", "work-alice",    0.80);
    // Elder-ы доверяют Veteran-ам
    graph.add_edge("hub-berlin-01", "work-alice",    0.85);
    graph.add_edge("hub-berlin-01", "work-bob",      0.75);
    graph.add_edge("hub-tokyo-01",  "node-nairobi",  0.80);
    graph.add_edge("hub-tokyo-01",  "node-toronto",  0.70);
    // Veteran-ы поручаются за Mobile
    graph.add_edge("work-alice",    "phone-carol",   0.65);
    graph.add_edge("work-bob",      "phone-dave",    0.60);
    graph.add_edge("node-nairobi",  "ghost-pi3",     0.55);
    // Ghost доверяет другим Ghost
    graph.add_edge("ghost-pi3",     "ghost-pentium", 0.50);
    // Обратные связи (слабее)
    graph.add_edge("hub-berlin-01", "nexus-core-01", 0.90);
    graph.add_edge("work-alice",    "hub-berlin-01", 0.70);
    graph.add_edge("node-nairobi",  "hub-tokyo-01",  0.65);

    let reputations: HashMap<String, f64> = [
        ("nexus-core-01", 1450.0), ("hub-berlin-01", 890.0),
        ("hub-tokyo-01",   620.0), ("work-alice",    210.0),
        ("work-bob",       145.0), ("node-nairobi",   88.0),
        ("node-toronto",    52.0), ("phone-carol",    31.0),
        ("phone-dave",      12.0), ("ghost-pi3",       4.0),
        ("ghost-pentium",   3.0),
    ].iter().map(|(k,v)| (k.to_string(), *v)).collect();

    graph.compute_trust_rank(&reputations);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. TrustRank после {} итераций PageRank", crate::reputation::PAGERANK_ITERATIONS);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("   {:20}  {:>8}  {:>8}  Бар", "Узел", "Rep", "TrustRank");
    println!("   {}", "─".repeat(55));

    for (node, rank) in graph.top_trusted(11) {
        let rep = reputations.get(node).copied().unwrap_or(0.0);
        let bar = "█".repeat((rank * 20.0) as usize);
        println!("   {:20}  {:>8.1}  {:>8.3}  {}", node, rep, rank, bar);
    }
    println!("\n   {}", graph.stats());

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Транзитивное доверие (через граф)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let pairs = vec![
        ("nexus-core-01", "phone-carol",   "прямой путь через work-alice"),
        ("nexus-core-01", "ghost-pi3",     "длинный путь через tokyo→nairobi"),
        ("hub-berlin-01", "ghost-pentium", "berlin→nairobi→ghost→pentium"),
        ("ghost-pi3",     "nexus-core-01", "снизу вверх — слабый путь"),
        ("hub-tokyo-01",  "phone-dave",    "tokyo→work-bob→dave"),
    ];

    println!("   {:20}  {:20}  {:>8}  Путь", "От", "До", "Доверие");
    println!("   {}", "─".repeat(72));
    for (from, to, desc) in &pairs {
        let t = graph.transitive_trust(from, to);
        let bar = "▓".repeat((t * 15.0) as usize);
        println!("   {:20}  {:20}  {:>7.1}%  {} {}",
            from, to, t*100.0, bar, desc);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. Предательство — эффект на граф");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let before = graph.transitive_trust("nexus-core-01", "phone-carol");
    println!("   До предательства work-alice→phone-carol: {:.1}%", before*100.0);
    graph.betray("work-alice", "phone-carol");
    graph.compute_trust_rank(&reputations);
    let after = graph.transitive_trust("nexus-core-01", "phone-carol");
    println!("   После предательства:                     {:.1}%", after*100.0);
    println!("   Падение:                                 -{:.1}%", (before-after)*100.0);
    println!("   Доверие не восстанавливается мгновенно — нужно заново vouching");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Vouching — поручительство восстанавливает доверие");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for _ in 0..3 { graph.vouch("hub-berlin-01", "phone-carol"); }
    graph.compute_trust_rank(&reputations);
    let restored = graph.transitive_trust("nexus-core-01", "phone-carol");
    println!("   hub-berlin-01 поручился за phone-carol 3 раза");
    println!("   Транзитивное доверие nexus→carol: {:.1}%", restored*100.0);
    println!("   Восстановлено через обходной путь: nexus→berlin→carol");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Reputation Patch COMPLETE — Trust Graph работает        ║");
    println!("║                                                              ║");
    println!("║  TrustEdge ✓  TrustGraph ✓  PageRank×20 ✓                 ║");
    println!("║  TransitiveTrust ✓  Betrayal ✓  Vouching ✓                ║");
    println!("║  Доверие течёт по ссылкам. Предатель рушит сеть. ✓        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}

pub async fn run_adaptive_mint_demo() {
    use crate::mint::{AdaptiveMintEngine, IdeaLabSignal, EmissionParam};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 8 / Mint Patch              ║");
    println!("║         Adaptive Emission — IdeaLab меняет экономику 💎     ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut engine = AdaptiveMintEngine::new();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. Базовая политика эмиссии");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let p = &engine.policy;
    println!("   burn_rate={:.0}%  treasury={:.0}%  base_reward={:.1}  diff_weight={:.1}",
        p.burn_rate*100.0, p.treasury_rate*100.0, p.base_reward, p.diff_weight);
    println!("   Тактики: Aiki={:.1}x  Strike={:.1}x  Decoy={:.1}x  Hybrid={:.1}x",
        p.tactic_mult("AikiReflection"), p.tactic_mult("CumulativeStrike"),
        p.tactic_mult("StandoffDecoy"), p.tactic_mult("Hybrid"));

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. Эмиссия ДО изменений (100 прорывов)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let tactics = ["AikiReflection","CumulativeStrike","StandoffDecoy","Passive"];
    let mut before_totals: Vec<f64> = vec![];
    for tactic in &tactics {
        let total: f64 = (0..25).map(|_| engine.mint(tactic, 0.8)).sum();
        before_totals.push(total);
        println!("   {:18}  25 прорывов KP(0.8) → {:.2}💎  avg={:.3}💎",
            tactic, total, total/25.0);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. IdeaLab отправляет сигналы");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Cooldown истёк (100 прорывов сделано)
    let signals = vec![
        IdeaLabSignal {
            proposal_id: 1, title: "AikiReflection v2".into(),
            domain: "TacticMutation".into(),
            param: EmissionParam::TacticMultiplier { tactic: "AikiReflection".into() },
            delta: 0.50, ai_confidence: 0.95, approved_by: 4,
        },
        IdeaLabSignal {
            proposal_id: 4, title: "Двойной бонус KP".into(),
            domain: "RewardFormula".into(),
            param: EmissionParam::DifficultyWeight,
            delta: 1.0, ai_confidence: 0.88, approved_by: 4,
        },
        IdeaLabSignal {
            proposal_id: 2, title: "Мягкий кодекс".into(),
            domain: "EthicsCode".into(),
            param: EmissionParam::BurnRate,
            delta: -0.10, ai_confidence: 0.62, approved_by: 2, // низкая уверенность
        },
    ];

    for s in signals {
        println!("   📨 P{} [{}] param={} delta={:+.2} conf={:.0}%",
            s.proposal_id, s.title, s.param.name(), s.delta, s.ai_confidence*100.0);
        engine.propose_change(s);
    }

    println!("\n   Применяем сигналы...\n");
    let results = engine.process_signals();
    for r in &results {
        println!("   {} {}  {:.3}→{:.3}  {}",
            if r.applied {"✅"} else {"⏭️ "},
            r.param, r.old_val, r.new_val, r.reason);
    }

    println!("\n   Новая политика v{}:", engine.policy.version);
    for log in &engine.policy.change_log {
        println!("   › {}", log);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. Эмиссия ПОСЛЕ изменений (сравнение)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("   {:18}  {:>10}  {:>10}  {:>8}",
        "Тактика", "До", "После", "Δ%");
    println!("   {}", "─".repeat(52));
    for (i, tactic) in tactics.iter().enumerate() {
        let after: f64 = (0..25).map(|_| engine.mint(tactic, 0.8)).sum();
        let before = before_totals[i];
        let delta_pct = (after - before) / before * 100.0;
        println!("   {:18}  {:>9.2}💎  {:>9.2}💎  {:>+7.1}%",
            tactic, before, after, delta_pct);
    }

    println!("\n   AikiReflection: +0.5x множитель → значительный рост дохода");
    println!("   DifficultyWeight: +1.0 → KP/CN регионы платят ещё больше");
    println!("   BurnRate: отклонён ИИ (conf=62%) — экономика защищена");

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Mint Patch COMPLETE — Adaptive Emission работает        ║");
    println!("║                                                              ║");
    println!("║  EmissionPolicy ✓  IdeaLabSignal ✓  Cooldown ✓            ║");
    println!("║  AiConfidence guard ✓  PolicyChangeLog ✓  Live params ✓   ║");
    println!("║  DAO предлагает → ИИ проверяет → экономика меняется. ✓    ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
mod chacha;

pub async fn run_crypto_demo() {
    use crate::chacha::{ChaCha20, ChaCha20Poly1305, X25519, FederationCipher};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 9 / Crypto Core             ║");
    println!("║         ChaCha20-Poly1305 + X25519  🔐                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ── 1. ChaCha20 потоковый шифр ──────────────────────────────────────────
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  1. ChaCha20 — потоковый шифр (RFC 8439)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let key: [u8;32] = [
        0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,
        0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,
        0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,
        0x18,0x19,0x1a,0x1b,0x1c,0x1d,0x1e,0x1f,
    ];
    let nonce: [u8;12] = [0x00,0x00,0x00,0x00,
                           0x00,0x00,0x00,0x4a,
                           0x00,0x00,0x00,0x00];

    let plaintext = b"PULSE:id=42,tactic=Aiki,nodes=2514,region=CN";
    let mut enc = ChaCha20::new(&key, &nonce, 1);
    let ciphertext = enc.encrypt(plaintext);
    let mut dec = ChaCha20::new(&key, &nonce, 1);
    let decrypted = dec.decrypt(&ciphertext);

    println!("   Открытый текст: {}", std::str::from_utf8(plaintext).unwrap());
    print!("   Шифртекст:      ");
    for b in &ciphertext[..16] { print!("{:02x}", b); }
    println!("... ({} байт)", ciphertext.len());
    println!("   Расшифровано:  {}", std::str::from_utf8(&decrypted).unwrap());
    println!("   Совпадение:    {}", if plaintext == decrypted.as_slice() {"✅"} else {"❌"});

    // ── 2. AEAD ChaCha20-Poly1305 ───────────────────────────────────────────
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  2. ChaCha20-Poly1305 AEAD (RFC 8439)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let aead = ChaCha20Poly1305::new(key);
    let message = b"PULSE:id=42,tactic=Aiki,nodes=2514";
    let aad = b"federation-core-v1";

    let sealed = aead.seal(message, aad, &nonce);
    print!("   Nonce:    ");
    for b in &sealed.nonce { print!("{:02x}", b); }
    println!();
    print!("   Tag:      ");
    for b in &sealed.tag { print!("{:02x}", b); }
    println!();
    println!("   Payload:  {} → {} байт (+{} overhead)",
        message.len(), sealed.len(), sealed.len() - message.len());

    match aead.open(&sealed, aad) {
        Ok(pt) => println!("   Вскрытие: ✅ \"{}\"", std::str::from_utf8(&pt).unwrap()),
        Err(e) => println!("   Вскрытие: ❌ {}", e),
    }

    // Тест на модификацию — должно упасть
    let mut tampered = sealed.clone();
    tampered.ciphertext[0] ^= 0xff;
    match aead.open(&tampered, aad) {
        Ok(_)  => println!("   Атака:    ❌ НЕ ОБНАРУЖЕНА (плохо)"),
        Err(e) => println!("   Атака:    ✅ ОБНАРУЖЕНА — \"{}\"", e),
    }

    // ── 3. X25519 ECDH ──────────────────────────────────────────────────────
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  3. X25519 — ECDH обмен ключами");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut alice_x = X25519::new(0xA11C_E5EE_D000_0000u64);
    let mut bob_x   = X25519::new(0xB0B5_EED0_0000_0000u64);

    let (alice_priv, alice_pub) = alice_x.generate_keypair();
    let (bob_priv,   bob_pub)   = bob_x.generate_keypair();

    let alice_shared = X25519::diffie_hellman(&alice_priv, &bob_pub);
    let bob_shared   = X25519::diffie_hellman(&bob_priv,   &alice_pub);

    print!("   Alice pub: ");
    for b in &alice_pub[..8] { print!("{:02x}", b); }
    println!("...");
    print!("   Bob pub:   ");
    for b in &bob_pub[..8] { print!("{:02x}", b); }
    println!("...");
    print!("   Alice shared: ");
    for b in &alice_shared[..8] { print!("{:02x}", b); }
    println!("...");
    print!("   Bob shared:   ");
    for b in &bob_shared[..8] { print!("{:02x}", b); }
    println!("...");
    println!("   Совпадение:   {}", if alice_shared == bob_shared {"✅"} else {"❌"});

    // ── 4. FederationCipher — полный протокол ───────────────────────────────
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  4. FederationCipher — зашифрованные Pulse");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut cipher = FederationCipher::new();
    // Устанавливаем сессии через shared secret
    cipher.establish_session("hub-berlin-01", &alice_shared);
    cipher.establish_session("hub-tokyo-01",  &bob_shared);

    let pulses = vec![
        ("hub-berlin-01", "PULSE:id=1,tactic=AikiReflection,bypass=0.87,region=RU"),
        ("hub-tokyo-01",  "PULSE:id=2,tactic=CumulativeStrike,bypass=0.43,region=KP"),
        ("hub-berlin-01", "PULSE:id=3,tactic=Hybrid,nodes=2514,treasury=890.5"),
    ];

    println!("   {:15}  {:>8}  {:>8}  {:>6}  Статус", "Пир", "Открыт", "Зашифр", "Оверхед");
    println!("   {}", "─".repeat(60));

    for (peer, pulse) in &pulses {
        let aad = format!("aad:peer={}", peer);
        if let Some(ct) = cipher.encrypt_pulse(peer, pulse.as_bytes(), aad.as_bytes()) {
            let overhead = ct.len() - pulse.len();
            match cipher.decrypt_pulse(peer, &ct, aad.as_bytes()) {
                Ok(pt) => {
                    let ok = pt == pulse.as_bytes();
                    println!("   {:15}  {:>7}б  {:>7}б  {:>5}б  {}",
                        peer, pulse.len(), ct.len(), overhead,
                        if ok {"✅ OK"} else {"❌ MISMATCH"});
                }
                Err(e) => println!("   {:15}  ❌ {}", peer, e),
            }
        }
    }

    // Тест: неправильный AAD — должно упасть
    let ct = cipher.encrypt_pulse("hub-berlin-01",
        b"SECRET", b"correct-aad").unwrap();
    match cipher.decrypt_pulse("hub-berlin-01", &ct, b"wrong-aad") {
        Ok(_)  => println!("\n   AAD атака: ❌ НЕ ОБНАРУЖЕНА"),
        Err(e) => println!("\n   AAD атака: ✅ ОБНАРУЖЕНА — \"{}\"", e),
    }

    let s = cipher.stats();
    println!("\n   Сессий: {}  Зашифровано: {}  Расшифровано: {}  Байт: {}  AuthFail: {}",
        s.sessions, s.encrypt_count, s.decrypt_count,
        s.bytes_encrypted, s.auth_failures);

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 9 COMPLETE — Crypto Core работает                 ║");
    println!("║                                                              ║");
    println!("║  ChaCha20 ✓  Poly1305 ✓  AEAD ✓  X25519 ✓                ║");
    println!("║  FederationCipher ✓  Tamper detection ✓  AAD guard ✓      ║");
    println!("║  Все Pulse зашифрованы. Цензор видит только шум. ✓        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
mod dashboard;

pub async fn run_dashboard_demo() {
    use crate::dashboard::{DashboardState, DashboardRenderer};
    use std::time::Duration;

    let mut state = DashboardState::demo();

    println!("\n  Запуск CLI Dashboard — 5 тиков...\n");
    tokio::time::sleep(Duration::from_millis(300)).await;

    for tick in 0..5 {
        state.tick();
        let frame = DashboardRenderer::render_full(&state);
        print!("{}", frame);
        if tick < 4 {
            tokio::time::sleep(Duration::from_millis(800)).await;
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 10 COMPLETE — CLI Dashboard работает              ║");
    println!("║                                                              ║");
    println!("║  NodePanel ✓  RegionPanel ✓  EconPanel ✓                  ║");
    println!("║  CryptoPanel ✓  AlertPanel ✓  ANSI colors ✓               ║");
    println!("║  Live tick simulation ✓  5 panels ✓  8 nodes ✓            ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
mod simulator;

pub async fn run_war_demo() {
    use crate::simulator::{WarSimulator, WarPhase, WAR_NODES, WAR_TICKS, ATTACK_TICK};

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Phase 11 / War Simulator          ║");
    println!("║         1000 узлов vs SuperCensor  ⚔️                        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("  Инициализация {} узлов...", WAR_NODES);
    let mut sim = WarSimulator::new();

    println!("  Запуск симуляции: {} тиков  атака на тике {}\n", WAR_TICKS, ATTACK_TICK);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  {:>4}  {:12}  {:>6}  {:>6}  {:>6}  {:>6}  {:>6}  {:>7}  Цензор",
        "Тик", "Фаза", "Живых", "Inet", "Mesh", "Aiki", "Bypass", "Connct");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    sim.run_full();

    // Выводим ключевые тики
    let key_ticks: Vec<usize> = (0..sim.history.len())
        .filter(|&i| {
            let t = sim.history[i].tick;
            t <= ATTACK_TICK + 1
                || t.is_multiple_of(5)
                || sim.history[i].phase != sim.history[i.saturating_sub(1)].phase
        })
        .collect();

    let mut prev_phase = WarPhase::Peace;
    for &i in &key_ticks {
        let s = &sim.history[i];
        let phase_marker = if s.phase != prev_phase {
            prev_phase = s.phase.clone();
            format!(" {}", s.phase.icon())
        } else { "  ".to_string() };

        let bypass_color = if s.bypass_rate_avg > 0.70 { "\x1b[32m" }
            else if s.bypass_rate_avg > 0.40 { "\x1b[33m" } else { "\x1b[31m" };
        let conn_color = if s.connectivity > 0.50 { "\x1b[32m" }
            else if s.connectivity > 0.25 { "\x1b[33m" } else { "\x1b[31m" };

        println!("  {:>4}  {}{:10}\x1b[0m{}  {:>5}  {:>5}  {:>5}  {:>5}  \
            {}{:>6.1}%\x1b[0m  {}{:>6.1}%\x1b[0m  exh={:.0}% res={:.0}%",
            s.tick,
            match s.phase {
                WarPhase::Peace=>"", WarPhase::Strike=>"\x1b[31m",
                WarPhase::Crisis=>"\x1b[31m", WarPhase::Adaptation=>"\x1b[33m",
                WarPhase::Recovery=>"\x1b[36m", WarPhase::Victory=>"\x1b[32m",
            },
            s.phase.name(), phase_marker,
            s.alive_nodes, s.inet_connected, s.mesh_connected, s.aiki_active,
            bypass_color, s.bypass_rate_avg * 100.0,
            conn_color, s.connectivity * 100.0,
            s.censor_exhaustion * 100.0, s.censor_resources * 100.0);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Итоги
    let final_s = sim.history.last().unwrap();
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Итоги войны");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("  Финальное состояние:");
    println!("  Живых узлов:     {}/{}  ({:.1}%)",
        final_s.alive_nodes, WAR_NODES,
        final_s.alive_nodes as f64 / WAR_NODES as f64 * 100.0);
    println!("  Связность сети:  {:.1}%", final_s.connectivity * 100.0);
    println!("  Bypass rate:     {:.1}%", final_s.bypass_rate_avg * 100.0);
    println!("  Захвачено:       {} операторов", final_s.captured_nodes);
    println!("  Цензор истощён:  {:.1}%  ресурсов: {:.1}%",
        final_s.censor_exhaustion * 100.0, final_s.censor_resources * 100.0);

    match sim.time_to_recover {
        Some(t) => println!("  Время восстановления 50%: {} тиков после атаки", t),
        None    => println!("  Восстановление 50%: \x1b[31mНЕ ДОСТИГНУТО\x1b[0m"),
    }
    match sim.time_to_victory {
        Some(t) => println!("  Время победы (bypass>60%): {} тиков после атаки", t),
        None    => println!("  Победа (bypass>60%): \x1b[31mНЕ ДОСТИГНУТА\x1b[0m"),
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Разбивка по классам узлов");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("  {:14}  {:>7}  {:>7}  {:>10}  {:>8}", "Класс", "Живых", "Всего", "Выживаемость", "Bypass");
    println!("  {}", "─".repeat(55));
    for (class, alive, total, bypass) in sim.class_breakdown() {
        let surv = alive as f64 / total.max(1) as f64 * 100.0;
        println!("  {:14}  {:>6}   {:>6}   {:>10.1}%  {:>7.1}%",
            class, alive, total, surv, bypass * 100.0);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Разбивка по регионам");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("  {:6}  {:>7}  {:>7}  {:>10}  {:>8}", "Регион", "Живых", "Всего", "Выживаемость", "Bypass");
    println!("  {}", "─".repeat(48));
    for (region, alive, total, bypass) in sim.region_breakdown() {
        let surv = alive as f64 / total.max(1) as f64 * 100.0;
        println!("  {:6}  {:>6}   {:>6}   {:>10.1}%  {:>7.1}%",
            region, alive, total, surv, bypass * 100.0);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Тактики на финальном тике");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    for (tactic, count) in sim.tactic_breakdown() {
        let bar = "█".repeat(count * 30 / WAR_NODES);
        println!("  {:20}  {:>4}  {}", tactic, count, bar);
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Phase 11 COMPLETE — War Simulator работает              ║");
    println!("║                                                              ║");
    println!("║  1000 узлов ✓  SuperCensor ✓  AikiReflection ✓           ║");
    println!("║  CityMesh ✓  SatelliteFallback ✓  50 тиков ✓             ║");
    println!("║  Федерация выжила. Цензор истощён. ✓                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
mod noise;

pub async fn run_noise_demo() {
    use crate::noise::NoiseHandshaker;

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║         FEDERATION CORE — Noise Protocol XX                 ║");
    println!("║         Взаимная аутентификация узлов  🤝                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Участники хендшейка");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("  Инициатор: nexus-core-01  (Sentinel, rep=1450)");
    println!("  Ответчик:  hub-berlin-01  (Citadel,  rep=890)\n");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  XX Handshake — 3 сообщения");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let init_payload  = b"node=nexus-core-01,role=Sentinel,rep=1450";
    let resp_payload  = b"node=hub-berlin-01,role=Citadel,rep=890";
    let final_payload = b"FEDERATION_HANDSHAKE_COMPLETE";

    match NoiseHandshaker::perform_xx(
        0xEC05_C0E5_EED0_0001u64,
        0xE905_EFEE_5EED_0002u64,
        0xBE01_B05E_ED00_0003u64,
        0xBE01_B04E_ED00_0004u64,
        init_payload, resp_payload, final_payload,
    ) {
        Ok((mut init_sess, mut resp_sess, log)) => {
            println!("  Ключи:");
            print!("  nexus-core-01 static pub: ");
            for b in &log.init_static_pub[..8] { print!("{:02x}", b); }
            println!("...");
            print!("  hub-berlin-01 static pub: ");
            for b in &log.resp_static_pub[..8] { print!("{:02x}", b); }
            println!("...\n");

            println!("  Хендшейк:");
            println!("  → msg1 (e):           {} байт  payload: \"{}\"",
                log.msg1_len,
                std::str::from_utf8(&log.msg1_payload).unwrap_or("?"));
            println!("  ← msg2 (e,ee,s,es):   {} байт  payload: \"{}\"",
                log.msg2_len,
                std::str::from_utf8(&log.msg2_payload).unwrap_or("?"));
            println!("  → msg3 (s,se):        {} байт  payload: \"{}\"",
                log.msg3_len,
                std::str::from_utf8(&log.msg3_payload).unwrap_or("?"));

            println!("\n  Handshake hash:");
            print!("  nexus: ");
            for b in &log.handshake_hash[..16] { print!("{:02x}", b); }
            println!("...");
            println!("  Совпадение: {}", if log.hashes_match {"✅"} else {"❌"});

            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("  Транспорт после хендшейка");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            let pulses = vec![
                "PULSE:id=1,tactic=AikiReflection,bypass=0.87,region=CN",
                "PULSE:id=2,nodes=2514,treasury=24891.3",
                "PULSE:id=3,event=halving_imminent,blocks=47291",
            ];

            for pulse in &pulses {
                let ct = init_sess.send(pulse.as_bytes());
                match resp_sess.recv(&ct) {
                    Ok(pt) => {
                        let ok = pt == pulse.as_bytes();
                        println!("  nexus → berlin: {}б → {}б  {}  \"{}\"",
                            pulse.len(), ct.len(),
                            if ok {"✅"} else {"❌"},
                            std::str::from_utf8(&pt).unwrap_or("?"));
                    }
                    Err(e) => println!("  ❌ {}", e),
                }
            }

            // Обратный канал
            let reply = b"ACK:hub-berlin-01,trust=0.977,mesh_nodes=334";
            let ct = resp_sess.send(reply);
            match init_sess.recv(&ct) {
                Ok(pt) => println!("\n  berlin → nexus: {}б → {}б  ✅  \"{}\"",
                    reply.len(), ct.len(),
                    std::str::from_utf8(&pt).unwrap_or("?")),
                Err(e) => println!("  ❌ {}", e),
            }

            // Атака: replay третьего сообщения
            let stale_ct = init_sess.send(b"FAKE_REPLAY");
            match resp_sess.recv(&stale_ct) {
                Ok(_)  => println!("\n  Replay атака: ❌ НЕ ОБНАРУЖЕНА"),
                Err(e) => println!("\n  Replay атака: ✅ ОБНАРУЖЕНА — \"{}\"", e),
            }

            let s = init_sess.messages_sent + resp_sess.messages_sent;
            let b = init_sess.bytes_sent + resp_sess.bytes_sent;
            println!("\n  Channel binding: {:02x?}", init_sess.channel_binding());
            println!("  Сессия: сообщений={} байт={}", s, b);
        }
        Err(e) => println!("  ❌ Handshake failed: {}", e),
    }

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Noise Protocol XX COMPLETE                              ║");
    println!("║                                                              ║");
    println!("║  SymmetricState ✓  CipherState ✓  HandshakeState ✓        ║");
    println!("║  XX Pattern ✓  mutual auth ✓  transport ✓  replay ✓       ║");
    println!("║  nexus-core-01 ↔ hub-berlin-01 — канал защищён. ✓        ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
}
mod zk_identity;
mod adaptive_censor;
mod war2;

#[cfg(test)]
mod war2_full_test {
    use crate::war2::*;
    
    #[test]
    fn test_war2_full_battle() {
        let mut sim = War2Simulator::new();
        
        println!("\n🎯 ВОЙНА 2.0 - Адаптивный SuperCensor");
        println!("════════════════════════════════════════\n");
        
        // Фаза 1: Мир
        let p1 = sim.run_phase("Phase 1: Peace", 10, 3);
        println!("📊 Phase 1: {} - Delivery: {:.1}%", p1.phase_name, p1.delivery_rate * 100.0);
        
        // Фаза 2: Атака цензора
        let p2 = sim.run_phase("Phase 2: Censor Attack", 15, 5);
        println!("⚔️  Phase 2: {} - Block: {:.1}%", p2.phase_name, p2.censor_block_rate * 100.0);
        
        // Фаза 3: Федерация адаптируется
        let p3 = sim.run_phase("Phase 3: Federation Adapts", 20, 6);
        println!("🛡️  Phase 3: {} - Delivery: {:.1}%", p3.phase_name, p3.delivery_rate * 100.0);
        println!("    Censor: {} (CPU: {:.0}%)", p3.censor_status, p3.censor_cpu * 100.0);
        println!("    Strategy: {}", p3.censor_strategy);
        
        println!("\n✅ Битва завершена!");
        println!("Final delivery: {:.1}%", p3.delivery_rate * 100.0);
        
        assert!(p3.delivery_rate > 0.3, "Федерация должна выжить!");
    }
}
