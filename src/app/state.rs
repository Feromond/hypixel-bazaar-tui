use crate::api::models::{BazaarResponse, Product};
use crate::app::search::score_normalized;
use crate::util::{normalize, pretty_name};
use indexmap::IndexMap;
use std::collections::VecDeque;
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

#[derive(Debug)]
pub struct BazaarData {
    pub products: IndexMap<String, Product>,
    pub last_updated: i64,
    pub index: Vec<ProductIndexItem>,
}

#[derive(Debug)]
pub struct SearchState {
    pub input: String,
    pub mode: SearchMode,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub needs_filter: bool,
    pub last_input_change: Instant,
    pub sort_by_spread: bool,
}

#[derive(Debug)]
pub struct DetailState {
    pub product_id: Option<String>,
    pub history: VecDeque<(Instant, f64, f64)>, // (time, buy, sell)
    pub show_percent: bool,
    pub show_sma: bool,
    pub show_midline: bool,
    
    // Background refresh
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
    pub update_tx: Option<mpsc::UnboundedSender<Product>>,
}

impl App {
    pub fn new(response: BazaarResponse) -> Self {
        let mut products = IndexMap::new();
        for (k, v) in response.products {
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
                last_updated: response.last_updated,
                index,
            },
            search: SearchState {
                input: String::new(),
                mode: SearchMode::Insert,
                filtered_indices,
                selected_index: 0,
                needs_filter: true,
                last_input_change: Instant::now(),
                sort_by_spread: false,
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
        }
    }

    pub fn set_update_sender(&mut self, tx: mpsc::UnboundedSender<Product>) {
        self.update_tx = Some(tx);
    }

    pub fn current_product(&self) -> Option<&Product> {
        self.detail.product_id.as_ref().and_then(|id| self.data.products.get(id))
    }

    // --- Search Logic ---

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

            // Sort by score desc, then index asc
            scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            self.search.filtered_indices = scored.into_iter().map(|(i, _)| i).collect();
        }

        if self.search.sort_by_spread {
            self.sort_filtered_by_spread();
        }

        // Clamp selection
        let count = self.search.filtered_indices.len();
        if count == 0 {
            self.search.selected_index = 0;
        } else {
            self.search.selected_index = self.search.selected_index.min(count - 1);
        }
    }

    fn sort_filtered_by_spread(&mut self) {
        let spreads: std::collections::HashMap<usize, f64> = self
            .search
            .filtered_indices
            .iter()
            .map(|&idx| (idx, self.get_spread(idx)))
            .collect();

        self.search.filtered_indices.sort_by(|&a_idx, &b_idx| {
            let spread_a = spreads.get(&a_idx).unwrap_or(&0.0);
            let spread_b = spreads.get(&b_idx).unwrap_or(&0.0);
            spread_b
                .partial_cmp(spread_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    fn get_spread(&self, index: usize) -> f64 {
        if let Some(item) = self.data.index.get(index)
            && let Some(p) = self.data.products.get(&item.id) {
                return p.quick_status.sell_price - p.quick_status.buy_price;
            }
        0.0
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

    // --- Detail Logic ---

    pub fn enter_detail(&mut self) {
        if let Some(&idx) = self.search.filtered_indices.get(self.search.selected_index) {
            let id = self.data.index[idx].id.clone();
            self.detail.product_id = Some(id.clone());
            self.detail.history.clear();
            if let Some(p) = self.data.products.get(&id) {
                self.push_history(p.quick_status.buy_price, p.quick_status.sell_price);
            }
            self.view = View::Detail;
            
            // Start refreshing
            self.start_refresh(id);
        }
    }

    pub fn exit_detail(&mut self) {
        self.stop_refresh();
        self.view = View::Search;
        self.detail.product_id = None;
        self.detail.history.clear();
    }

    pub fn update_product(&mut self, p: Product) {
        let id = p.product_id.clone();
        // Only update if this is the currently selected product or we just want to update cache
        self.data.products.insert(id.clone(), p.clone());
        
        if self.detail.product_id.as_deref() == Some(&id) {
             self.push_history(p.quick_status.buy_price, p.quick_status.sell_price);
             self.status = "Updated".into();
        }
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
        let pid_task = product_id.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        if let Ok(response) = crate::api::client::fetch_bazaar().await
                             && let Some(p) = response.products.get(&pid_task)
                                 && let Some(out) = &outbound {
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
            tokio::spawn(async move {
                 if let Ok(response) = crate::api::client::fetch_bazaar().await
                     && let Some(p) = response.products.get(&id)
                         && let Some(out) = &outbound {
                             let _ = out.send(p.clone());
                         }
            });
            self.status = "Refreshing...".into();
        }
    }
}
