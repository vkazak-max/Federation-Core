// =============================================================================
// FEDERATION CORE — market.rs
// PHASE 5 / STEP 2 — «Bandwidth Auction Market»
// =============================================================================
//
// Децентрализованный аукцион пропускной способности.
// Пользователи платят Credits за доставку пакетов.
// Узлы конкурируют за заявки — побеждает лучший offer.
//
// Механика:
//   BidRequest  — пользователь хочет доставить пакет
//   NodeOffer   — узел предлагает цену и гарантии
//   Auction     — матчинг заявок и предложений
//   Settlement  — расчёт после доставки
// =============================================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MIN_BID_CREDITS: f64    = 0.1;
pub const MARKET_FEE_RATE: f64    = 0.02;  // 2% комиссия рынка
pub const SLASHING_RATE: f64      = 0.20;  // 20% штраф за провал
pub const PREMIUM_THRESHOLD: f64  = 2.0;   // выше — «премиум» трафик
pub const AUCTION_WINDOW_MS: u64  = 5_000; // окно аукциона 5 сек

// -----------------------------------------------------------------------------
// TrafficTier — класс трафика
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrafficTier {
    Economy,    // дёшево, без гарантий
    Standard,   // средняя цена, базовые гарантии
    Premium,    // дорого, гарантированная доставка
    Armored,    // максимум — StandoffDecoy обязателен
}

impl TrafficTier {
    pub fn base_price(&self) -> f64 {
        match self {
            TrafficTier::Economy  => 0.1,
            TrafficTier::Standard => 0.5,
            TrafficTier::Premium  => 2.0,
            TrafficTier::Armored  => 5.0,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            TrafficTier::Economy  => "Economy",
            TrafficTier::Standard => "Standard",
            TrafficTier::Premium  => "Premium",
            TrafficTier::Armored  => "Armored",
        }
    }
    pub fn requires_tactic(&self) -> Option<&str> {
        match self {
            TrafficTier::Armored  => Some("StandoffDecoy"),
            TrafficTier::Premium  => Some("CumulativeStrike"),
            _                     => None,
        }
    }
}

// -----------------------------------------------------------------------------
// BidRequest — заявка пользователя
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidRequest {
    pub bid_id: u64,
    pub user_id: String,
    pub destination_region: String,
    pub payload_size_kb: u32,
    pub max_price: f64,          // максимум credits готов платить
    pub tier: TrafficTier,
    pub deadline_ms: u64,
    pub submitted_at: i64,
}

impl BidRequest {
    pub fn new(user_id: &str, region: &str, size_kb: u32,
               max_price: f64, tier: TrafficTier, counter: u64) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap().as_millis() as i64;
        BidRequest {
            bid_id: counter,
            user_id: user_id.to_string(),
            destination_region: region.to_string(),
            payload_size_kb: size_kb,
            max_price,
            tier,
            deadline_ms: AUCTION_WINDOW_MS,
            submitted_at: now,
        }
    }
}

// -----------------------------------------------------------------------------
// NodeOffer — предложение узла
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOffer {
    pub offer_id: u64,
    pub node_id: String,
    pub bid_id: u64,
    pub price: f64,              // credits за доставку
    pub tactic: String,
    pub estimated_latency_ms: u32,
    pub success_guarantee: f64,  // вероятность успеха (0..1)
    pub stake: f64,              // залог (будет slash при провале)
    pub region_difficulty: f64,
}

impl NodeOffer {
    pub fn score(&self) -> f64 {
        // Скоринг: дешевле + надёжнее + быстрее = лучше
        let price_score    = 1.0 / self.price.max(0.001);
        let quality_score  = self.success_guarantee;
        let latency_score  = 1.0 / (self.estimated_latency_ms as f64).max(1.0);
        let tactic_bonus   = match self.tactic.as_str() {
            "AikiReflection"   => 1.3,
            "CumulativeStrike" => 1.2,
            "StandoffDecoy"    => 1.1,
            "Hybrid"           => 1.25,
            _                  => 1.0,
        };
        (price_score * 0.4 + quality_score * 0.4 + latency_score * 0.2)
            * tactic_bonus
    }
}

// -----------------------------------------------------------------------------
// AuctionResult — итог аукциона
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionResult {
    pub bid_id: u64,
    pub winner_node: String,
    pub winning_price: f64,
    pub winning_tactic: String,
    pub competing_offers: usize,
    pub market_fee: f64,
    pub node_revenue: f64,
    pub success_guarantee: f64,
}

// -----------------------------------------------------------------------------
// Settlement — расчёт после доставки
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub bid_id: u64,
    pub node_id: String,
    pub delivered: bool,
    pub agreed_price: f64,
    pub actual_paid: f64,
    pub slash_amount: f64,
    pub net_earnings: f64,
    pub reason: String,
}

// -----------------------------------------------------------------------------
// BandwidthMarket — главный объект рынка
// -----------------------------------------------------------------------------

pub struct BandwidthMarket {
    pub bids: HashMap<u64, BidRequest>,
    pub offers: HashMap<u64, Vec<NodeOffer>>,  // bid_id → offers
    pub results: Vec<AuctionResult>,
    pub settlements: Vec<Settlement>,
    pub node_balances: HashMap<String, f64>,
    pub market_treasury: f64,
    pub total_volume: f64,
    counter: u64,
}

impl BandwidthMarket {
    pub fn new() -> Self {
        BandwidthMarket {
            bids: HashMap::new(),
            offers: HashMap::new(),
            results: vec![],
            settlements: vec![],
            node_balances: HashMap::new(),
            market_treasury: 0.0,
            total_volume: 0.0,
            counter: 0,
        }
    }

    pub fn submit_bid(&mut self, user_id: &str, region: &str,
                      size_kb: u32, max_price: f64,
                      tier: TrafficTier) -> u64 {
        self.counter += 1;
        let bid = BidRequest::new(user_id, region, size_kb,
            max_price, tier, self.counter);
        let id = bid.bid_id;
        self.bids.insert(id, bid);
        id
    }

    pub fn submit_offer(&mut self, node_id: &str, bid_id: u64,
                        price: f64, tactic: &str,
                        latency_ms: u32, guarantee: f64,
                        stake: f64, difficulty: f64) {
        self.counter += 1;
        let offer = NodeOffer {
            offer_id: self.counter,
            node_id: node_id.to_string(),
            bid_id, price, tactic: tactic.to_string(),
            estimated_latency_ms: latency_ms,
            success_guarantee: guarantee,
            stake, region_difficulty: difficulty,
        };
        self.offers.entry(bid_id).or_default().push(offer);
    }

    /// Провести аукцион — выбрать победителя
    pub fn run_auction(&mut self, bid_id: u64) -> Option<AuctionResult> {
        let bid = self.bids.get(&bid_id)?.clone();
        let offers = self.offers.get(&bid_id)?;

        // Фильтруем: цена <= max_price
        let mut valid: Vec<&NodeOffer> = offers.iter()
            .filter(|o| o.price <= bid.max_price)
            .collect();

        // Проверяем требования тактики для тира
        if let Some(required_tactic) = bid.tier.requires_tactic() {
            valid.retain(|o| o.tactic == required_tactic
                || o.tactic == "Hybrid");
        }

        if valid.is_empty() { return None; }

        // Победитель — максимальный score
        let winner = valid.iter()
            .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap())?;

        let market_fee = winner.price * MARKET_FEE_RATE;
        let node_revenue = winner.price - market_fee;

        let result = AuctionResult {
            bid_id,
            winner_node: winner.node_id.clone(),
            winning_price: winner.price,
            winning_tactic: winner.tactic.clone(),
            competing_offers: offers.len(),
            market_fee,
            node_revenue,
            success_guarantee: winner.success_guarantee,
        };

        self.results.push(result.clone());
        self.total_volume += winner.price;
        self.market_treasury += market_fee;
        Some(result)
    }

    /// Расчёт после доставки
    pub fn settle(&mut self, bid_id: u64, node_id: &str,
                  delivered: bool, agreed_price: f64,
                  stake: f64) -> Settlement {
        let (actual_paid, slash, reason) = if delivered {
            (agreed_price, 0.0, "Доставлено успешно".into())
        } else {
            let slash = stake * SLASHING_RATE;
            (0.0, slash, "Провал доставки — slash".into())
        };

        let net = actual_paid - slash;
        *self.node_balances.entry(node_id.to_string()).or_insert(0.0) += net;

        let s = Settlement {
            bid_id, node_id: node_id.to_string(),
            delivered, agreed_price, actual_paid,
            slash_amount: slash, net_earnings: net, reason,
        };
        self.settlements.push(s.clone());
        s
    }

    pub fn price_discovery(&self, region: &str) -> PriceStats {
        let relevant: Vec<&AuctionResult> = self.results.iter()
            .filter(|r| {
                self.bids.get(&r.bid_id)
                    .map(|b| b.destination_region == region)
                    .unwrap_or(false)
            }).collect();

        if relevant.is_empty() {
            return PriceStats { region: region.to_string(),
                min: 0.0, max: 0.0, avg: 0.0, count: 0 };
        }

        let prices: Vec<f64> = relevant.iter()
            .map(|r| r.winning_price).collect();
        PriceStats {
            region: region.to_string(),
            min: prices.iter().cloned().fold(f64::MAX, f64::min),
            max: prices.iter().cloned().fold(f64::MIN, f64::max),
            avg: prices.iter().sum::<f64>() / prices.len() as f64,
            count: prices.len(),
        }
    }

    pub fn market_stats(&self) -> MarketStats {
        let total_bids = self.bids.len();
        let matched = self.results.len();
        let fill_rate = if total_bids > 0 {
            matched as f64 / total_bids as f64 } else { 0.0 };

        MarketStats {
            total_bids, matched_bids: matched,
            fill_rate, total_volume: self.total_volume,
            market_treasury: self.market_treasury,
            successful_deliveries: self.settlements.iter()
                .filter(|s| s.delivered).count(),
            total_slashed: self.settlements.iter()
                .map(|s| s.slash_amount).sum(),
        }
    }
}

impl Default for BandwidthMarket { fn default() -> Self { Self::new() } }

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceStats {
    pub region: String,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketStats {
    pub total_bids: usize,
    pub matched_bids: usize,
    pub fill_rate: f64,
    pub total_volume: f64,
    pub market_treasury: f64,
    pub successful_deliveries: usize,
    pub total_slashed: f64,
}

impl std::fmt::Display for MarketStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "╔══════════════════════════════════════════════╗\n\
             ║  BANDWIDTH MARKET — STATS                    ║\n\
             ╠══════════════════════════════════════════════╣\n\
             ║  Заявок:  {:>4}  Исполнено: {:>4}  Fill:{:.0}%  ║\n\
             ║  Объём:   {:>10.3} credits                 ║\n\
             ║  Казна:   {:>10.3} credits                 ║\n\
             ║  Доставок успешных: {:>4}  Slash: {:>8.3}  ║\n\
             ╚══════════════════════════════════════════════╝",
            self.total_bids, self.matched_bids,
            self.fill_rate * 100.0,
            self.total_volume, self.market_treasury,
            self.successful_deliveries, self.total_slashed,
        )
    }
}
