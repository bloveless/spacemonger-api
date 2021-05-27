use spacetraders::client::Client;
use sqlx::PgPool;
use spacetraders::shared::Good;
use chrono::{DateTime, Utc, Duration};
use crate::{db, funcs};
use crate::db::DbRoute;
use std::cmp::{max, min};
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
pub enum PollResult {
    UpdateCredits(i32),
}

#[derive(Debug, Clone)]
pub enum ShipAssignment {
    Trader,
    Scout { system_symbol: String, location_symbol: String },
}

#[derive(Debug, Clone)]
enum ShipState {
    // Shared states, CheckIfMoving is the starting state
    CheckIfMoving,
    WaitForArrival,
    MoveToLocation,

    // Scout specific states
    CheckForCorrectLocation,
    HarvestMarketData,
    Wait,

    // Trader specific states
    SellAllCargo,
    PickBestTrade,
    PurchaseMaxGoodForTrading,
    PickRandomLocation,
}

#[derive(Debug, Clone)]
pub struct ShipMachine {
    client: Client,
    pg_pool: PgPool,
    user_id: String,
    username: String,
    ship_id: String,
    assignment: ShipAssignment,
    destination: String,
    state: ShipState,
    arrival_time: DateTime<Utc>,
    next_harvest_time: DateTime<Utc>,
    route: Option<DbRoute>,
}

impl ShipMachine {
    pub fn new(client: Client, pg_pool: PgPool, username: String, ship_id: String, user_id: String, assignment: ShipAssignment) -> ShipMachine {
        ShipMachine {
            client,
            pg_pool,
            user_id,
            username,
            ship_id,
            assignment: assignment.clone(),
            destination: match assignment {
                ShipAssignment::Trader => String::new(),
                ShipAssignment::Scout { system_symbol: _ , location_symbol } => location_symbol,
            },
            state: ShipState::CheckIfMoving,
            arrival_time: Utc::now(),
            next_harvest_time: Utc::now(),
            route: None,
        }
    }

    pub async fn poll(&mut self) -> Result<Option<PollResult>, Box<dyn std::error::Error>> {
        match self.state {
            ShipState::CheckIfMoving => {
                let ships = self.client.get_my_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                if ship.location == None {
                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &self.ship_id)
                        .await.expect("Unable to find flight plan for a ship that is in motion").unwrap();

                    log::info!("{} -- Ship is moving to {}. Waiting for arrival", self.username, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.state = ShipState::WaitForArrival;
                } else {
                    self.state = ShipState::CheckForCorrectLocation;
                }
            }
            ShipState::CheckForCorrectLocation => {
                let ships = self.client.get_my_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                match self.assignment.clone() {
                    ShipAssignment::Trader => {
                        // Traders don't really have a correct location so just start the trade loop
                        self.state = ShipState::SellAllCargo;
                    }
                    ShipAssignment::Scout { system_symbol: _, location_symbol } => {
                        if ship.location == Some(self.destination.clone()) {
                            log::trace!("{} -- Ship assigned to harvest market data from {} is at the correct location. Begin harvesting", self.username, location_symbol);
                            self.state = ShipState::HarvestMarketData;
                        } else {
                            log::trace!("{} -- Ship destined to {} was as the wrong location. Moving", self.username, self.destination);
                            self.state = ShipState::MoveToLocation;
                        }
                    }
                }
            }
            ShipState::WaitForArrival => {
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    log::info!("{} -- Ship traveling to {} has arrived", self.username, self.destination);
                    self.state = ShipState::CheckForCorrectLocation;
                }
            }
            ShipState::PickRandomLocation => {
                let my_ship = self.client.get_my_ship(&self.ship_id).await?;

                if my_ship.ship.location.is_none() {
                    log::warn!("{} -- We can't pick a random location while a ship is in motion... trying again later", self.username);
                    return Ok(None);
                }

                let current_ship_location = my_ship.ship.location.unwrap();

                let locations = db::get_system_locations_from_location(self.pg_pool.clone(), &current_ship_location).await?;

                log::debug!("{} -- Found locations in system to randomly pick from {:?}", self.username, locations);

                let new_location = locations.choose(&mut rand::thread_rng());

                match new_location {
                    Some(location) => {
                        // Don't try and send a ship to it's current location
                        if *location == current_ship_location {
                            log::debug!("{} -- Tried to send ship to it's current location... picking another location", self.username);
                            return Ok(None);
                        }

                        log::info!("{} -- Randomly picked {} to start trading at", self.username, location);
                        self.destination = location.clone();
                        self.state = ShipState::MoveToLocation;
                    }
                    None => {
                        log::error!("{} -- Unable to find a new random location to move to... trying again", self.username);
                    }
                }
            }
            ShipState::MoveToLocation => {
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let current_fuel = ship.ship.cargo.into_iter()
                    .filter(|c| c.good == Good::Fuel)
                    .fold(0, |acc, c| acc + c.quantity);

                let fuel_required = funcs::get_fuel_required_for_trip(self.pg_pool.clone(), &ship.ship.location.unwrap(), &self.destination, &ship.ship.ship_type).await?;
                let fuel_required = fuel_required.ceil() as i32;

                let mut new_user_credits = 0;
                if current_fuel < fuel_required {
                    log::info!("{} -- Ship destined to {} is filling up with {} fuel", self.username, self.destination, fuel_required);
                    let purchase_order = funcs::create_purchase_order(
                        self.client.clone(),
                        self.pg_pool.clone(),
                        &self.user_id,
                        &self.ship_id,
                        Good::Fuel,
                        // Don't ever try and buy more fuel than the ship can hold
                        min(fuel_required - current_fuel, ship.ship.space_available),
                    ).await?;

                    new_user_credits = purchase_order.credits;
                }

                log::info!("{} -- Ship destined to {} is creating a flight plan", self.username, self.destination);
                let flight_plan = funcs::create_flight_plan(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    &self.ship_id,
                    &self.destination,
                ).await?;

                log::info!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.destination, flight_plan.flight_plan.arrives_at);
                self.arrival_time = flight_plan.flight_plan.arrives_at;
                self.state = ShipState::WaitForArrival;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            }
            ShipState::HarvestMarketData => match self.assignment.clone() {
                ShipAssignment::Trader => panic!("ShipState::HarvestMarketData -- Traders do not harvest market data... yet"),
                ShipAssignment::Scout { system_symbol: _, location_symbol } => {
                    let marketplace_data = self.client.get_location_marketplace(&self.destination).await?;

                    log::trace!("{} -- Ship assigned to {} has received marketplace data", self.username, location_symbol);

                    for datum in marketplace_data.marketplace {
                        db::persist_market_data(
                            self.pg_pool.clone(),
                            &location_symbol,
                            &datum,
                        )
                            .await.expect("Unable to save market data");
                    }

                    self.state = ShipState::Wait;
                    self.next_harvest_time = Utc::now() + Duration::minutes(3);
                    log::trace!("{} -- Ship assigned to {} will check market data again at {}", self.username, location_symbol, self.next_harvest_time);
                }
            },
            ShipState::Wait => match self.assignment.clone() {
                ShipAssignment::Trader => panic!("ShipState::Wait -- Traders do not harvest market data... yet"),
                ShipAssignment::Scout { system_symbol: _, location_symbol } => {
                    if Utc::now().ge(&self.next_harvest_time) {
                        log::trace!("{} -- Ship assigned to {} to harvest market data has finished waiting for next harvest time", self.username, location_symbol);
                        self.state = ShipState::HarvestMarketData;
                    }
                }
            },
            ShipState::SellAllCargo => {
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let mut new_user_credits = 0;
                for cargo in ship.ship.cargo {
                    log::info!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, ship.ship.location.clone().unwrap());
                    let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, &self.ship_id, cargo.good, cargo.quantity).await?;
                    new_user_credits = sell_order.credits;
                }

                self.state = ShipState::PickBestTrade;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            }
            ShipState::PickBestTrade => {
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let routes = funcs::get_routes_for_ship(
                    self.pg_pool.clone(),
                    &ship.ship
                ).await?;

                log::debug!("{} -- Routes: {:?}", self.username, routes);

                for route in routes {
                    if route.sell_location_symbol != "OE-XV-91-2" && route.purchase_quantity > 500 && route.profit_speed_volume_distance > 0.0 {
                        log::info!("{} -- Trading {} from {} to {} (purchase quantity {}, sell quantity {})", self.username, route.good, route.purchase_location_symbol, route.sell_location_symbol, route.purchase_quantity, route.sell_quantity);

                        self.route = Some(route);
                        self.state = ShipState::PurchaseMaxGoodForTrading;

                        return Ok(None);
                    }
                }

                log::warn!("{} -- Found no available routes from {}. Randomly picking a new location to move to in this system", self.username, ship.ship.location.unwrap());
                self.state = ShipState::PickRandomLocation;
            }
            ShipState::PurchaseMaxGoodForTrading => {
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let route = self.route.clone().unwrap();

                let fuel_required = funcs::get_fuel_required_for_trip(
                    self.pg_pool.clone(),
                    &ship.ship.location.unwrap(),
                    &route.sell_location_symbol,
                    &ship.ship.ship_type,
                ).await?;

                let current_fuel = ship.ship.cargo.into_iter()
                    .filter(|c| c.good == Good::Fuel)
                    .fold(0, |acc, c| acc + c.quantity);

                log::info!("{} -- Ship space available {}, fuel required {}, current fuel {}", self.username, ship.ship.space_available, fuel_required.ceil() as i32, current_fuel);

                let fuel_required: i32 = max(fuel_required.ceil() as i32 - current_fuel, 0);

                let room_available_for_trading = ship.ship.space_available - fuel_required;

                log::info!(
                    "{} -- Purchasing {} {} for trading (volume per unit {}). Purchase price at {} is {}. Sell price at {} is {}",
                    self.username,
                    room_available_for_trading / route.good.get_volume(),
                    route.good,
                    route.good.get_volume(),
                    route.purchase_location_symbol,
                    route.purchase_price_per_unit,
                    route.sell_location_symbol,
                    route.sell_price_per_unit
                );

                match funcs::create_purchase_order(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    &self.ship_id,
                    route.good,
                    room_available_for_trading / route.good.get_volume(),
                ).await {
                    Ok(purchase_order) => {
                        self.destination = route.sell_location_symbol;
                        self.state = ShipState::MoveToLocation;

                        return Ok(Some(PollResult::UpdateCredits(purchase_order.credits)));
                    },
                    Err(e) => {
                        log::error!("{} -- Unable to create purchase order. Picking a new trade. Error: {}", self.username, e);
                        // If there is any error then pick another trade
                        self.state = ShipState::PickBestTrade;
                    }
                }
            }
        }

        Ok(None)
    }
}
