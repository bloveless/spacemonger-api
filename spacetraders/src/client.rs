//! The game module wraps the interactions between the client and the server
use crate::{responses, requests};
use crate::shared;

use reqwest;
use serde::Deserialize;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::error::Error;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Method, Url};
use std::str::FromStr;
use tokio::time::Duration;

pub type ArcHttpClient = Arc<Mutex<HttpClient>>;

#[derive(Debug)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    fn new() -> HttpClient {
        HttpClient {
            client: reqwest::Client::new(),
        }
    }

    pub fn get_request_builder(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client.request(method, url)
    }

    pub async fn execute_request(&self, request_builder: reqwest::RequestBuilder, token: Option<String>) -> Result<reqwest::Response, reqwest::Error> {
        let mut request_builder = request_builder.try_clone().unwrap();
        if let Some(token) = token {
            request_builder = request_builder.header(
                HeaderName::from_lowercase(b"authorization").unwrap(),
                HeaderValue::from_str(format!("Bearer {}", &token).as_str()).unwrap(),
            );
        }

        let request = request_builder.build().unwrap();
        match self.client.execute(request.try_clone().unwrap()).await {
            Ok(response) => {
                // Check if the response was a throttle exception (status 429 means we have been rate limited)
                if response.status() == 429 {
                    let mut retry_after: f64 = response.headers()
                        .get("retry-after").unwrap()
                        .to_str().unwrap()
                        .parse().unwrap();

                    // If it was a throttle then wait based on the retry-after response headers
                    println!("Rate limited... waiting for {} seconds before trying again", retry_after);
                    tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;

                    // Now if there is an error then pass that error along
                    self.client.execute(request).await
                } else {
                    Ok(response)
                }
            }
            Err(e) => Err(e),
        }
    }
}

pub fn get_http_client() -> ArcHttpClient {
    Arc::new(Mutex::new(HttpClient::new()))
}

/// Parse a response string into the type represented by T
/// If the `response_text` cannot be parsed into type T then it is assumed that an error
/// occurred and an shared::ErrorMessage will be returned
///
/// # Arguments
///
/// * `response_text` - A string containing the JSON response to be parsed
fn parse_response<'a, T: Deserialize<'a>>(response_text: &'a str) -> Result<T, anyhow::Error> {
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
pub async fn claim_username(http_client: ArcHttpClient, username: String) -> Result<responses::ClaimUsername, anyhow::Error> {
    let http_client = http_client.lock().await;
    let request_builder = http_client.get_request_builder(
        Method::POST,
        Url::from_str(&format!("https://api.spacetraders.io/users/{}/token", username)).unwrap(),
    ).body("this response body doesn't matter");

    let response_text = http_client.execute_request(request_builder, None).await.unwrap().text().await?;

    println!("ResponseText: {}", response_text);

    parse_response::<responses::ClaimUsername>(&response_text)
}


/// A game that is associated to a specific username
#[derive(Debug)]
pub struct Client {
    http_client: ArcHttpClient,
    /// The users spacetraders API id
    pub user_id: String,
    /// The users username
    pub username: String,
    /// The uses access token
    pub token: String,
}

impl Client {
    /// Create a new game with a reqwest client that has the Authorization header set
    ///
    /// # Arguments
    ///
    /// * `username` - A string containing the username of the current player
    /// * `token` - A string containing the access token for the username provided
    pub fn new(http_client: ArcHttpClient, user_id: String, username: String, token: String) -> Client {
        Client {
            http_client,
            user_id,
            username,
            token,
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
    /// * `id` - A string containing the flight plan id
    pub async fn get_flight_plan(&self, id: String) -> Result<responses::FlightPlan, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            format!("https://api.spacetraders.io/users/{}/flight-plans/{}", self.username, id).parse().unwrap(),
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::POST,
            format!("https://api.spacetraders.io/users/{}/flight-plans", self.username).parse().unwrap(),
        )
            .json(&flight_plan_request);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::FlightPlan>(&response_text)
    }

    /// Get the status of the game API.
    pub async fn get_game_status(&self) -> Result<responses::GameStatus, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            "https://api.spacetraders.io/game/status".parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::GameStatus>(&response_text)
    }

    /// Get all available loans
    pub async fn get_available_loans(&self) -> Result<responses::AvailableLoans, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            "https://api.spacetraders.io/game/loans".parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::AvailableLoans>(&response_text)
    }

    /// Get any loans taken out by the current user
    pub async fn get_your_loans(&self) -> Result<responses::LoanInfo, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            format!("https://api.spacetraders.io/users/{}/loans", self.username).parse().unwrap(),
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::LoanInfo>(&response_text)
    }

    /// Pay off a loan completely
    ///
    /// # Arguments
    ///
    /// * `loan_id` - A string containing the loan_id of the loan to pay off
    pub async fn pay_off_loan(&self, loan_id: String) -> Result<responses::UserInfo, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::PUT,
            format!("https://api.spacetraders.io/users/{}/loans/{}", self.username, loan_id).parse().unwrap()
        );



        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::POST,
            format!("https://api.spacetraders.io/users/{}/loans", self.username).parse().unwrap()
        )
            .json(&request_new_loan_request);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }

    /// Get location info about a specific location
    ///
    /// # Arguments
    ///
    /// * `location` - A string containing the location name to get info about
    pub async fn get_location_info(&self, location: String) -> Result<responses::LocationInfo, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
                format!("https://api.spacetraders.io/game/locations/{}", location).parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
                format!("https://api.spacetraders.io/game/systems/{}/locations", system).parse().unwrap()
        )
            .query(&query);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
                format!("https://api.spacetraders.io/game/locations/{}/marketplace", location).parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::POST,
                format!("https://api.spacetraders.io/users/{}/purchase-orders", self.username).parse().unwrap()
        )
            .json(&purchase_order_request);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

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

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::POST,
            format!("https://api.spacetraders.io/users/{}/sell-orders", self.username).parse().unwrap()
        )
            .json(&sell_order_request);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::PurchaseOrder>(&response_text)
    }

    /// Add a ship to the users inventory by purchasing it
    ///
    /// # Arguments
    ///
    /// * `location` - A string containing the location you'd like to purchase the ship from
    /// * `ship_type` - A string containing the type of ship you'd like to purchase
    pub async fn purchase_ship(&self, location: String, ship_type: String) -> Result<responses::PurchaseShip, anyhow::Error> {
        let purchase_ship_request = requests::PurchaseShipRequest {
            location,
            ship_type,
        };

        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::POST,
            format!("https://api.spacetraders.io/users/{}/ships", self.username).parse().unwrap()
        )
            .json(&purchase_ship_request);

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::PurchaseShip>(&response_text)
    }

    /// Get all ships that are available for sale
    pub async fn get_ships_for_sale(&self) -> Result<responses::ShipsForSale, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            "https://api.spacetraders.io/game/ships".parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::ShipsForSale>(&response_text)
    }

    /// Get all your ships
    pub async fn get_your_ships(&self) -> Result<responses::YourShips, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
                format!("https://api.spacetraders.io/users/{}/ships", self.username).parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::YourShips>(&response_text)
    }

    /// Get information about all systems
    pub async fn get_systems_info(&self) -> Result<responses::SystemsInfo, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
            "https://api.spacetraders.io/game/systems".parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::SystemsInfo>(&response_text)
    }

    /// Get all information about the current user
    pub async fn get_user_info(&self) -> Result<responses::UserInfo, anyhow::Error> {
        let http_client = self.http_client.lock().await;
        let request_builder = http_client.get_request_builder(
            Method::GET,
                format!("https://api.spacetraders.io/users/{}", self.username).parse().unwrap()
        );

        let response_text = http_client.execute_request(request_builder, Some(self.token.clone()))
            .await?.text().await?;

        parse_response::<responses::UserInfo>(&response_text)
    }
}
