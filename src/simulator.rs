// =============================================================================
// FEDERATION CORE — simulator.rs
// PHASE 11 — WAR SIMULATOR
// 1000 узлов vs SuperCensor
// =============================================================================

use std::collections::HashMap;

pub const WAR_NODES: usize        = 1000;
pub const WAR_TICKS: usize        = 50;
pub const ATTACK_TICK: usize      = 10;
pub const INET_KILL_RATE: f64     = 0.80;
pub const SATELLITE_JAM: f64      = 0.70;
pub const OPERATOR_ARREST: f64    = 0.15;
pub const DPI_BLOCK_RATE: f64     = 0.65;
pub const AIKI_EXHAUST_RATE: f64  = 0.12;
pub const MESH_SPREAD_RATE: f64   = 0.08;
pub const RECOVERY_THRESHOLD: f64 = 0.50;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeClass {
    Sentinel, Citadel, Workstation, Ghost, Mobile, Droid,
}

impl NodeClass {
    pub fn from_id(id: usize) -> Self {
        match id % 100 {
            0..=4   => NodeClass::Sentinel,
            5..=14  => NodeClass::Citadel,
            15..=39 => NodeClass::Workstation,
            40..=69 => NodeClass::Ghost,
            70..=89 => NodeClass::Mobile,
            _       => NodeClass::Droid,
        }
    }
    pub fn inet_resilience(&self) -> f64 {
        match self {
            NodeClass::Sentinel    => 0.95,
            NodeClass::Citadel     => 0.85,
            NodeClass::Workstation => 0.40,
            NodeClass::Ghost       => 0.70,
            NodeClass::Mobile      => 0.60,
            NodeClass::Droid       => 0.80,
        }
    }
    pub fn dpi_resilience(&self) -> f64 {
        match self {
            NodeClass::Sentinel    => 0.90,
            NodeClass::Citadel     => 0.80,
            NodeClass::Workstation => 0.50,
            NodeClass::Ghost       => 0.85,
            NodeClass::Mobile      => 0.45,
            NodeClass::Droid       => 0.75,
        }
    }
    pub fn mesh_capable(&self) -> bool {
        matches!(self, NodeClass::Droid | NodeClass::Ghost | NodeClass::Mobile)
    }
    pub fn name(&self) -> &str {
        match self {
            NodeClass::Sentinel    => "Sentinel",
            NodeClass::Citadel     => "Citadel",
            NodeClass::Workstation => "Workstation",
            NodeClass::Ghost       => "Ghost",
            NodeClass::Mobile      => "Mobile",
            NodeClass::Droid       => "Droid",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarTactic {
    AikiReflection, CityMesh, SatelliteFallback, Dormant, Captured,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarRegion { CN, KP, RU, IR, FREE }

impl WarRegion {
    pub fn from_id(id: usize) -> Self {
        match id % 10 {
            0..=1 => WarRegion::CN,
            2     => WarRegion::KP,
            3..=4 => WarRegion::RU,
            5     => WarRegion::IR,
            _     => WarRegion::FREE,
        }
    }
    pub fn censor_strength(&self) -> f64 {
        match self {
            WarRegion::CN   => 0.95,
            WarRegion::KP   => 0.99,
            WarRegion::RU   => 0.75,
            WarRegion::IR   => 0.85,
            WarRegion::FREE => 0.20,
        }
    }
    pub fn name(&self) -> &str {
        match self { WarRegion::CN=>"CN", WarRegion::KP=>"KP",
            WarRegion::RU=>"RU", WarRegion::IR=>"IR", WarRegion::FREE=>"FREE" }
    }
}

#[derive(Debug, Clone)]
pub struct WarNode {
    pub id: usize,
    pub class: NodeClass,
    pub alive: bool,
    pub inet_connected: bool,
    pub mesh_connected: bool,
    pub bypass_rate: f64,
    pub aiki_active: bool,
    pub tactic: WarTactic,
    pub survived_ticks: u32,
    pub region: WarRegion,
}

impl WarNode {
    pub fn new(id: usize) -> Self {
        WarNode { id, class: NodeClass::from_id(id),
            alive: true, inet_connected: true, mesh_connected: false,
            bypass_rate: 0.85, aiki_active: false,
            tactic: WarTactic::AikiReflection,
            survived_ticks: 0, region: WarRegion::from_id(id) }
    }
}

#[derive(Debug, Clone)]
pub struct CensorState {
    pub active: bool,
    pub inet_kill_coverage: f64,
    pub dpi_effectiveness: f64,
    pub satellite_jam: f64,
    pub exhaustion: f64,
    pub resources: f64,
}

impl CensorState {
    pub fn new() -> Self {
        CensorState { active:false, inet_kill_coverage:0.0,
            dpi_effectiveness:0.0, satellite_jam:0.0,
            exhaustion:0.0, resources:1.0 }
    }
    pub fn activate(&mut self) {
        self.active = true;
        self.inet_kill_coverage = INET_KILL_RATE;
        self.dpi_effectiveness  = DPI_BLOCK_RATE;
        self.satellite_jam      = SATELLITE_JAM;
    }
    pub fn apply_aiki_exhaust(&mut self, aiki_nodes: usize) {
        let exhaust = aiki_nodes as f64 * AIKI_EXHAUST_RATE * 0.01;
        self.exhaustion = (self.exhaustion + exhaust).min(1.0);
        self.resources  = (self.resources  - exhaust * 0.5).max(0.0);
        let factor = 1.0 - self.exhaustion * 0.7;
        self.dpi_effectiveness  = (DPI_BLOCK_RATE * factor).max(0.0);
        self.inet_kill_coverage = (INET_KILL_RATE * (1.0 - self.exhaustion * 0.3)).max(0.0);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarPhase {
    Peace, Strike, Crisis, Adaptation, Recovery, Victory,
}

impl WarPhase {
    pub fn icon(&self) -> &str {
        match self {
            WarPhase::Peace      => "🕊️ ",
            WarPhase::Strike     => "💥",
            WarPhase::Crisis     => "🔥",
            WarPhase::Adaptation => "⚡",
            WarPhase::Recovery   => "🔄",
            WarPhase::Victory    => "✅",
        }
    }
    pub fn name(&self) -> &str {
        match self {
            WarPhase::Peace      => "PEACE",
            WarPhase::Strike     => "STRIKE",
            WarPhase::Crisis     => "CRISIS",
            WarPhase::Adaptation => "ADAPTATION",
            WarPhase::Recovery   => "RECOVERY",
            WarPhase::Victory    => "VICTORY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TickStats {
    pub tick: usize,
    pub alive_nodes: usize,
    pub inet_connected: usize,
    pub mesh_connected: usize,
    pub aiki_active: usize,
    pub satellite_active: usize,
    pub captured_nodes: usize,
    pub bypass_rate_avg: f64,
    pub connectivity: f64,
    pub censor_exhaustion: f64,
    pub censor_resources: f64,
    pub phase: WarPhase,
}

pub struct WarSimulator {
    pub nodes: Vec<WarNode>,
    pub censor: CensorState,
    pub tick: usize,
    pub history: Vec<TickStats>,
    rng: u64,
    pub time_to_recover: Option<usize>,
    pub time_to_victory: Option<usize>,
}

impl WarSimulator {
    pub fn new() -> Self {
        let nodes = (0..WAR_NODES).map(WarNode::new).collect();
        WarSimulator { nodes, censor: CensorState::new(),
            tick: 0, history: Vec::new(),
            rng: 0xFEDE_0000_0000_0000u64,
            time_to_recover: None, time_to_victory: None }
    }

    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng & 0xFFFFFF) as f64 / 0xFFFFFF as f64
    }

    pub fn step(&mut self) {
        self.tick += 1;

        if self.tick == ATTACK_TICK {
            self.censor.activate();
            for i in 0..self.nodes.len() {
                if self.rand() < OPERATOR_ARREST {
                    self.nodes[i].alive = false;
                    self.nodes[i].tactic = WarTactic::Captured;
                }
            }
        }

        if !self.censor.active {
            for node in &mut self.nodes {
                node.survived_ticks += 1;
                node.bypass_rate = 0.88;
            }
        } else {
            // Отключение интернета
            for i in 0..self.nodes.len() {
                if !self.nodes[i].alive { continue; }
                let kill = self.censor.inet_kill_coverage
                    * (1.0 - self.nodes[i].class.inet_resilience());
                if self.rand() < kill { self.nodes[i].inet_connected = false; }
            }
            // DPI
            for i in 0..self.nodes.len() {
                if !self.nodes[i].alive || !self.nodes[i].inet_connected { continue; }
                let dpi = self.censor.dpi_effectiveness
                    * (1.0 - self.nodes[i].class.dpi_resilience());
                if self.rand() < dpi {
                    self.nodes[i].bypass_rate = (self.nodes[i].bypass_rate - 0.25).max(0.0);
                }
            }
            // AikiReflection
            let mut aiki_count = 0usize;
            for i in 0..self.nodes.len() {
                if !self.nodes[i].alive || !self.nodes[i].inet_connected { continue; }
                if self.nodes[i].bypass_rate > 0.3 {
                    self.nodes[i].aiki_active = true;
                    self.nodes[i].tactic = WarTactic::AikiReflection;
                    let rf = 1.0 - self.nodes[i].region.censor_strength() * 0.5;
                    self.nodes[i].bypass_rate = (self.nodes[i].bypass_rate + 0.08 * rf).min(0.95);
                    aiki_count += 1;
                }
            }
            self.censor.apply_aiki_exhaust(aiki_count);

            // CityMesh
            let mesh_count = self.nodes.iter()
                .filter(|n| n.alive && n.class.mesh_capable() && n.inet_connected)
                .count();
            for i in 0..self.nodes.len() {
                if !self.nodes[i].alive || self.nodes[i].inet_connected { continue; }
                if self.nodes[i].class.mesh_capable() {
                    let chance = MESH_SPREAD_RATE * mesh_count as f64 * 0.01;
                    if self.rand() < chance {
                        self.nodes[i].mesh_connected = true;
                        self.nodes[i].tactic = WarTactic::CityMesh;
                        self.nodes[i].bypass_rate = (self.nodes[i].bypass_rate + 0.05).min(0.70);
                    }
                }
            }
            // Спутниковый fallback
            let sat = 1.0 - self.censor.satellite_jam;
            for i in 0..self.nodes.len() {
                if !self.nodes[i].alive || self.nodes[i].inet_connected || self.nodes[i].mesh_connected { continue; }
                if matches!(self.nodes[i].class, NodeClass::Sentinel | NodeClass::Ghost)
                    && self.rand() < sat * 0.3 {
                        self.nodes[i].tactic = WarTactic::SatelliteFallback;
                        self.nodes[i].bypass_rate = (self.nodes[i].bypass_rate + 0.03).min(0.50);
                    }
            }
            // Dormant
            for node in &mut self.nodes {
                if !node.alive { continue; }
                if !node.inet_connected && !node.mesh_connected
                    && !matches!(node.tactic, WarTactic::SatelliteFallback
                        | WarTactic::Captured) {
                    node.tactic = WarTactic::Dormant;
                }
            }
            for node in &mut self.nodes {
                if node.alive { node.survived_ticks += 1; }
            }
        }

        // Статистика
        let alive       = self.nodes.iter().filter(|n| n.alive).count();
        let inet_conn   = self.nodes.iter().filter(|n| n.alive && n.inet_connected).count();
        let mesh_conn   = self.nodes.iter().filter(|n| n.alive && n.mesh_connected).count();
        let aiki_active = self.nodes.iter().filter(|n| n.alive && n.aiki_active).count();
        let sat_active  = self.nodes.iter()
            .filter(|n| n.alive && matches!(n.tactic, WarTactic::SatelliteFallback)).count();
        let captured    = self.nodes.iter()
            .filter(|n| matches!(n.tactic, WarTactic::Captured)).count();
        let bypass_avg  = if alive > 0 {
            self.nodes.iter().filter(|n| n.alive).map(|n| n.bypass_rate).sum::<f64>()
                / alive as f64 } else { 0.0 };
        let connectivity = (inet_conn + mesh_conn) as f64 / WAR_NODES as f64;

        if self.tick > ATTACK_TICK && self.time_to_recover.is_none()
            && connectivity >= RECOVERY_THRESHOLD {
            self.time_to_recover = Some(self.tick - ATTACK_TICK);
        }
        if self.tick > ATTACK_TICK && self.time_to_victory.is_none()
            && bypass_avg >= 0.60 {
            self.time_to_victory = Some(self.tick - ATTACK_TICK);
        }

        let phase = if self.tick < ATTACK_TICK { WarPhase::Peace }
            else if self.tick == ATTACK_TICK { WarPhase::Strike }
            else if connectivity < 0.20 { WarPhase::Crisis }
            else if connectivity < RECOVERY_THRESHOLD { WarPhase::Adaptation }
            else if bypass_avg < 0.60 { WarPhase::Recovery }
            else { WarPhase::Victory };

        self.history.push(TickStats { tick: self.tick, alive_nodes: alive,
            inet_connected: inet_conn, mesh_connected: mesh_conn,
            aiki_active, satellite_active: sat_active, captured_nodes: captured,
            bypass_rate_avg: bypass_avg, connectivity,
            censor_exhaustion: self.censor.exhaustion,
            censor_resources: self.censor.resources, phase });
    }

    pub fn run_full(&mut self) {
        for _ in 0..WAR_TICKS { self.step(); }
    }

    pub fn class_breakdown(&self) -> Vec<(String, usize, usize, f64)> {
        let classes = ["Sentinel","Citadel","Workstation","Ghost","Mobile","Droid"];
        classes.iter().map(|c| {
            let nodes: Vec<&WarNode> = self.nodes.iter()
                .filter(|n| n.class.name() == *c).collect();
            let total = nodes.len();
            let alive = nodes.iter().filter(|n| n.alive).count();
            let bypass = nodes.iter().filter(|n| n.alive)
                .map(|n| n.bypass_rate).sum::<f64>() / alive.max(1) as f64;
            (c.to_string(), alive, total, bypass)
        }).collect()
    }

    pub fn region_breakdown(&self) -> Vec<(String, usize, usize, f64)> {
        let regions = ["CN","KP","RU","IR","FREE"];
        regions.iter().map(|r| {
            let nodes: Vec<&WarNode> = self.nodes.iter()
                .filter(|n| n.region.name() == *r).collect();
            let total = nodes.len();
            let alive = nodes.iter().filter(|n| n.alive).count();
            let bypass = nodes.iter().filter(|n| n.alive)
                .map(|n| n.bypass_rate).sum::<f64>() / alive.max(1) as f64;
            (r.to_string(), alive, total, bypass)
        }).collect()
    }

    pub fn tactic_breakdown(&self) -> Vec<(String, usize)> {
        let mut map: HashMap<String,usize> = HashMap::new();
        for node in &self.nodes {
            let t = match node.tactic {
                WarTactic::AikiReflection   => "AikiReflection",
                WarTactic::CityMesh         => "CityMesh",
                WarTactic::SatelliteFallback=> "Satellite",
                WarTactic::Dormant          => "Dormant",
                WarTactic::Captured         => "Captured",
            };
            *map.entry(t.to_string()).or_insert(0) += 1;
        }
        let mut v: Vec<(String,usize)> = map.into_iter().collect();
        v.sort_by(|a,b| b.1.cmp(&a.1));
        v
    }
}
