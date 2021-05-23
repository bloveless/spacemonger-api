use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct MarketData {
    pub id: i32,
    pub location_symbol: String,
    pub system_symbol: String,
    pub good_symbol: String,
    pub price_per_unit: i32,
    pub volume_per_unit: i32,
    pub quantity_available: i32,
    pub created_at: DateTime<Utc>,
    pub purchase_price_per_unit: i32,
    pub sell_price_per_unit: i32,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub assignment: String,
    pub system_symbol: Option<String>,
    pub location_symbol: Option<String>,
    pub credits: i32,
    pub ship_count: i32,
    pub ships: Option<String>,
    pub stats_updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct UserStats {
    pub user_id: String,
    pub credits: i32,
    pub ship_count: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct UserStatsResponse {
    pub username: String,
    pub stats: Vec<UserStats>,
}

#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub system_symbol: String,
    pub system_name: String,
    pub location_symbol: String,
    pub location_name: String,
    pub location_type: String,
    pub x: i32,
    pub y: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub purchase_location_symbol: String,
    pub sell_location_symbol: String,
    pub good_symbol: String,
    pub purchase_x: i32,
    pub purchase_y: i32,
    pub sell_x: i32,
    pub sell_y: i32,
    pub distance: f64,
    pub purchase_location_type: String,
    pub approximate_fuel: i32,
    pub purchase_quantity_available: i32,
    pub sell_quantity_available: i32,
    pub purchase_price_per_unit: i32,
    pub sell_price_per_unit: i32,
    pub purchase_created_at: DateTime<Utc>,
    pub sell_created_at: DateTime<Utc>,
}
