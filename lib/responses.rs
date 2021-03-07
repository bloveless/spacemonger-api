use crate::shared;
use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Deserialize, Debug)]
pub struct GameStatus {
    pub status: String,
}

#[derive(Deserialize, Debug)]
pub struct UserInfoData {
    pub username: String,
    pub credits: i32,
    pub ships: Vec<shared::Ship>,
    pub loans: Vec<shared::Loan>,
}

#[derive(Deserialize, Debug)]
pub struct UserInfo {
    pub user: UserInfoData,
}

#[derive(Deserialize, Debug)]
pub struct AvailableLoan {
    #[serde(rename = "type")]
    pub loan_type: shared::LoanType,
    pub amount: u32,
    pub rate: f64,
    #[serde(rename = "termInDays")]
    pub term_in_days: u32,
    #[serde(rename = "collateralRequired")]
    pub collateral_required: bool,
}

#[derive(Deserialize, Debug)]
pub struct AvailableLoans {
    pub loans: Vec<AvailableLoan>,
}

#[derive(Deserialize, Debug)]
pub struct ShipsForSale {
    pub ships: Vec<shared::ShipForSale>,
}

#[derive(Deserialize, Debug)]
pub struct PurchaseOrder {
    pub credits: u32,
    pub order: Vec<shared::Order>,
    pub ship: shared::Ship,
}

#[derive(Deserialize, Debug)]
pub struct AvailableLocations {
    pub locations: Vec<shared::Location>,
}

#[derive(Deserialize, Debug)]
pub struct FlightPlan {
    #[serde(rename = "flightPlan")]
    pub flight_plan: shared::FlightPlanData,
}

#[derive(Deserialize, Debug)]
pub struct SystemsInfo {
    pub systems: Vec<shared::SystemsInfoData>,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub picture: Option<String>,
    pub email: Option<String>,
    pub credits: u32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct ClaimUsernameResponse {
    pub token: String,
    pub user: User,
}

#[derive(Deserialize, Debug)]
pub struct YourShips {
    pub ships: Vec<shared::Ship>,
}

#[derive(Deserialize, Debug)]
pub struct LocationInfo {
    pub planet: shared::SystemsInfoLocation,
}
