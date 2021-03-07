use serde::{Serialize, Deserialize};
use std::fmt;
use std::fmt::Formatter;
use std::error::Error;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug)]
pub enum LoanType {
    #[serde(rename = "STARTUP")]
    Startup,
    #[serde(rename = "ENTERPRISE")]
    Enterprise,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Good {
    #[serde(rename= "METALS")]
    Metals,
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
    #[serde(rename = "ELECTROINICS")]
    Electronics,
    #[serde(rename = "RESEARCH")]
    Research,
    #[serde(rename = "SHIP_PARTS")]
    ShipParts,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct Ship {
    pub id: String,
    pub location: String,
    pub cargo: Vec<Cargo>,
    #[serde(rename = "spaceAvailable")]
    pub space_available: u32,
    #[serde(rename = "type")]
    pub ship_type: String,
    pub class: String,
    #[serde(rename = "maxCargo")]
    pub max_cargo: u32,
    pub speed: u32,
    pub manufacturer: String,
    pub plating: u32,
    pub weapons: u32,
}


#[derive(Deserialize, Debug)]
pub struct Cargo {
    pub good: Good,
    pub quantity: u32,
}

#[derive(Deserialize, Debug)]
pub struct Order {
    pub good: Good,
    pub quantity: u32,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: u32,
    pub total: u32,
}


#[derive(Deserialize, Debug)]
pub struct Loan {
    pub id: String,
    pub due: String,
    #[serde(rename = "repaymentAmount")]
    pub repayment_amount: u32,
    pub status: String,
    #[serde(rename = "type")]
    pub loan_type: LoanType
}

#[derive(Deserialize, Debug)]
pub struct PurchaseLocation {
    pub location: String,
    pub price: u32,
}

#[derive(Deserialize, Debug)]
pub struct ShipForSale {
    #[serde(rename = "type")]
    pub ship_type: String,
    pub class: String,
    #[serde(rename = "maxCargo")]
    pub max_cargo: u32,
    pub speed: u32,
    pub manufacturer: String,
    pub plating: u32,
    pub weapons: u32,
    #[serde(rename = "purchaseLocations")]
    pub purchase_locations: Vec<PurchaseLocation>,
}

#[derive(Deserialize, Debug)]
pub struct Location {
    pub symbol: String,
    #[serde(rename = "type")]
    pub location_type: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug)]
pub struct ErrorMessageData {
    pub code: u32,
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct FlightPlanData {
    pub id: String,
    #[serde(rename = "ship")]
    pub ship_id: String,
    #[serde(rename = "fuelConsumed")]
    pub fuel_consumed: u32,
    #[serde(rename = "fuelRemaining")]
    pub fuel_remaining: u32,
    #[serde(rename = "timeRemainingInSeconds")]
    pub time_remaining_in_seconds: u32,
    #[serde(rename = "arrivesAt")]
    pub arrives_at: DateTime<Utc>,
    #[serde(rename = "terminatedAt")]
    pub terminated_at: Option<DateTime<Utc>>,
    pub destination: String,
    pub departure: String,
    pub distance: u32,
}

#[derive(Deserialize, Debug)]
pub struct SystemsInfoLocation {
    pub symbol: String,
    #[serde(rename = "type")]
    pub systems_info_type: LocationType,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Deserialize, Debug)]
pub struct SystemsInfoData {
    pub symbol: String,
    pub name: String,
    pub locations: Vec<SystemsInfoLocation>,
}

#[derive(Deserialize, Debug)]
pub struct ErrorMessage {
    pub error: ErrorMessageData,
}

impl fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error Code: {} Error Message: {}", self.error.code, self.error.message)
    }
}

impl Error for ErrorMessage {}
