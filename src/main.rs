mod tensor;
mod network;
mod p2p;
mod routing;
mod dag;
mod zkp;
mod mirage;
mod overlay;

use overlay::{default_seed_nodes, FederationMVP};
use std::env;

fn parse_args() -> (String, u16) {
    // usage:
    //   cargo run -- node_A 7777
    // or:
    //   cargo run -- node_A
    // or:
    //   cargo run
    let mut args = env::args().skip(1);

    let node_id = args.next().unwrap_or_else(|| "node_LOCAL".to_string());
    let port = args
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(7777);

    (node_id, port)
}

#[tokio::main]
async fn main() {
    // Logging:
    //   RUST_LOG=info cargo run --
    //   RUST_LOG=debug cargo run -- node_A 7777
    //
    // Pokud RUST_LOG není nastaven, dáme rozumný default.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let (node_id, port) = parse_args();

    // Seeds (MVP)
    let seeds = default_seed_nodes();

    // Start Federation MVP node
    let mvp = FederationMVP::new(&node_id, port);

    // (volitelné) – rychlé info do logu
    log::info!("Starting FEDERATION CORE MVP node_id={} port={}", node_id, port);

    // Start all subsystems (TCP listener, heartbeat, loops, bootstrap)
    mvp.start(seeds).await;
}
