use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct BazaarResponse {
    pub success: bool,
    pub cause: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: i64,
    #[serde(default)]
    pub products: HashMap<String, Product>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Product {
    #[serde(rename = "product_id")]
    pub product_id: String,
    pub sell_summary: Vec<OrderSummary>,
    pub buy_summary: Vec<OrderSummary>,
    pub quick_status: QuickStatus,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrderSummary {
    pub amount: i64,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: f64,
    pub orders: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QuickStatus {
    #[serde(rename = "productId")]
    pub product_id: String,
    #[serde(rename = "sellPrice")]
    pub sell_price: f64,
    #[serde(rename = "sellVolume")]
    pub sell_volume: i64,
    #[serde(rename = "sellMovingWeek")]
    pub sell_moving_week: i64,
    #[serde(rename = "sellOrders")]
    pub sell_orders: i64,
    #[serde(rename = "buyPrice")]
    pub buy_price: f64,
    #[serde(rename = "buyVolume")]
    pub buy_volume: i64,
    #[serde(rename = "buyMovingWeek")]
    pub buy_moving_week: i64,
    #[serde(rename = "buyOrders")]
    pub buy_orders: i64,
}
