use serde::{Serialize, Deserialize};
use std::fmt;
use std::fmt::Formatter;
use std::error::Error;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum LoanType {
    #[serde(rename = "STARTUP")]
    Startup,
    #[serde(rename = "ENTERPRISE")]
    Enterprise,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Good {
    #[serde(rename = "METALS")]
    Metals,
    #[serde(rename = "RARE_METALS")]
    RareMetals,
    #[serde(rename = "CHEMICALS")]
    Chemicals,
    #[serde(rename = "FUEL")]
    Fuel,
    #[serde(rename = "FOOD")]
    Food,
    #[serde(rename = "WORKERS")]
    Workers,
    #[serde(rename = "TEXTILES")]
    Textiles,
    #[serde(rename = "CONSUMER_GOODS")]
    ConsumerGoods,
    #[serde(rename = "MACHINERY")]
    Machinery,
    #[serde(rename = "CONSTRUCTION_MATERIALS")]
    ConstructionMaterials,
    #[serde(rename = "ELECTRONICS")]
    Electronics,
    #[serde(rename = "RESEARCH")]
    Research,
    #[serde(rename = "SHIP_PARTS")]
    ShipParts,
    #[serde(rename = "SHIP_PLATING")]
    ShipPlating,
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum LocationType {
    #[serde(rename = "PLANET")]
    Planet,
    #[serde(rename = "MOON")]
    Moon,
    #[serde(rename = "GAS_GIANT")]
    GasGiant,
    #[serde(rename = "ASTEROID")]
    Asteroid,
}

impl fmt::Display for LocationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Ship {
    pub id: String,
    pub location: Option<String>,
    pub cargo: Vec<Cargo>,
    #[serde(rename = "spaceAvailable")]
    pub space_available: i32,
    #[serde(rename = "type")]
    pub ship_type: String,
    pub class: String,
    #[serde(rename = "maxCargo")]
    pub max_cargo: i32,
    pub speed: i32,
    pub manufacturer: String,
    pub plating: i32,
    pub weapons: i32,
}


#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Cargo {
    pub good: Good,
    pub quantity: i32,
    #[serde(rename = "totalVolume")]
    pub total_volume: i32,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Order {
    pub good: Good,
    pub quantity: i32,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i32,
    pub total: i32,
}


#[derive(Deserialize, Debug, Clone)]
pub struct Loan {
    pub id: String,
    pub due: String,
    #[serde(rename = "repaymentAmount")]
    pub repayment_amount: i32,
    pub status: String,
    #[serde(rename = "type")]
    pub loan_type: LoanType
}

#[derive(Deserialize, Debug, Clone)]
pub struct PurchaseLocation {
    pub location: String,
    pub price: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ShipForSale {
    #[serde(rename = "type")]
    pub ship_type: String,
    pub class: String,
    #[serde(rename = "maxCargo")]
    pub max_cargo: i32,
    pub speed: i32,
    pub manufacturer: String,
    pub plating: i32,
    pub weapons: i32,
    #[serde(rename = "purchaseLocations")]
    pub purchase_locations: Vec<PurchaseLocation>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Location {
    pub symbol: String,
    #[serde(rename = "type")]
    pub location_type: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ErrorMessageData {
    pub code: i32,
    pub message: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FlightPlanData {
    pub id: String,
    #[serde(rename = "ship")]
    pub ship_id: String,
    #[serde(rename = "fuelConsumed")]
    pub fuel_consumed: i32,
    #[serde(rename = "fuelRemaining")]
    pub fuel_remaining: i32,
    #[serde(rename = "timeRemainingInSeconds")]
    pub time_remaining_in_seconds: i32,
    #[serde(rename = "arrivesAt")]
    pub arrives_at: DateTime<Utc>,
    #[serde(rename = "terminatedAt")]
    pub terminated_at: Option<DateTime<Utc>>,
    pub destination: String,
    pub departure: String,
    pub distance: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemsInfoLocation {
    pub symbol: String,
    #[serde(rename = "type")]
    pub systems_info_type: LocationType,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemsInfoData {
    pub symbol: String,
    pub name: String,
    pub locations: Vec<SystemsInfoLocation>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct MarketplaceData {
    #[serde(rename = "quantityAvailable")]
    pub quantity_available: i32,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i32,
    #[serde(rename = "volumePerUnit")]
    pub volume_per_unit: i32,
    pub symbol: Good,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlanetMarketplaceData {
    pub name: String,
    pub symbol: String,
    #[serde(rename = "type")]
    pub planet_type: String,
    pub x: i32,
    pub y: i32,
    pub marketplace: Vec<MarketplaceData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ErrorMessage {
    pub error: ErrorMessageData,
}

impl fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error Code: {} Error Message: {}", self.error.code, self.error.message)
    }
}

impl Error for ErrorMessage {}
