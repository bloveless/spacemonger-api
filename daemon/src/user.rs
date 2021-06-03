use crate::db;

use spacetraders::client::{Client, HttpClient};
use sqlx::PgPool;
use spacetraders::{client, responses, shared};
use spacetraders::responses::MyShips;
use spacetraders::shared::LoanType;
use spacetraders::errors::SpaceTradersClientError;
use crate::ship_machines::{ShipMachine, ShipAssignment, new_trader_machine, new_scout_machine};

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub token: String,
    pub id: String,
    client: Client,
    pg_pool: PgPool,
    assignment: ShipAssignment,
    pub ship_machines: Vec<ShipMachine>,
    pub loans: Vec<shared::Loan>,
    pub outstanding_loans: usize,
    pub credits: i32,
}

impl User {
    pub async fn new(http_client: HttpClient, pg_pool: PgPool, username: String, assignment: ShipAssignment) -> anyhow::Result<User> {
        let db_user = db::get_user(pg_pool.clone(), username.clone()).await?;

        if let Some(user) = db_user {
            log::debug!("Found existing user {}", username);
            let client = Client::new(http_client, user.username, user.token.clone());
            let info = client.get_my_info().await?;
            let ships = client.get_my_ships().await?;
            let loans = client.get_my_loans().await?;

            log::info!("User credits {}", info.user.credits);

            let mut user = User {
                username,
                token: user.token.clone(),
                id: user.id,
                client,
                pg_pool: pg_pool.clone(),
                assignment: assignment.clone(),
                ship_machines: Vec::new(),
                credits: info.user.credits,
                loans: loans.loans.clone(),
                outstanding_loans: loans.loans.into_iter().filter(|f| { !f.status.contains("PAID") }).count()
            };

            user.add_ship_machines_from_user_info(&ships, &assignment);

            for ship in &ships.ships {
                db::persist_ship(pg_pool.clone(), &user.id, ship).await?;
            }

            Ok(user)
        } else {
            log::debug!("Creating new user {}", username);
            let claimed_user = client::claim_username(http_client.clone(), username.clone()).await?;

            log::info!("Claimed new user {:?}", claimed_user);

            let db_user = db::persist_user(
                pg_pool.clone(),
                username.clone(),
                claimed_user.token.clone(),
                assignment.clone(),
            ).await?;

            log::debug!("New user persisted");

            let client = Client::new(http_client, username.clone(), claimed_user.token.clone());
            let info = client.get_my_info().await?;
            let ships = client.get_my_ships().await?;
            let loans = client.get_my_loans().await?;

            log::info!("User credits {}", info.user.credits);

            let mut user = User {
                username: username.clone(),
                token: claimed_user.token.clone(),
                id: db_user.id,
                client,
                pg_pool: pg_pool.clone(),
                assignment: assignment.clone(),
                ship_machines: Vec::new(),
                credits: info.user.credits,
                loans: loans.loans.clone(),
                outstanding_loans: loans.loans.into_iter().filter(|f| { !f.status.contains("PAID") }).count()
            };

            user.add_ship_machines_from_user_info(&ships, &assignment);

            for ship in &ships.ships {
                db::persist_ship(pg_pool.clone(), &user.id, ship).await?;
            }

            Ok(user)
        }
    }

    fn add_ship_machines_from_user_info(&mut self, ships: &MyShips, assignment: &ShipAssignment) {
        self.ship_machines = ships.ships.clone().into_iter().map(|ship| {
            self.ship_to_machine(&ship, &assignment)
        }).collect()
    }

    fn ship_to_machine(&self, ship: &shared::Ship, assignment: &ShipAssignment) -> ShipMachine {
        match assignment {
            ShipAssignment::Trader => new_trader_machine(
                self.client.clone(),
                self.pg_pool.clone(),
                self.username.clone(),
                self.id.clone(),
                ship.clone(),
            ),
            ShipAssignment::Scout { system_symbol, location_symbol } => new_scout_machine(
                self.client.clone(),
                self.pg_pool.clone(),
                self.username.clone(),
                self.id.clone(),
                ship.clone(),
                system_symbol.to_owned(),
                location_symbol.to_owned()
            )
        }
    }

    pub async fn request_new_loan(&mut self, loan_type: LoanType) -> anyhow::Result<()> {
        let loan_response = self.client.request_new_loan(loan_type).await?;

        // Update our info to contain the new data from the loan response
        self.credits = loan_response.credits;

        // Keep track of loans...
        self.loans.push(loan_response.loan);
        self.outstanding_loans = self.loans.clone().into_iter().filter(|f| { !f.status.contains("PAID") }).count();

        Ok(())
    }

    pub async fn purchase_ship(&mut self, fastest_ship_location: String, ship_type: String) -> anyhow::Result<()> {
        let purchase_ship_response = self.client.purchase_ship(fastest_ship_location, ship_type).await?;

        // TODO: Record new ship
        db::persist_ship(self.pg_pool.clone(), &self.id, &purchase_ship_response.ship).await?;

        self.credits = purchase_ship_response.credits;
        self.ship_machines.push(self.ship_to_machine(&purchase_ship_response.ship, &self.assignment));

        Ok(())
    }

    pub async fn purchase_fastest_ship(&mut self, system_symbol: Option<&str>) -> anyhow::Result<()> {
        let available_ships = self.client.get_ships_for_sale().await?;
        let mut fastest_ship = None;
        let mut fastest_ship_speed: i32 = 0;
        let mut fastest_ship_location = "".to_string();
        let mut fastest_ship_price: i32 = 0;

        let ships = self.client.get_my_ships().await?;
        let ships_count = ships.ships.len();
        let valid_locations: Vec<String> = ships.ships
            .into_iter()
            .filter(|s| s.location != None)
            .map(|s| s.location.unwrap())
            .collect();

        log::info!("{} -- Valid locations to purchase a ship from are {:?}", self.username, valid_locations.clone());
        log::info!("{} -- User currently has {} ships", self.username, ships_count);
        log::info!("{} -- Ships available for purchase {:?}", self.username, available_ships.clone());

        if ships_count > 0 && valid_locations.is_empty() {
            log::warn!("{} -- No docked ships found to purchase ships with. Will retry later", self.username);
            return Ok(());
        }

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.speed > fastest_ship_speed
                    && available_ship.restricted_goods == None
                    && self.credits > purchase_location.price
                    && (ships_count == 0 || valid_locations.contains(&purchase_location.location))
                    && (system_symbol.is_none() || purchase_location.system == system_symbol.unwrap())
                {
                    fastest_ship_speed = available_ship.speed;
                    fastest_ship = Some(available_ship);
                    fastest_ship_location = purchase_location.location.clone();
                    fastest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = fastest_ship {
            log::info!("Ship {} -- Buying {} for {} at location {}", self.username, ship.ship_type.clone(), fastest_ship_price, fastest_ship_location);
            self.purchase_ship(fastest_ship_location, ship.ship_type.clone()).await?;
        } else {
            log::warn!("Unable to find a ship for the user to purchase");
        }

        Ok(())
    }

    pub async fn purchase_largest_ship(&mut self, system_symbol: Option<&str>) -> anyhow::Result<()> {
        let available_ships = self.client.get_ships_for_sale().await?;
        let mut largest_ship = None;
        let mut largest_ship_capacity: i32 = 0;
        let mut largest_ship_location = "".to_string();
        let mut largest_ship_price: i32 = 0;

        let ships = self.client.get_my_ships().await?;
        let ships_count = ships.ships.len();
        let valid_locations: Vec<String> = ships.ships
            .into_iter()
            .filter(|s| s.location != None)
            .map(|s| s.location.unwrap())
            .collect();

        log::info!("{} -- Valid locations to purchase a ship from are {:?}", self.username, valid_locations.clone());
        log::info!("{} -- User currently has {} ships", self.username, ships_count);
        log::info!("{} -- Ships available for purchase {:?}", self.username, available_ships.clone());

        if ships_count > 0 && valid_locations.is_empty() {
            log::warn!("{} -- No docked ships found to purchase ships with. Will retry later", self.username);
            return Ok(());
        }

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.max_cargo > largest_ship_capacity
                    && available_ship.restricted_goods == None
                    && self.credits > purchase_location.price
                    && (ships_count == 0 || valid_locations.contains(&purchase_location.location))
                    && (system_symbol.is_none() || purchase_location.system == system_symbol.unwrap())
                {
                    largest_ship_capacity = available_ship.max_cargo;
                    largest_ship = Some(available_ship);
                    largest_ship_location = purchase_location.location.clone();
                    largest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = largest_ship {
            log::info!("Ship {} -- Buying {} for {} at location {}", self.username, ship.ship_type.clone(), largest_ship_price, largest_ship_location);
            self.purchase_ship(largest_ship_location, ship.ship_type.clone()).await?;
        } else {
            log::warn!("Unable to find a ship for the user to purchase");
        }

        Ok(())
    }

    pub async fn get_systems(&self) -> anyhow::Result<responses::SystemsInfo> {
        let systems_info = self.client.get_systems_info().await?;
        log::debug!("Systems info: {:?}", systems_info);

        for system in &systems_info.systems {
            for location in &system.locations {
                db::persist_system_location(self.pg_pool.clone(), system, location).await?;
            }
        }

        Ok(systems_info)
    }

    pub async fn get_my_ships(&self) -> Result<responses::MyShips, SpaceTradersClientError> {
        self.client.get_my_ships().await
    }

    pub async fn pay_off_loan(&self, loan_id: &str) -> Result<responses::PayLoanResponse, SpaceTradersClientError> {
        self.client.pay_off_loan(loan_id).await
    }
}
