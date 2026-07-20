use crate::app::search::score_normalized;
use crate::util::{normalize, pretty_name};
use hypixel::HypixelClient;
use hypixel::models::skyblock::{Bazaar, BazaarProduct};
use hypixel::util::market::{self, BazaarFlip};
use indexmap::IndexMap;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Insert,
    Navigate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Search,
    Detail,
}

#[derive(Debug, Clone)]
pub struct ProductIndexItem {
    pub id: String,
    pub display: String,
    pub norm_display: String,
}

/// Weekly movement needed on both sides before a spread is worth ranking;
/// without it the list is topped by unfillable one-sided books.
pub const MIN_WEEKLY_VOLUME: i64 = 1_000;

#[derive(Debug)]
pub struct BazaarData {
    pub products: IndexMap<String, BazaarProduct>,
    pub last_updated: i64,
    pub index: Vec<ProductIndexItem>,
    /// Viable flips only; absent means illiquid or unprofitable after tax.
    pub flips: HashMap<String, BazaarFlip>,
}

/// What a player can transact at right now.
///
/// `buy_summary` is the ask side and `sell_summary` the bid side, despite the
/// names. Ask exceeds bid at depth 1, but the depth-weighted `quick_status` can
/// invert on a thin book, so [`Prices::spread`] may legitimately be negative.
#[derive(Debug, Clone, Copy)]
pub struct Prices {
    pub instant_buy: f64,
    pub instant_sell: f64,
}

impl Prices {
    pub fn spread(&self) -> f64 {
        self.instant_buy - self.instant_sell
    }

    /// Spread relative to what you receive, so cheap items compare fairly.
    pub fn spread_pct(&self) -> f64 {
        if self.instant_sell.abs() > f64::EPSILON {
            self.spread() / self.instant_sell * 100.0
        } else {
            0.0
        }
    }
}

/// Prefers `quick_status` (depth-weighted, less jumpy), falling back to the book.
pub fn prices(product: &BazaarProduct) -> Option<Prices> {
    if let Some(q) = product.quick_status.as_ref() {
        return Some(Prices {
            instant_buy: q.buy_price,
            instant_sell: q.sell_price,
        });
    }
    let spread = market::bazaar_spread(product)?;
    Some(Prices {
        instant_buy: spread.instant_buy_price,
        instant_sell: spread.instant_sell_price,
    })
}

#[derive(Debug)]
pub struct SearchState {
    pub input: String,
    pub mode: SearchMode,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub needs_filter: bool,
    pub last_input_change: Instant,
    pub sort_by_profit: bool,
}

#[derive(Debug)]
pub struct DetailState {
    pub product_id: Option<String>,
    pub history: VecDeque<(Instant, f64, f64)>, // (time, buy, sell)
    pub show_percent: bool,
    pub show_sma: bool,
    pub show_midline: bool,
    
    refresh_task: Option<JoinHandle<()>>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

#[derive(Debug)]
pub struct App {
    pub view: View,
    pub status: String,
    pub data: BazaarData,
    pub search: SearchState,
    pub detail: DetailState,
    pub update_tx: Option<mpsc::UnboundedSender<BazaarProduct>>,
    client: HypixelClient,
}

impl App {
    pub fn new(client: HypixelClient, bazaar: Bazaar) -> Self {
        let flips: HashMap<String, BazaarFlip> = market::bazaar_flips(&bazaar, MIN_WEEKLY_VOLUME)
            .into_iter()
            .map(|f| (f.product_id.clone(), f))
            .collect();

        let mut products = IndexMap::new();
        for (k, v) in bazaar.products {
            products.insert(k, v);
        }

        let index = products
            .keys()
            .map(|id| {
                let display = pretty_name(id);
                ProductIndexItem {
                    id: id.clone(),
                    display: display.clone(),
                    norm_display: normalize(&display),
                }
            })
            .collect();

        let filtered_indices = (0..products.len()).collect();

        Self {
            view: View::Search,
            status: "Loaded".into(),
            data: BazaarData {
                products,
                last_updated: bazaar.last_updated,
                index,
                flips,
            },
            search: SearchState {
                input: String::new(),
                mode: SearchMode::Insert,
                filtered_indices,
                selected_index: 0,
                needs_filter: true,
                last_input_change: Instant::now(),
                sort_by_profit: false,
            },
            detail: DetailState {
                product_id: None,
                history: VecDeque::with_capacity(256),
                show_percent: false,
                show_sma: true,
                show_midline: false,
                refresh_task: None,
                cancel_tx: None,
            },
            update_tx: None,
            client,
        }
    }

    pub fn set_update_sender(&mut self, tx: mpsc::UnboundedSender<BazaarProduct>) {
        self.update_tx = Some(tx);
    }

    pub fn current_product(&self) -> Option<&BazaarProduct> {
        self.detail.product_id.as_ref().and_then(|id| self.data.products.get(id))
    }

    pub fn on_input(&mut self, ch: char) {
        self.search.input.push(ch);
        self.mark_input_changed();
    }

    pub fn on_backspace(&mut self) {
        self.search.input.pop();
        self.mark_input_changed();
    }

    pub fn on_delete(&mut self) {
        self.search.input.clear();
        self.mark_input_changed();
    }

    fn mark_input_changed(&mut self) {
        self.search.needs_filter = true;
        self.search.last_input_change = Instant::now();
    }

    pub fn maybe_apply_filter(&mut self, debounce: Duration) {
        if self.search.needs_filter && self.search.last_input_change.elapsed() >= debounce {
            self.apply_filter();
            self.search.needs_filter = false;
        }
    }

    pub fn recompute_filter(&mut self) {
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        if self.search.input.trim().is_empty() {
            self.search.filtered_indices = (0..self.data.index.len()).collect();
        } else {
            let query = normalize(&self.search.input);
            let mut scored: Vec<(usize, i32)> = self.data.index
                .iter()
                .enumerate()
                .map(|(i, item)| (i, score_normalized(&query, &item.norm_display)))
                .filter(|(_, score)| *score > crate::app::search::MIN_SCORE)
                .collect();

            scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            self.search.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        }

        if self.search.sort_by_profit {
            self.sort_filtered_by_profit();
        }

        let count = self.search.filtered_indices.len();
        if count == 0 {
            self.search.selected_index = 0;
        } else {
            self.search.selected_index = self.search.selected_index.min(count - 1);
        }
    }

    fn sort_filtered_by_profit(&mut self) {
        let profits: HashMap<usize, f64> = self
            .search
            .filtered_indices
            .iter()
            .map(|&idx| (idx, self.flip_profit(idx)))
            .collect();

        self.search.filtered_indices.sort_by(|a_idx, b_idx| {
            let a = profits.get(a_idx).copied().unwrap_or(0.0);
            let b = profits.get(b_idx).copied().unwrap_or(0.0);
            b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Post-tax profit per item; rejected products score zero and sink.
    fn flip_profit(&self, index: usize) -> f64 {
        self.data
            .index
            .get(index)
            .and_then(|item| self.data.flips.get(&item.id))
            .map_or(0.0, |f| f.profit_per_item)
    }

    pub fn flip(&self, product_id: &str) -> Option<&BazaarFlip> {
        self.data.flips.get(product_id)
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.search.filtered_indices.is_empty() {
            return;
        }
        let len = self.search.filtered_indices.len() as isize;
        let mut idx = self.search.selected_index as isize + delta;
        idx = idx.clamp(0, len - 1);
        self.search.selected_index = idx as usize;
    }

    pub fn jump_to_top(&mut self) {
        if !self.search.filtered_indices.is_empty() {
            self.search.selected_index = 0;
        }
    }

    pub fn jump_to_bottom(&mut self) {
        if !self.search.filtered_indices.is_empty() {
            self.search.selected_index = self.search.filtered_indices.len() - 1;
        }
    }

    pub fn enter_detail(&mut self) {
        if let Some(&idx) = self.search.filtered_indices.get(self.search.selected_index) {
            let id = self.data.index[idx].id.clone();
            self.detail.product_id = Some(id.clone());
            self.detail.history.clear();
            if let Some(p) = self.data.products.get(&id).and_then(prices) {
                self.push_history(p.instant_buy, p.instant_sell);
            }
            self.view = View::Detail;
            
            self.start_refresh(id);
        }
    }

    pub fn exit_detail(&mut self) {
        self.stop_refresh();
        self.view = View::Search;
        self.detail.product_id = None;
        self.detail.history.clear();
    }

    pub fn update_product(&mut self, p: BazaarProduct) {
        let id = p.product_id.clone();

        if self.detail.product_id.as_deref() == Some(&id) {
            if let Some(px) = prices(&p) {
                self.push_history(px.instant_buy, px.instant_sell);
            }
            self.status = "Updated".into();
        }

        self.data.products.insert(id, p);
    }

    fn push_history(&mut self, buy: f64, sell: f64) {
        let now = Instant::now();
        if self.detail.history.len() == self.detail.history.capacity() {
            self.detail.history.pop_front();
        }
        self.detail.history.push_back((now, buy, sell));
    }

    fn start_refresh(&mut self, product_id: String) {
        self.stop_refresh();

        let (tx, mut rx) = oneshot::channel::<()>();
        self.detail.cancel_tx = Some(tx);
        let outbound = self.update_tx.clone();
        let client = self.client.clone();
        let pid_task = product_id.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Ok(bazaar) = client.skyblock_bazaar().await
                            && let Some(p) = bazaar.products.get(&pid_task)
                            && let Some(out) = &outbound
                        {
                            let _ = out.send(p.clone());
                        }
                    }
                    _ = &mut rx => {
                        break;
                    }
                }
            }
        });
        self.detail.refresh_task = Some(handle);
        self.status = format!("Detail: {} (refreshing every 3s)", product_id);
    }

    pub fn stop_refresh(&mut self) {
        if let Some(tx) = self.detail.cancel_tx.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.detail.refresh_task.take() {
            h.abort();
        }
    }

    pub fn manual_refresh(&mut self) {
        if let Some(id) = &self.detail.product_id {
            let id = id.clone();
            let outbound = self.update_tx.clone();
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Ok(bazaar) = client.skyblock_bazaar().await
                    && let Some(p) = bazaar.products.get(&id)
                    && let Some(out) = &outbound
                {
                    let _ = out.send(p.clone());
                }
            });
            self.status = "Refreshing...".into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypixel::models::skyblock::{BazaarOrder, BazaarQuickStatus};

    fn order(price: f64) -> BazaarOrder {
        BazaarOrder {
            amount: 1,
            price_per_unit: price,
            orders: 1,
        }
    }

    /// One-level book: asks in `buy_summary`, bids in `sell_summary`.
    fn product(ask: f64, bid: f64) -> BazaarProduct {
        BazaarProduct {
            product_id: "ENCHANTED_DIAMOND".into(),
            buy_summary: vec![order(ask)],
            sell_summary: vec![order(bid)],
            quick_status: None,
        }
    }

    fn with_quick_status(mut p: BazaarProduct, buy_price: f64, sell_price: f64) -> BazaarProduct {
        p.quick_status = Some(BazaarQuickStatus {
            product_id: p.product_id.clone(),
            buy_price,
            sell_price,
            buy_volume: 0,
            sell_volume: 0,
            buy_moving_week: 0,
            sell_moving_week: 0,
            buy_orders: 0,
            sell_orders: 0,
            extra: Default::default(),
        });
        p
    }

    #[test]
    fn quick_status_maps_buy_to_instant_buy() {
        let p = with_quick_status(product(1355.20, 1257.90), 1379.05, 1257.31);
        let px = prices(&p).expect("quick status present");

        assert_eq!(px.instant_buy, 1379.05);
        assert_eq!(px.instant_sell, 1257.31);
    }

    /// Guards the sign bug: a spread read off the book must be positive.
    #[test]
    fn fallback_uses_top_of_book_with_correct_orientation() {
        let px = prices(&product(1355.20, 1257.90)).expect("both sides populated");

        assert_eq!(px.instant_buy, 1355.20, "instant buy is the lowest ask");
        assert_eq!(px.instant_sell, 1257.90, "instant sell is the highest bid");
        assert!(px.spread() > 0.0, "ask sits above bid, so spread is positive");
        assert!((px.spread() - 97.30).abs() < 1e-9);
    }

    #[test]
    fn spread_is_relative_to_what_you_receive() {
        let px = prices(&product(200.0, 100.0)).unwrap();

        assert_eq!(px.spread(), 100.0);
        assert_eq!(px.spread_pct(), 100.0);
    }

    #[test]
    fn quick_status_takes_precedence_over_the_book() {
        let p = with_quick_status(product(999.0, 1.0), 1379.05, 1257.31);
        let px = prices(&p).unwrap();

        assert_eq!(px.instant_buy, 1379.05);
    }

    #[test]
    fn a_one_sided_book_has_no_prices() {
        let mut p = product(1355.20, 1257.90);
        p.sell_summary.clear();

        assert!(prices(&p).is_none());
    }

    #[test]
    fn zero_bid_does_not_blow_up_the_percentage() {
        let px = prices(&product(10.0, 0.0)).unwrap();

        assert_eq!(px.spread(), 10.0);
        assert_eq!(px.spread_pct(), 0.0);
    }

    /// A depth-weighted inversion is real signal; it must not be clamped.
    #[test]
    fn an_inverted_quick_status_keeps_its_sign() {
        let p = with_quick_status(product(100.0, 50.0), 10.0, 20.0);
        let px = prices(&p).unwrap();

        assert!(px.spread() < 0.0);
    }
}
