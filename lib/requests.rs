use serde::Serialize;
use crate::shared;

#[derive(Serialize, Debug)]
pub struct PurchaseOrderRequest {
    #[serde(rename(serialize = "shipId"))]
    pub ship_id: String,
    pub good: shared::Good,
    pub quantity: u32,
}

#[derive(Serialize, Debug)]
pub struct PurchaseShipRequest {
    pub location: String,
    #[serde(rename(serialize = "type"))]
    pub ship_type: String,
}

#[derive(Serialize, Debug)]
pub struct RequestNewLoanRequest {
    #[serde(rename(serialize = "type"))]
    pub loan_type: shared::LoanType,
}

#[derive(Serialize, Debug)]
pub struct FlightPlanRequest {
    #[serde(rename(serialize = "shipId"))]
    pub ship_id: String,
    pub destination: String,
}
