use crate::db;

use spacetraders::client::{Client, HttpClient};
use sqlx::PgPool;
use spacetraders::{client, responses};
use spacetraders::responses::UserInfo;
use spacetraders::shared::{LoanType, Good};
use chrono::Utc;
use std::time::Duration;
use std::convert::{TryFrom, TryInto};

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub id: String,
    pub assignment: String,
    pub system_symbol: Option<String>,
    pub location_symbol: Option<String>,
    client: Client,
    pg_pool: PgPool,
    pub info: UserInfo,
}

impl User {
    pub async fn new(http_client: HttpClient, pg_pool: PgPool, username: String, assignment: String, system_symbol: Option<String>, location_symbol: Option<String>) -> Result<User, Box<dyn std::error::Error>> {
        let db_user = db::get_user(pg_pool.clone(), username.to_owned()).await?;

        if let Some(user) = db_user {
            println!("Found existing user {}", username);
            let client = Client::new(http_client, user.username, user.token);
            let info = client.get_user_info().await?;

            Ok(
                User {
                    username,
                    id: user.id,
                    assignment,
                    system_symbol,
                    location_symbol,
                    client,
                    pg_pool,
                    info,
                }
            )
        } else {
            println!("Creating new user {}", username);
            let claimed_user = client::claim_username(http_client.clone(), username.to_owned()).await?;

            println!("Claimed new user {:?}", claimed_user);

            let user = db::persist_user(
                pg_pool.clone(),
                username.to_owned(),
                claimed_user.token.to_owned(),
                assignment.to_owned(),
                system_symbol.to_owned(),
                location_symbol.to_owned(),
            ).await?;

            println!("New user persisted");

            let client = Client::new(http_client, username.to_owned(), claimed_user.token.to_owned());
            let info = client.get_user_info().await?;

            Ok(
                User {
                    username: username.to_owned(),
                    id: user.id,
                    assignment,
                    system_symbol,
                    location_symbol,
                    client,
                    pg_pool,
                    info,
                }
            )
        }
    }

    pub async fn update_user_info(&mut self) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        self.info = self.client.get_user_info().await?;

        Ok(())
    }

    pub async fn request_new_loan(&mut self, loan_type: LoanType) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        let loan_response = self.client.request_new_loan(LoanType::Startup).await?;

        // Update our info to contain the new data from the loan response
        self.info.user.credits = loan_response.credits;
        self.info.user.loans.push(loan_response.loan);

        Ok(())
    }

    pub async fn purchase_ship(&mut self, fastest_ship_location: String, ship_type: String) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        let purchase_ship_response = self.client.purchase_ship(fastest_ship_location, ship_type).await?;

        self.info.user.credits = purchase_ship_response.credits;
        self.info.user.ships.push(purchase_ship_response.ship);

        Ok(())
    }

    pub async fn purchase_fastest_ship(&mut self) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        let available_ships = self.client.get_ships_for_sale().await?;
        let mut fastest_ship = None;
        let mut fastest_ship_speed = 0;
        let mut fastest_ship_location = "".to_string();
        let mut fastest_ship_price = 0;

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.speed > fastest_ship_speed
                    && self.info.user.credits > purchase_location.price
                    && purchase_location.location.contains(&self.system_symbol.clone().unwrap())
                {
                    fastest_ship_speed = available_ship.speed;
                    fastest_ship = Some(available_ship);
                    fastest_ship_location = purchase_location.location.to_owned();
                    fastest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = fastest_ship {
            println!("Scout {} -- Buying {} for {} at location {}", self.username, ship.ship_type.to_owned(), fastest_ship_price, fastest_ship_location);
            self.purchase_ship(fastest_ship_location, ship.ship_type.to_owned()).await;
        } else {
            panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
        }

        Ok(())
    }

    pub async fn maybe_wait_for_ship_to_arrive(&mut self, ship_index: usize) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        let ship = self.info.user.ships.get_mut(ship_index).unwrap();

        // If the ship is currently in motion then look up it's flight plan and wait for
        // the remaining time before continuing
        if ship.location == None {
            println!("Scout {} -- is currently in motion...", self.username);

            // search for any stored flight plans that are valid for this scout.
            let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &ship)
                .await.expect("Unable to get active flight plans");


            if let Some(flight_plan) = flight_plan {
                println!("Scout {} -- current flight plan {:?}", self.username, flight_plan);

                let system_location = db::get_system_location(self.pg_pool.clone(), flight_plan.destination.clone())
                    .await.expect("Unable to find a system location that should have existed in the db");

                // Adding 5 seconds here just to give the flight plan a little buffer
                let remaining_seconds = (flight_plan.arrives_at - Utc::now()).num_seconds() + 5;

                println!("Scout {} -- {} seconds remaining in flight plan... waiting", self.username, remaining_seconds);
                if remaining_seconds > 0 {
                    tokio::time::sleep(Duration::from_secs(
                        u64::try_from(remaining_seconds).expect("Invalid remaining seconds encountered")
                    )).await;
                }

                ship.location = Some(flight_plan.destination);
                ship.x = Some(system_location.x);
                ship.y = Some(system_location.y);
            }
        }

        Ok(())
    }

    pub async fn create_flight_plan(&mut self, pg_pool: PgPool, ship_index: usize, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
        let ship = self.info.user.ships.get(ship_index).unwrap();

        let flight_plan = self.client.create_flight_plan(ship.id.to_owned(), destination.to_owned()).await?;

        db::persist_flight_plan(pg_pool, self.id.clone(), ship, &flight_plan).await?;

        Ok(flight_plan)
    }

    pub async fn send_ship_to_location(&mut self, pg_pool: PgPool, ship_index: usize, location_symbol: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut ship = self.info.user.ships.get_mut(ship_index).unwrap();

        if ship.location.clone() != Some(location_symbol.clone()) {
            println!("Scout {} -- moving to location {}", self.username, location_symbol.clone());

            // If the ship has any space available fill it up with fuel
            if ship.space_available > 0 {
                println!("Scout {} -- filling ship with {} fuel", self.username, ship.space_available);
                let purchase_order = self.client.create_purchase_order(ship.to_owned(), Good::Fuel, ship.space_available).await?;

                self.info.user.credits = purchase_order.credits;

                // TODO: Does this type of assignment work? Assigning over the entire ship
                ship = &mut purchase_order.ship.clone();
            }

            let flight_plan = self.create_flight_plan(
                pg_pool.clone(),
                ship_index,
                location_symbol.clone())
                .await
                .expect("Unable to create flight plan");

            let flight_seconds = flight_plan.flight_plan.time_remaining_in_seconds + 5;
            println!("Scout {} -- waiting for {} seconds", self.username, flight_seconds);

            tokio::time::sleep(Duration::from_secs(flight_seconds.try_into().unwrap())).await;

            println!("Scout {} -- arrived at {}", self.username, location_symbol);
        }

        Ok(())
    }

    pub async fn update_marketplace_data(&self) -> Result<(), Box<dyn std::error::Error + std::marker::Send>> {
        let location_symbol = self.location_symbol.as_ref().unwrap();
        let marketplace_data = self.client.get_location_marketplace(location_symbol.clone()).await?;
        println!("Scout {} -- at {} received marketplace data {:?}", self.username, location_symbol.clone(), marketplace_data);

        for datum in marketplace_data.location.marketplace {
            db::persist_market_data(self.pg_pool.clone(), location_symbol.clone(), &datum)
                .await.expect("Unable to save market data");
        }

        Ok(())
    }

    pub async fn get_systems(&self) -> Result<responses::SystemsInfo, Box<dyn std::error::Error>> {
        let systems_info = self.client.get_systems_info().await?;
        println!("Systems info: {:?}", systems_info);

        for system in &systems_info.systems {
            for location in &system.locations {
                db::persist_system_location(self.pg_pool.clone(), system, location).await?;
            }
        }

        Ok(systems_info)
    }
}
