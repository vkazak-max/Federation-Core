// =============================================================================
// FEDERATION CORE — dashboard.rs
// PHASE 10 — CLI Dashboard
// Живой терминал мониторинга Федерации
//
// Панели:
//   NodePanel    — статус узлов сети
//   BypassPanel  — bypass rate по регионам
//   EconPanel    — $PULSE экономика
//   DaoPanel     — активные голосования
//   CryptoPanel  — статистика шифрования
//   AlertPanel   — тревоги и события
// =============================================================================


pub const DASH_WIDTH: usize  = 78;
pub const BAR_WIDTH: usize   = 20;
pub const REFRESH_MS: u64    = 500;

// -----------------------------------------------------------------------------
// Цвета ANSI
// -----------------------------------------------------------------------------

pub struct Color;
impl Color {
    pub const RED:     &'static str = "\x1b[31m";
    pub const GREEN:   &'static str = "\x1b[32m";
    pub const YELLOW:  &'static str = "\x1b[33m";
    pub const BLUE:    &'static str = "\x1b[34m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const CYAN:    &'static str = "\x1b[36m";
    pub const WHITE:   &'static str = "\x1b[37m";
    pub const BOLD:    &'static str = "\x1b[1m";
    pub const DIM:     &'static str = "\x1b[2m";
    pub const RESET:   &'static str = "\x1b[0m";
    pub const BG_DARK: &'static str = "\x1b[40m";
}

// -----------------------------------------------------------------------------
// Утилиты отрисовки
// -----------------------------------------------------------------------------

pub fn bar(value: f64, max: f64, width: usize, color: &str) -> String {
    let filled = ((value / max.max(0.001)) * width as f64) as usize;
    let filled = filled.min(width);
    let empty  = width - filled;
    format!("{}{}{}{}{}",
        color,
        "█".repeat(filled),
        Color::DIM,
        "░".repeat(empty),
        Color::RESET)
}

pub fn hline(width: usize) -> String {
    format!("{}{}{}",
        Color::DIM, "─".repeat(width), Color::RESET)
}

pub fn header(title: &str) -> String {
    let inner = DASH_WIDTH - title.len() - 4;
    let line = "═".repeat(DASH_WIDTH - 2);
    let spaces = " ".repeat(inner);
    format!("{}╔{}╗{}\n{}║ {}{}{}{}║{}\n{}╚{}╝{}",
        Color::BLUE, line, Color::RESET,
        Color::BLUE, Color::BOLD, Color::WHITE, title,
        spaces,
        Color::RESET,
        Color::BLUE, line, Color::RESET)
}

pub fn panel_header(title: &str, color: &str) -> String {
    format!("{}┌─ {}{}{} {}{}",
        Color::DIM, color, Color::BOLD, title, Color::RESET,
        format!("{}{}{}", Color::DIM,
            "─".repeat(DASH_WIDTH.saturating_sub(title.len() + 5)),
            Color::RESET))
}

// -----------------------------------------------------------------------------
// NodeSnapshot — снимок состояния узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NodeSnapshot {
    pub id: String,
    pub role: String,
    pub region: String,
    pub online: bool,
    pub bypass_rate: f64,
    pub reputation: f64,
    pub pulse_balance: f64,
    pub cpu_load: f64,
    pub uptime_days: u32,
    pub current_tactic: String,
    pub hw_age_years: u32,
    pub trust_rank: f64,
}

// -----------------------------------------------------------------------------
// RegionSnapshot — состояние региона
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RegionSnapshot {
    pub code: String,
    pub censor_strength: f64,
    pub bypass_rate: f64,
    pub active_nodes: u32,
    pub pulses_today: u64,
    pub trend: Trend,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Trend { Up, Down, Stable }

impl Trend {
    pub fn icon(&self) -> &str {
        match self {
            Trend::Up     => "↑",
            Trend::Down   => "↓",
            Trend::Stable => "→",
        }
    }
    pub fn color(&self) -> &str {
        match self {
            Trend::Up     => Color::GREEN,
            Trend::Down   => Color::RED,
            Trend::Stable => Color::YELLOW,
        }
    }
}

// -----------------------------------------------------------------------------
// DashboardState — полное состояние дашборда
// -----------------------------------------------------------------------------

pub struct DashboardState {
    pub nodes: Vec<NodeSnapshot>,
    pub regions: Vec<RegionSnapshot>,
    pub total_pulse_supply: f64,
    pub treasury: f64,
    pub burn_total: f64,
    pub active_proposals: u32,
    pub encrypt_count: u64,
    pub auth_failures: u64,
    pub mesh_nodes: u32,
    pub satellite_active: bool,
    pub uptime_secs: u64,
    pub alerts: Vec<String>,
    pub tick: u64,
}

impl DashboardState {
    pub fn demo() -> Self {
        let nodes = vec![
            NodeSnapshot { id:"nexus-core-01".into(), role:"Sentinel".into(),
                region:"DE".into(), online:true, bypass_rate:0.94,
                reputation:1450.0, pulse_balance:8920.5, cpu_load:0.25,
                uptime_days:412, current_tactic:"AikiReflection".into(),
                hw_age_years:1, trust_rank:0.690 },
            NodeSnapshot { id:"hub-berlin-01".into(), role:"Citadel".into(),
                region:"DE".into(), online:true, bypass_rate:0.91,
                reputation:890.0, pulse_balance:4210.3, cpu_load:0.38,
                uptime_days:287, current_tactic:"Hybrid".into(),
                hw_age_years:2, trust_rank:0.977 },
            NodeSnapshot { id:"hub-tokyo-01".into(), role:"Citadel".into(),
                region:"JP".into(), online:true, bypass_rate:0.88,
                reputation:620.0, pulse_balance:2890.1, cpu_load:0.45,
                uptime_days:201, current_tactic:"CumulativeStrike".into(),
                hw_age_years:3, trust_rank:0.974 },
            NodeSnapshot { id:"work-alice".into(), role:"Workstation".into(),
                region:"RU".into(), online:true, bypass_rate:0.79,
                reputation:210.0, pulse_balance:890.7, cpu_load:0.55,
                uptime_days:98, current_tactic:"AikiReflection".into(),
                hw_age_years:4, trust_rank:0.851 },
            NodeSnapshot { id:"node-nairobi".into(), role:"Workstation".into(),
                region:"KE".into(), online:true, bypass_rate:0.82,
                reputation:88.0, pulse_balance:312.4, cpu_load:0.42,
                uptime_days:67, current_tactic:"StandoffDecoy".into(),
                hw_age_years:5, trust_rank:0.833 },
            NodeSnapshot { id:"ghost-pi3".into(), role:"Ghost".into(),
                region:"CN".into(), online:true, bypass_rate:0.61,
                reputation:4.0, pulse_balance:89.2, cpu_load:0.80,
                uptime_days:12, current_tactic:"StandoffDecoy".into(),
                hw_age_years:8, trust_rank:0.716 },
            NodeSnapshot { id:"ghost-pentium".into(), role:"Ghost".into(),
                region:"RU".into(), online:false, bypass_rate:0.0,
                reputation:3.0, pulse_balance:45.1, cpu_load:0.0,
                uptime_days:0, current_tactic:"—".into(),
                hw_age_years:12, trust_rank:1.000 },
            NodeSnapshot { id:"phone-carol".into(), role:"Mobile".into(),
                region:"IR".into(), online:true, bypass_rate:0.55,
                reputation:31.0, pulse_balance:78.9, cpu_load:0.65,
                uptime_days:5, current_tactic:"StandoffDecoy".into(),
                hw_age_years:3, trust_rank:0.740 },
        ];

        let regions = vec![
            RegionSnapshot { code:"CN".into(), censor_strength:0.95,
                bypass_rate:0.63, active_nodes:142, pulses_today:8914, trend:Trend::Up },
            RegionSnapshot { code:"KP".into(), censor_strength:0.99,
                bypass_rate:0.41, active_nodes:8, pulses_today:291, trend:Trend::Up },
            RegionSnapshot { code:"RU".into(), censor_strength:0.75,
                bypass_rate:0.79, active_nodes:334, pulses_today:22104, trend:Trend::Stable },
            RegionSnapshot { code:"IR".into(), censor_strength:0.85,
                bypass_rate:0.71, active_nodes:67, pulses_today:4892, trend:Trend::Down },
            RegionSnapshot { code:"DE".into(), censor_strength:0.20,
                bypass_rate:0.96, active_nodes:891, pulses_today:41203, trend:Trend::Stable },
        ];

        DashboardState {
            nodes, regions,
            total_pulse_supply: 847_291.5,
            treasury: 24_891.3,
            burn_total: 312_048.2,
            active_proposals: 3,
            encrypt_count: 1_247_891,
            auth_failures: 14,
            mesh_nodes: 2514,
            satellite_active: true,
            uptime_secs: 35712841,
            alerts: vec![
                "⚠️  ghost-pentium оффлайн 2ч".into(),
                "🎯 CN bypass +12% после AikiReflection v2".into(),
                "🔐 14 auth failures отклонено".into(),
                "💎 Халвинг через 47,291 прорывов".into(),
            ],
            tick: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        // Симуляция изменений
        let t = self.tick;
        for node in &mut self.nodes {
            if node.online {
                let noise = (t as f64 * 0.17 + node.reputation * 0.01).sin() * 0.02;
                node.bypass_rate = (node.bypass_rate + noise).clamp(0.1, 0.99);
                node.cpu_load = (node.cpu_load + noise * 0.5).clamp(0.05, 0.99);
            }
        }
        for region in &mut self.regions {
            let noise = (t as f64 * 0.13 + region.censor_strength).cos() * 0.01;
            region.bypass_rate = (region.bypass_rate + noise).clamp(0.1, 0.99);
        }
        self.uptime_secs += 1;
        self.encrypt_count += 847;
    }
}

// -----------------------------------------------------------------------------
// DashboardRenderer — отрисовка панелей
// -----------------------------------------------------------------------------

pub struct DashboardRenderer;

impl DashboardRenderer {

    pub fn render_header(state: &DashboardState) -> String {
        let days = state.uptime_secs / 86400;
        let hrs  = (state.uptime_secs % 86400) / 3600;
        let mins = (state.uptime_secs % 3600) / 60;
        let secs = state.uptime_secs % 60;
        format!(
            "\x1b[2J\x1b[H\
            {bold}{blue}╔{line}╗{reset}\n\
            {blue}║{reset}  {bold}{white}⚡ FEDERATION CORE{reset}  {dim}nexus-core-01{reset}\
              {dim}  uptime {days}d {hrs:02}:{mins:02}:{secs:02}  tick={tick}{reset}\
              {blue}{pad}║{reset}\n\
            {blue}╚{line}╝{reset}",
            bold=Color::BOLD, blue=Color::BLUE, reset=Color::RESET,
            white=Color::WHITE, dim=Color::DIM,
            line="═".repeat(DASH_WIDTH-2),
            days=days, hrs=hrs, mins=mins, secs=secs, tick=state.tick,
            pad=" ".repeat(14),
        )
    }

    pub fn render_nodes(state: &DashboardState) -> String {
        let mut out = format!("{}\n", panel_header("NODES", Color::CYAN));
        out += &format!("  {dim}{:18}  {:10}  {:4}  {:6}  {:8}  {:>6}  {:>5}  Tactic{reset}\n",
            "ID", "Role", "Reg", "CPU%", "Bypass", "Rep", "Trust",
            dim=Color::DIM, reset=Color::RESET);
        out += &format!("  {}\n", hline(DASH_WIDTH - 2));

        for n in &state.nodes {
            let status = if n.online {
                format!("{}●{}", Color::GREEN, Color::RESET)
            } else {
                format!("{}●{}", Color::RED, Color::RESET)
            };
            let bypass_color = if n.bypass_rate > 0.80 { Color::GREEN }
                else if n.bypass_rate > 0.60 { Color::YELLOW } else { Color::RED };
            let cpu_color = if n.cpu_load < 0.60 { Color::GREEN }
                else if n.cpu_load < 0.80 { Color::YELLOW } else { Color::RED };
            let eco = if n.hw_age_years >= 7 { "🟤" }
                else if n.hw_age_years >= 3 { "🟡" } else { "" };

            out += &format!("  {} {}{:17}{}  {:10}  {:>3}  {}{:5.0}%{}  {}{:5.1}%{}  {:>7.0}  {:>5.3}  {}{}\n",
                status, Color::BOLD, &n.id[..n.id.len().min(17)], Color::RESET,
                n.role, n.region,
                cpu_color, n.cpu_load*100.0, Color::RESET,
                bypass_color, n.bypass_rate*100.0, Color::RESET,
                n.reputation, n.trust_rank,
                n.current_tactic, eco);
        }
        out
    }

    pub fn render_regions(state: &DashboardState) -> String {
        let mut out = format!("{}\n", panel_header("REGIONS", Color::MAGENTA));
        out += &format!("  {dim}{:4}  {:>7}  {:>7}  {:>20}  {:>8}  {:>10}  Trend{reset}\n",
            "Reg", "Censor", "Bypass", "Bypass Bar", "Nodes", "Pulses/day",
            dim=Color::DIM, reset=Color::RESET);
        out += &format!("  {}\n", hline(DASH_WIDTH - 2));

        for r in &state.regions {
            let bypass_color = if r.bypass_rate > 0.80 { Color::GREEN }
                else if r.bypass_rate > 0.60 { Color::YELLOW } else { Color::RED };
            let censor_color = if r.censor_strength > 0.85 { Color::RED }
                else if r.censor_strength > 0.50 { Color::YELLOW } else { Color::GREEN };
            out += &format!("  {}{:4}{}  {}{:6.0}%{}  {}{:6.1}%{}  {}  {:>8}  {:>10}  {}{}{}\n",
                Color::BOLD, r.code, Color::RESET,
                censor_color, r.censor_strength*100.0, Color::RESET,
                bypass_color, r.bypass_rate*100.0, Color::RESET,
                bar(r.bypass_rate, 1.0, BAR_WIDTH, bypass_color),
                r.active_nodes, r.pulses_today,
                r.trend.color(), r.trend.icon(), Color::RESET);
        }
        out
    }

    pub fn render_econ(state: &DashboardState) -> String {
        let mut out = format!("{}\n", panel_header("$PULSE ECONOMICS", Color::YELLOW));
        let max_supply = 21_000_000.0f64;
        let supply_pct = state.total_pulse_supply / max_supply * 100.0;
        out += &format!("  Supply  {} {yellow}{:>12.1}💎{reset}  {dim}/ {:.0}  ({:.2}% minted){reset}\n",
            bar(state.total_pulse_supply, max_supply, BAR_WIDTH, Color::YELLOW),
            state.total_pulse_supply, max_supply, supply_pct,
            yellow=Color::YELLOW, reset=Color::RESET, dim=Color::DIM);
        out += &format!("  Treasury  {green}{:>10.1}💎{reset}   \
                         Burned  {red}{:>10.1}💎{reset}   \
                         Proposals  {cyan}{}{reset}\n",
            state.treasury, state.burn_total, state.active_proposals,
            green=Color::GREEN, red=Color::RED, cyan=Color::CYAN, reset=Color::RESET);
        out
    }

    pub fn render_crypto(state: &DashboardState) -> String {
        let mut out = format!("{}\n", panel_header("CRYPTO", Color::GREEN));
        out += &format!("  🔐 Encrypted  {green}{:>12}{reset}   \
                         🛡️  AuthFail  {}{:>6}{reset}   \
                         🛰️  Satellite  {}\n",
            state.encrypt_count,
            if state.auth_failures > 50 { Color::RED } else { Color::GREEN },
            state.auth_failures,
            if state.satellite_active {
                format!("{}ACTIVE{}", Color::GREEN, Color::RESET)
            } else {
                format!("{}OFFLINE{}", Color::RED, Color::RESET)
            },
            green=Color::GREEN, reset=Color::RESET);
        out += &format!("  🤖 Mesh nodes  {cyan}{:>6}{reset}   ChaCha20-Poly1305  \
                         {green}E2E encrypted{reset}\n",
            state.mesh_nodes, cyan=Color::CYAN,
            green=Color::GREEN, reset=Color::RESET);
        out
    }

    pub fn render_alerts(state: &DashboardState) -> String {
        let mut out = format!("{}\n", panel_header("ALERTS", Color::RED));
        for alert in &state.alerts {
            out += &format!("  {}\n", alert);
        }
        out
    }

    pub fn render_full(state: &DashboardState) -> String {
        let mut out = String::new();
        out += &Self::render_header(state);
        out += "\n";
        out += &Self::render_nodes(state);
        out += "\n";
        out += &Self::render_regions(state);
        out += "\n";
        out += &Self::render_econ(state);
        out += "\n";
        out += &Self::render_crypto(state);
        out += "\n";
        out += &Self::render_alerts(state);
        out += &format!("\n  {}[q] quit  [r] refresh  [n] nodes  [e] econ  [d] dao{}\n",
            Color::DIM, Color::RESET);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_codes() {
        let c = Color::GREEN;
        assert!(c.starts_with("\x1b["));
    }

    #[test]
    fn test_bar_rendering() {
        let bar = bar(75.0, 100.0, 20, Color::GREEN);
        assert!(!bar.is_empty());
        assert!(bar.contains("█"));
    }

    #[test]
    fn test_hline() {
        let line = hline(40);
        assert!(!line.is_empty());
    }

    #[test]
    fn test_trend_icon() {
        assert_eq!(Trend::Up.icon(), "↑");
        assert_eq!(Trend::Down.icon(), "↓");
        assert_eq!(Trend::Stable.icon(), "→");
    }

    #[test]
    fn test_dashboard_demo() {
        let state = DashboardState::demo();
        assert!(state.nodes.len() > 0);
        assert!(state.total_pulse_supply > 0.0);
        assert!(state.tick == 0);
    }

    #[test]
    fn test_dashboard_tick() {
        let mut state = DashboardState::demo();
        let tick_before = state.tick;
        state.tick();
        assert_eq!(state.tick, tick_before + 1);
    }

    #[test]
    fn test_render_header() {
        let state = DashboardState::demo();
        let header = DashboardRenderer::render_header(&state);
        assert!(header.contains("FEDERATION"));
    }

    #[test]
    fn test_render_nodes() {
        let state = DashboardState::demo();
        let nodes = DashboardRenderer::render_nodes(&state);
        assert!(nodes.contains("NODE"));
    }

    #[test]
    fn test_render_regions() {
        let state = DashboardState::demo();
        let regions = DashboardRenderer::render_regions(&state);
        assert!(regions.contains("REGION"));
    }

    #[test]
    fn test_render_econ() {
        let state = DashboardState::demo();
        let econ = DashboardRenderer::render_econ(&state);
        assert!(econ.contains("PULSE"));
    }

    #[test]
    fn test_render_crypto() {
        let state = DashboardState::demo();
        let crypto = DashboardRenderer::render_crypto(&state);
        assert!(!crypto.is_empty());
    }

    #[test]
    fn test_render_full() {
        let state = DashboardState::demo();
        let full = DashboardRenderer::render_full(&state);
        assert!(full.contains("FEDERATION"));
        assert!(full.contains("NODE"));
        assert!(full.contains("PULSE"));
    }

    #[test]
    fn test_node_snapshot_structure() {
        let node = NodeSnapshot {
            id: "test-node".to_string(),
            role: "Sentinel".to_string(),
            region: "EU".to_string(),
            online: true,
            bypass_rate: 0.85,
            reputation: 0.9,
            pulse_balance: 100.0,
            cpu_load: 0.5,
            uptime_days: 30,
            current_tactic: "Mirage".to_string(),
            hw_age_years: 2,
            trust_rank: 0.9,
        };
        assert_eq!(node.id, "test-node");
        assert_eq!(node.pulse_balance, 100.0);
        assert_eq!(node.reputation, 0.9);
    }
}
