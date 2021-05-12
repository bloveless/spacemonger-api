use crate::db;

use spacetraders::client::{Client, HttpClient};
use sqlx::PgPool;
use spacetraders::{client, responses, shared};
use spacetraders::responses::UserInfo;
use spacetraders::shared::LoanType;
use crate::ship_machine::{ShipMachine, ShipAssignment};

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub token: String,
    pub id: String,
    client: Client,
    pg_pool: PgPool,
    assignment: ShipAssignment,
    pub ship_machines: Vec<ShipMachine>,
    pub credits: i32,
}

impl User {
    pub async fn new(http_client: HttpClient, pg_pool: PgPool, username: String, assignment: ShipAssignment) -> Result<User, Box<dyn std::error::Error>> {
        let db_user = db::get_user(pg_pool.clone(), username.clone()).await?;

        if let Some(user) = db_user {
            println!("Found existing user {}", username);
            let client = Client::new(http_client, user.username, user.token.clone());
            let info = client.get_user_info().await?;

            println!("User credits {}", info.user.credits);

            let mut user = User {
                username,
                token: user.token.clone(),
                id: user.id,
                client,
                pg_pool,
                assignment: assignment.clone(),
                ship_machines: Vec::new(),
                credits: info.user.credits,
            };

            user.add_ship_machines_from_user_info(&info, &assignment);

            Ok(user)
        } else {
            println!("Creating new user {}", username);
            let claimed_user = client::claim_username(http_client.clone(), username.clone()).await?;

            println!("Claimed new user {:?}", claimed_user);

            let db_user = db::persist_user(
                pg_pool.clone(),
                username.clone(),
                claimed_user.token.clone(),
                assignment.clone(),
            ).await?;

            println!("New user persisted");

            let client = Client::new(http_client, username.clone(), claimed_user.token.clone());
            let info = client.get_user_info().await?;

            println!("User credits {}", info.user.credits);

            let mut user = User {
                username: username.clone(),
                token: claimed_user.token.clone(),
                id: db_user.id,
                client,
                pg_pool,
                assignment: assignment.clone(),
                ship_machines: Vec::new(),
                credits: info.user.credits,
            };

            user.add_ship_machines_from_user_info(&info,  &assignment);

            Ok(user)
        }
    }

    fn add_ship_machines_from_user_info(&mut self, info: &UserInfo, assignment: &ShipAssignment) {
        self.ship_machines = info.user.ships.clone().into_iter().map(|ship| {
            self.ship_to_machine(&ship, &assignment)
        }).collect()
    }

    fn ship_to_machine(&self, ship: &shared::Ship, assignment: &ShipAssignment) -> ShipMachine {
        ShipMachine::new(
            self.client.clone(),
            self.pg_pool.clone(),
            self.username.clone(),
            ship.id.clone(),
            self.id.clone(),
            assignment.clone(),
        )
    }

    pub async fn request_new_loan(&mut self, loan_type: LoanType) -> Result<(), Box<dyn std::error::Error>> {
        let loan_response = self.client.request_new_loan(loan_type).await?;

        // Update our info to contain the new data from the loan response
        self.credits = loan_response.credits;

        // TODO: Keep track of loans... maybe
        // self.info.user.loans.push(loan_response.loan);

        Ok(())
    }

    pub async fn purchase_ship(&mut self, fastest_ship_location: String, ship_type: String) -> Result<(), Box<dyn std::error::Error>> {
        let purchase_ship_response = self.client.purchase_ship(fastest_ship_location, ship_type).await?;

        self.credits = purchase_ship_response.credits;
        self.ship_machines.push(self.ship_to_machine(&purchase_ship_response.ship, &self.assignment));

        Ok(())
    }

    pub async fn purchase_fastest_ship(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let available_ships = self.client.get_ships_for_sale().await?;
        let mut fastest_ship = None;
        let mut fastest_ship_speed = 0;
        let mut fastest_ship_location = "".to_string();
        let mut fastest_ship_price = 0;

        let ships = self.client.get_your_ships().await?;
        let ships_count = ships.ships.len();
        let valid_locations: Vec<String> = ships.ships
            .into_iter()
            .filter(|s| s.location != None)
            .map(|s| s.location.unwrap())
            .collect();

        if ships_count > 0 && valid_locations.len() == 0 {
            println!("{} -- No docked ships found to purchase ships with. Will retry later", self.username);
            return Ok(());
        }

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.speed > fastest_ship_speed
                    && available_ship.restricted_goods == None
                    && self.credits > purchase_location.price
                    && (ships_count == 0 || valid_locations.contains(&purchase_location.location))
                {
                    fastest_ship_speed = available_ship.speed;
                    fastest_ship = Some(available_ship);
                    fastest_ship_location = purchase_location.location.clone();
                    fastest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = fastest_ship {
            println!("Ship {} -- Buying {} for {} at location {}", self.username, ship.ship_type.clone(), fastest_ship_price, fastest_ship_location);
            self.purchase_ship(fastest_ship_location, ship.ship_type.clone()).await?;
        } else {
            panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
        }

        Ok(())
    }

    pub async fn purchase_largest_ship(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let available_ships = self.client.get_ships_for_sale().await?;
        let mut largest_ship = None;
        let mut largest_ship_capacity = 0;
        let mut largest_ship_location = "".to_string();
        let mut largest_ship_price = 0;

        let ships = self.client.get_your_ships().await?;
        let ships_count = ships.ships.len();
        let valid_locations: Vec<String> = ships.ships
            .into_iter()
            .filter(|s| s.location != None)
            .map(|s| s.location.unwrap())
            .collect();

        if ships_count > 0 && valid_locations.len() == 0 {
            println!("{} -- No docked ships found to purchase ships with. Will retry later", self.username);
            return Ok(());
        }

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.max_cargo > largest_ship_capacity
                    && available_ship.restricted_goods == None
                    && self.credits > purchase_location.price
                    && (ships_count == 0 || valid_locations.contains(&purchase_location.location))
                {
                    largest_ship_capacity = available_ship.max_cargo;
                    largest_ship = Some(available_ship);
                    largest_ship_location = purchase_location.location.clone();
                    largest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = largest_ship {
            println!("Ship {} -- Buying {} for {} at location {}", self.username, ship.ship_type.clone(), largest_ship_price, largest_ship_location);
            self.purchase_ship(largest_ship_location, ship.ship_type.clone()).await?;
        } else {
            panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
        }

        Ok(())
    }

    // pub async fn maybe_wait_for_ship_to_arrive(&mut self, ship_index: usize) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get_mut(ship_index).unwrap();
    //
    //     // If the ship is currently in motion then look up it's flight plan and wait for
    //     // the remaining time before continuing
    //     if ship.location == None {
    //         println!("Ship {} -- is currently in motion...", self.username);
    //
    //         // search for any stored flight plans that are valid for this scout.
    //         let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &ship.id)
    //             .await.expect("Unable to get active flight plans");
    //
    //
    //         if let Some(flight_plan) = flight_plan {
    //             println!("Ship {} -- current flight plan {:?}", self.username, flight_plan);
    //
    //             let system_location = db::get_system_location(self.pg_pool.clone(), flight_plan.destination.clone())
    //                 .await.expect("Unable to find a system location that should have existed in the db");
    //
    //             // Adding 5 seconds here just to give the flight plan a little buffer
    //             let remaining_seconds = (flight_plan.arrives_at - Utc::now()).num_seconds() + 5;
    //
    //             println!("Ship {} -- {} seconds remaining in flight plan... waiting", self.username, remaining_seconds);
    //             if remaining_seconds > 0 {
    //                 tokio::time::sleep(Duration::from_secs(
    //                     u64::try_from(remaining_seconds).expect("Invalid remaining seconds encountered")
    //                 )).await;
    //             }
    //
    //             ship.location = Some(flight_plan.destination);
    //             ship.x = Some(system_location.x);
    //             ship.y = Some(system_location.y);
    //         }
    //     }
    //
    //     Ok(())
    // }

    // pub async fn create_flight_plan(&mut self, pg_pool: PgPool, ship_index: usize, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     let flight_plan = self.client.create_flight_plan(ship.id.clone(), destination.clone()).await?;
    //
    //     // db::persist_flight_plan(pg_pool, self.id.clone(), ship, &flight_plan).await?;
    //
    //     Ok(flight_plan)
    // }

    // pub async fn fill_ship_with_max_good(&mut self, ship_index: usize, good: Good) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     // If the ship has any space available fill it up with fuel
    //     if ship.space_available > 0 {
    //         println!("Ship {} -- filling ship with {} {:?}", self.username, ship.space_available, good);
    //
    //         let volume_per_unit = (db::get_good_volume(self.pg_pool.clone(), good).await)
    //             .unwrap_or(1);
    //
    //         self.create_purchase_order(
    //             ship_index,
    //             good,
    //             ship.space_available / volume_per_unit,
    //         ).await?;
    //     }
    //
    //     Ok(())
    // }

    // pub async fn sell_max_good(&mut self, ship_index: usize, good: Good) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     let good_quantity = ship.cargo.clone().into_iter()
    //         .filter(|s| s.good == good)
    //         .fold(0, |acc, good| acc + good.quantity);
    //
    //     if good_quantity > 0 {
    //         println!("Ship {} -- selling {} good {:?}", self.username, good_quantity, good);
    //
    //         // self.create_sell_order(ship_index, good, good_quantity).await?;
    //     }
    //
    //     Ok(())
    // }

    // pub async fn create_purchase_order(&mut self, ship_index: usize, good: Good, quantity: i32) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     println!("Ship {} -- Purchasing good {:?} Quantity: {}", self.username, good, quantity);
    //     let purchase_order = self.client.create_purchase_order(ship.id.clone(), good, quantity).await?;
    //     println!("Ship {} -- New credits {}", self.username, purchase_order.credits);
    //
    //     self.info.user.credits = purchase_order.credits;
    //     self.info.user.ships.splice(ship_index..=ship_index, vec!(purchase_order.ship));
    //
    //     Ok(())
    // }

    // pub fn print_ship_cargo(&self, ship_index: usize) {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     for cargo in ship.cargo.clone() {
    //         println!("Ship {} -- Cargo {:?} Quantity: {} Volume: {}", self.username, cargo.good, cargo.quantity, cargo.total_volume);
    //     }
    // }

    // pub async fn ensure_ship_has_enough_fuel(&mut self, ship_index: usize, fuel_required: i32) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get(ship_index).unwrap();
    //
    //     let ship_fuel_penalty = match ship.ship_type.as_str() {
    //         "GR-MK-II" => 1,
    //         "GR-MK-III" => 2,
    //         _ => 0,
    //     };
    //
    //     let fuel_required = fuel_required + ship_fuel_penalty;
    //
    //     let current_fuel = ship.clone()
    //         .cargo.into_iter()
    //         .filter(|c| c.good == Good::Fuel)
    //         .fold(0, |sum, c| sum + c.quantity);
    //
    //     println!("Ship currently has {} fuel", current_fuel);
    //
    //     if current_fuel < fuel_required {
    //         println!("Purchasing {} fuel", fuel_required - current_fuel);
    //         // TODO: We may need to pad this fuel amount depending on if our calculation is good enough
    //         self.create_purchase_order(ship_index, Good::Fuel, fuel_required - current_fuel).await?;
    //     }
    //
    //     Ok(())
    // }

    // pub async fn send_ship_to_location(&mut self, ship_index: usize, location_symbol: String) -> Result<(), Box<dyn std::error::Error>> {
    //     let ship = self.info.user.ships.get_mut(ship_index).unwrap();
    //
    //     if ship.location.clone() != Some(location_symbol.clone()) {
    //         println!("Ship {} -- moving to location {}", self.username, location_symbol);
    //
    //         let flight_plan = self.create_flight_plan(
    //             self.pg_pool.clone(),
    //             ship_index,
    //             location_symbol.clone())
    //             .await
    //             .expect("Unable to create flight plan");
    //
    //         println!("Ship {} -- requires {} fuel for flight to {}", self.username, flight_plan.flight_plan.fuel_consumed, location_symbol);
    //
    //         let flight_seconds = flight_plan.flight_plan.time_remaining_in_seconds + 5;
    //         println!("Ship {} -- waiting for {} seconds", self.username, flight_seconds);
    //
    //         tokio::time::sleep(Duration::from_secs(flight_seconds.try_into().unwrap())).await;
    //
    //         println!("Ship {} -- arrived at {}", self.username, location_symbol);
    //     }
    //
    //     // TODO: Is there a better way instead of refreshing my entire user again here
    //     //       I need to update all the information about the ship after this flight happened
    //     //       but the flight details just return a bunch of pieces of data rather than the whole
    //     //       ship again
    //     self.update_user_info().await?;
    //
    //     Ok(())
    // }

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
