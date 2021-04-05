//! The game module wraps the interactions between the client and the server
use crate::{responses, requests};
use crate::shared;

use reqwest::{Client as ReqwestClient};
use serde::Deserialize;
use governor::{RateLimiter, Quota};
use std::num::NonZeroU32;
use governor::{state::{NotKeyed, InMemoryState}, clock::DefaultClock};
use std::sync::Arc;

/// An in memory rate limiter
pub type ClientRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Get a rate limiter pre-configured for the limits of the spacetraders API
pub fn get_rate_limiter() -> ClientRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(1u32).unwrap());
    Arc::new(RateLimiter::direct(quota))
}

/// Parse a response string into the type represented by T
/// If the `response_text` cannot be parsed into type T then it is assumed that an error
/// occurred and an shared::ErrorMessage will be returned
///
/// # Arguments
///
/// * `response_text` - A string containing the JSON response to be parsed
fn parse_response<'a, T: Deserialize<'a>>(response_text: &'a String) -> Result<T, anyhow::Error> {
    match serde_json::from_str::<T>(&response_text) {
        Ok(o) => Ok(o),
        Err(e) => {
            println!("Error processing type {:?}: {}", std::any::type_name::<T>(), e);
            println!("Error response: {}", &response_text);

            match serde_json::from_str::<shared::ErrorMessage>(&response_text) {
                Ok(error_message) => Err(anyhow::Error::from(error_message)),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        }
    }
}

/// Claim a username and get a token
///
/// # Arguments
///
/// * `username` - A string containing the username to get a token for
pub async fn claim_username(username: String) -> Result<responses::ClaimUsername, anyhow::Error> {
    let client = reqwest::Client::new();
    let response_text = client.post(&format!("https://api.spacetraders.io/users/{}/token", username))
        .body("this response body doesn't matter")
        .send().await?
        .text().await?;

    println!("ResponseText: {}", response_text.to_owned());

    parse_response::<responses::ClaimUsername>(&response_text)
}

/// A game that is associated to a specific username
#[derive(Debug)]
pub struct Client {
    rate_limiter: ClientRateLimiter,
    http_client: ReqwestClient,
    /// The users spacetraders API id
    pub user_id: String,
    /// The users username
    pub username: String,
    // pub token: String,
}

impl Client {
    /// Create a new game with a reqwest client that has the Authorization header set
    ///
    /// # Arguments
    ///
    /// * `username` - A string containing the username of the current player
    /// * `token` - A string containing the access token for the username provided
    pub fn new(rate_limiter: ClientRateLimiter, user_id: String, username: String, token: String) -> Client {
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "Authorization",
            format!("Bearer {}", &token).to_string().parse().unwrap(),
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(default_headers)
            .build()
            .expect("unable to create game client");

        Client {
            rate_limiter,
            http_client: client,
            user_id,
            username,
            // token,
        }
    }

    // async fn execute_request<'a, T: Deserialize<'a>>(&self, request: Request) -> Result<T, anyhow::Error> {
    //     self.rate_limiter.until_ready().await;
    //     let response = self.http_client.execute(request).await?;
    //     let response_text = response.text().await?;
    //
    //     parse_response::<T>(&response_text)
    // }

    /// Get the current details of a flight plan
    ///
    /// # Arguments
    ///
    /// * `flight_plan_id` - A string containing the flight plan id
    pub async fn get_flight_plan(&self, flight_plan_id: String) -> Result<responses::FlightPlan, anyhow::Error> {
        // let request = self.http_client.get(&format!("https://api.spacetraders.io/users/{}/flight-plans/{}", self.username, flight_plan_id))
        //     .build().unwrap();

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/users/{}/flight-plans/{}", self.username, flight_plan_id))
            .send().await?
            .text().await?;

        parse_response::<responses::FlightPlan>(&response_text)
    }

    /// Create a flight plan.
    ///
    /// # Arguments
    ///
    /// * `ship_id` - A string containing the ship_id to create the flight plan for
    /// * `destination` - A string containing the location to send the ship to
    pub async fn create_flight_plan(&self, ship_id: String, destination: String) -> Result<responses::FlightPlan, anyhow::Error> {
        let flight_plan_request = requests::FlightPlanRequest {
            ship_id,
            destination,
        };

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.post(&format!("https://api.spacetraders.io/users/{}/flight-plans", self.username).to_string())
            .json(&flight_plan_request)
            .send().await?
            .text().await?;

        parse_response::<responses::FlightPlan>(&response_text)
    }

    /// Get the status of the game API.
    pub async fn get_game_status(&self) -> Result<responses::GameStatus, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get("https://api.spacetraders.io/game/status")
            .send().await?
            .text().await?;

        parse_response::<responses::GameStatus>(&response_text)
    }

    /// Get all available loans
    pub async fn get_available_loans(&self) -> Result<responses::AvailableLoans, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get("https://api.spacetraders.io/game/loans")
            .send().await?
            .text().await?;

        parse_response::<responses::AvailableLoans>(&response_text)
    }

    /// Get any loans taken out by the current user
    pub async fn get_your_loans(&self) -> Result<responses::LoanInfo, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/users/{}/loans", self.username))
            .send().await?
            .text().await?;

        parse_response::<responses::LoanInfo>(&response_text)
    }

    /// Pay off a loan completely
    ///
    /// # Arguments
    ///
    /// * `loan_id` - A string containing the loan_id of the loan to pay off
    pub async fn pay_off_loan(&self, loan_id: String) -> Result<responses::UserInfo, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.put(&format!("https://api.spacetraders.io/users/{}/loans/{}", self.username, loan_id).to_string())
            .send().await?
            .text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }

    /// Request a new loan
    ///
    /// # Arguments
    ///
    /// * `loan_type` - A LoanType with the type of loan being requested for the current user
    pub async fn request_new_loan(&self, loan_type: shared::LoanType) -> Result<responses::UserInfo, anyhow::Error> {
        let request_new_loan_request = requests::RequestNewLoanRequest {
            loan_type
        };

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.post(&format!("https://api.spacetraders.io/users/{}/loans", self.username).to_string())
            .json(&request_new_loan_request)
            .send().await?
            .text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }

    /// Get location info about a specific location
    ///
    /// # Arguments
    ///
    /// * `location` - A string containing the location name to get info about
    pub async fn get_location_info(&self, location: String) -> Result<responses::LocationInfo, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/game/locations/{}", location).to_string())
            .send().await?
            .text().await?;

        parse_response::<responses::LocationInfo>(&response_text)
    }

    /// Get all the locations in a particular system
    ///
    /// # Arguments
    ///
    /// * `system` - A string containing the system name to get the locations from
    /// * `location_type` - An optional LocationType if you want to filter the locations by type
    pub async fn get_locations_in_system(&self, system: String, location_type: Option<shared::LocationType>) -> Result<responses::AvailableLocations, anyhow::Error> {
        let mut query = Vec::new();
        if let Some(location_type) = location_type {
            query.push(("type", location_type));
        }

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/game/systems/{}/locations", system).to_string())
            .query(&query)
            .send().await?
            .text().await?;

        parse_response::<responses::AvailableLocations>(&response_text)
    }

    /// Get the marketplace data about a location.
    ///
    /// # Note
    ///
    /// You must have a ship docked at the location in order to get it's marketplace data
    ///
    /// # Arguments
    ///
    /// * `location` - A string containing the name of the location to get marketplace data for
    pub async fn get_location_marketplace(&self, location: String) -> Result<responses::LocationMarketplace, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/game/locations/{}/marketplace", location))
            .send().await?
            .text().await?;

        parse_response::<responses::LocationMarketplace>(&response_text)
    }

    /// Create a purchase order to transfer goods from a location to your ship
    ///
    /// # Arguments
    ///
    /// * `ship` - A Ship struct that you'd like to transfer the goods into
    /// * `good` - A Good enum containing the type of good you'd like to transfer
    /// * `quantity` - An i32 containing the quantity of good you'd like transferred
    pub async fn create_purchase_order(&self, ship: shared::Ship, good: shared::Good, quantity: i32) -> Result<responses::PurchaseOrder, anyhow::Error> {
        let purchase_order_request = requests::PurchaseOrderRequest {
            ship_id: ship.id.to_owned(),
            good,
            quantity,
        };

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.post(&format!("https://api.spacetraders.io/users/{}/purchase-orders", self.username).to_string())
            .json(&purchase_order_request)
            .send().await?
            .text().await?;

        parse_response::<responses::PurchaseOrder>(&response_text)
    }

    /// Create a sell order to transfer good from your ship to a location. Your ship will
    /// automatically sell the good to whatever location it is docked at
    ///
    /// # Arguments
    ///
    /// * `ship` - A Ship struct that you'd like to transfer the goods from
    /// * `good` - A Good enum containing the type of good you'd like to transfer
    /// * `quantity` - An i32 containing the quantity of good you'd like transferred
    pub async fn create_sell_order(&self, ship_id: String, good: shared::Good, quantity: i32) -> Result<responses::PurchaseOrder, anyhow::Error> {
        let sell_order_request = requests::SellOrderRequest {
            ship_id,
            good,
            quantity,
        };

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.post(&format!("https://api.spacetraders.io/users/{}/sell-orders", self.username))
            .json(&sell_order_request)
            .send().await?
            .text().await?;

        parse_response::<responses::PurchaseOrder>(&response_text)
    }

    /// Add a ship to the users inventory by purchasing it
    ///
    /// # Arguments
    ///
    /// * `location` - A string containing the location you'd like to purchase the ship from
    /// * `ship_type` - A string containing the type of ship you'd like to purchase
    pub async fn purchase_ship(&self, location: String, ship_type: String) -> Result<responses::UserInfo, anyhow::Error> {
        let purchase_ship_request = requests::PurchaseShipRequest {
            location,
            ship_type,
        };

        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.post(&format!("https://api.spacetraders.io/users/{}/ships", self.username).to_string())
            .json(&purchase_ship_request)
            .send().await?
            .text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }

    /// Get all ships that are available for sale
    pub async fn get_ships_for_sale(&self) -> Result<responses::ShipsForSale, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get("https://api.spacetraders.io/game/ships")
            .send().await?
            .text().await?;

        parse_response::<responses::ShipsForSale>(&response_text)
    }

    /// Get all your ships
    pub async fn get_your_ships(&self) -> Result<responses::YourShips, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/users/{}/ships", self.username).to_string())
            .send().await?
            .text().await?;

        parse_response::<responses::YourShips>(&response_text)
    }

    /// Get information about all systems
    pub async fn get_systems_info(&self) -> Result<responses::SystemsInfo, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get("https://api.spacetraders.io/game/systems")
            .send().await?
            .text().await?;

        parse_response::<responses::SystemsInfo>(&response_text)
    }

    /// You begin the game by claiming a username and receiving a token for that username.
    /// This will automatically use the username that was assigned when the Game struct was created
    pub async fn claim_username_get_token(&self) -> Result<responses::ClaimUsername, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text = self.http_client.get(&format!("https://api.spacetraders.io/users/{}/token", self.username).to_string())
            .send().await?
            .text().await?;

        parse_response::<responses::ClaimUsername>(&response_text)
    }

    /// Get all information about the current user
    pub async fn get_user_info(&self) -> Result<responses::UserInfo, anyhow::Error> {
        self.rate_limiter.until_ready().await;
        let response_text  = self.http_client.get(&format!("https://api.spacetraders.io/users/{}", self.username).to_string())
            .send().await?
            .text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }
}
