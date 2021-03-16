use crate::{responses, requests};
use crate::shared;

use reqwest::Client;
use std::fmt;
use serde::Deserialize;

pub struct Game {
    client: Client,
    username: String,
    // token: String,
}

#[derive(Debug)]
struct MyError(String);

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}

impl std::error::Error for MyError {}

impl Game {
    pub fn new(username: String, token: String) -> Game {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "Authorization",
            format!("Bearer {}", &token).to_string().parse().unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(default_headers)
            .build()
            .expect("unable to create game client");

        Game {
            client,
            username,
            // token,
        }
    }

    fn parse_response<'a, T: Deserialize<'a>>(&self, response_text: &'a String) -> Result<T, Box<dyn std::error::Error>> {
        match serde_json::from_str::<T>(&response_text) {
            Ok(o) => Ok(o),
            Err(e) => {
                println!("Error processing: {}", e);
                println!("Error response: {}", &response_text);

                match serde_json::from_str::<shared::ErrorMessage>(&response_text) {
                    Ok(error_message) => Err(Box::new(error_message)),
                    Err(e) => Err(Box::new(e)),
                }
            }
        }
    }

    pub async fn get_flight_plan(&self, flight_plan_id: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/users/{}/flight-plans/{}", self.username, flight_plan_id))
            .send().await?
            .text().await?;

        self.parse_response::<responses::FlightPlan>(&response_text)
    }

    pub async fn create_flight_plan(&self, ship_id: String, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
        let flight_plan_request = requests::FlightPlanRequest {
            ship_id,
            destination,
        };

        let response_text = self.client.post(&format!("https://api.spacetraders.io/users/{}/flight-plans", self.username).to_string())
            .json(&flight_plan_request)
            .send().await?
            .text().await?;

        self.parse_response::<responses::FlightPlan>(&response_text)
    }

    pub async fn get_game_status(&self) -> Result<responses::GameStatus, Box<dyn std::error::Error>> {
        let response_text = self.client.get("https://api.spacetraders.io/game/status")
            .send().await?
            .text().await?;

        self.parse_response::<responses::GameStatus>(&response_text)
    }

    pub async fn get_available_loans(&self) -> Result<responses::AvailableLoans, Box<dyn std::error::Error>> {
        let response_text = self.client.get("https://api.spacetraders.io/game/loans")
            .send().await?
            .text().await?;

        self.parse_response::<responses::AvailableLoans>(&response_text)
    }

    pub async fn get_your_loans(&self) -> Result<responses::LoanInfo, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/users/{}/loans", self.username))
            .send().await?
            .text().await?;

        self.parse_response::<responses::LoanInfo>(&response_text)
    }

    pub async fn request_new_loan(&self, loan_type: shared::LoanType) -> Result<responses::UserInfo, Box<dyn std::error::Error>> {
        let request_new_loan_request = requests::RequestNewLoanRequest {
            loan_type
        };

        let response_text = self.client.post(&format!("https://api.spacetraders.io/users/{}/loans", self.username).to_string())
            .json(&request_new_loan_request)
            .send().await?
            .text().await?;

        self.parse_response::<responses::UserInfo>(&response_text)
    }

    pub async fn get_location_info(&self, location_symbol: String) -> Result<responses::LocationInfo, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/game/locations/{}", location_symbol).to_string())
            .send().await?
            .text().await?;

        self.parse_response::<responses::LocationInfo>(&response_text)
    }

    pub async fn get_locations_in_system(&self, system: String, location_type: shared::LocationType) -> Result<responses::AvailableLocations, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/game/systems/{}/locations", system).to_string())
            .query(&[("type", location_type)])
            .send().await?
            .text().await?;

        self.parse_response::<responses::AvailableLocations>(&response_text)
    }

    pub async fn get_location_marketplace(&self, location: String) -> Result<responses::PlanetMarketplace, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/game/locations/{}/marketplace", location))
            .send().await?
            .text().await?;

        self.parse_response::<responses::PlanetMarketplace>(&response_text)
    }

    pub async fn create_purchase_order(&self, ship: shared::Ship, good: shared::Good, quantity: i32) -> Result<responses::PurchaseOrder, Box<dyn std::error::Error>> {
        let purchase_order_request = requests::PurchaseOrderRequest {
            ship_id: ship.id.to_owned(),
            good,
            quantity,
        };

        let response_text = self.client.post(&format!("https://api.spacetraders.io/users/{}/purchase-orders", self.username).to_string())
            .json(&purchase_order_request)
            .send().await?
            .text().await?;

        self.parse_response::<responses::PurchaseOrder>(&response_text)
    }

    pub async fn create_sell_order(&self, ship_id: String, good: shared::Good, quantity: i32) -> Result<responses::PurchaseOrder, Box<dyn std::error::Error>> {
        let sell_order_request = requests::SellOrderRequest {
            ship_id,
            good,
            quantity,
        };

        let response_text = self.client.post(&format!("https://api.spacetraders.io/users/{}/sell-orders", self.username))
            .json(&sell_order_request)
            .send().await?
            .text().await?;

        self.parse_response::<responses::PurchaseOrder>(&response_text)
    }

    pub async fn purchase_ship(&self, location: String, ship_type: String) -> Result<responses::UserInfo, Box<dyn std::error::Error>> {
        let purchase_ship_request = requests::PurchaseShipRequest {
            location,
            ship_type,
        };

        let response_text = self.client.post(&format!("https://api.spacetraders.io/users/{}/ships", self.username).to_string())
            .json(&purchase_ship_request)
            .send().await?
            .text().await?;

        self.parse_response::<responses::UserInfo>(&response_text)
    }

    pub async fn get_ships_for_sale(&self) -> Result<responses::ShipsForSale, Box<dyn std::error::Error>> {
        let response_text = self.client.get("https://api.spacetraders.io/game/ships")
            .send().await?
            .text().await?;

        self.parse_response::<responses::ShipsForSale>(&response_text)
    }

    pub async fn get_your_ships(&self) -> Result<responses::YourShips, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/users/{}/ships", self.username).to_string())
            .send().await?
            .text().await?;

        self.parse_response::<responses::YourShips>(&response_text)
    }

    pub async fn get_systems_info(&self) -> Result<responses::SystemsInfo, Box<dyn std::error::Error>> {
        let response_text = self.client.get("https://api.spacetraders.io/game/systems")
            .send().await?
            .text().await?;

        self.parse_response::<responses::SystemsInfo>(&response_text)
    }

    pub async fn claim_username_get_token(&self) -> Result<responses::ClaimUsernameResponse, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/users/{}/token", self.username).to_string())
            .send().await?
            .text().await?;

        self.parse_response::<responses::ClaimUsernameResponse>(&response_text)
    }

    pub async fn get_user_info(&self) -> Result<responses::UserInfo, Box<dyn std::error::Error>> {
        let response_text = self.client.get(&format!("https://api.spacetraders.io/users/{}", self.username).to_string())
            .send().await?
            .text().await?;

        self.parse_response::<responses::UserInfo>(&response_text)
    }
}
