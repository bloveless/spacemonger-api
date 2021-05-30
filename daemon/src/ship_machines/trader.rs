use crate::ship_machines::PollResult;
use spacetraders::client::Client;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::{db, funcs};
use crate::db::DbRoute;
use spacetraders::shared;
use spacetraders::shared::Good;
use std::cmp::{max, min};
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
enum TraderState {
    CheckIfMoving,
    WaitForArrival,
    MoveToLocation,
    SellAllCargo,
    PickBestTrade,
    PurchaseMaxGoodForTrading,
    PickRandomLocation,
}

#[derive(Debug, Clone)]
pub struct Trader {
    client: Client,
    pg_pool: PgPool,
    user_id: String,
    username: String,
    ship_id: String,
    state: TraderState,
    arrival_time: DateTime<Utc>,
    route: Option<DbRoute>,
    flight_plan: Option<shared::FlightPlanData>,
    destination: String,
}

impl Trader {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, ship_id: String) -> Trader {
        Trader {
            client,
            pg_pool,
            user_id,
            username,
            ship_id,
            state: TraderState::CheckIfMoving,
            arrival_time: Utc::now(),
            route: None,
            flight_plan: None,
            destination: String::new(),
        }
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self.state {
            TraderState::CheckIfMoving => {
                log::trace!("{}:{} -- TraderState::CheckIfMoving", self.username, self.ship_id);

                // search for any stored flight plans that are valid for this scout.
                match db::get_active_flight_plan(self.pg_pool.clone(), &self.ship_id)
                    .await.expect("Unable to find flight plan for a ship that is in motion") {
                    Some(flight_plan) => {
                        log::info!("{} -- Ship is moving to {}. Waiting for arrival", self.username, flight_plan.destination);
                        self.destination = flight_plan.destination.clone();
                        self.flight_plan = Some(flight_plan);
                        self.state = TraderState::WaitForArrival;
                    },
                    None => {
                        self.state = TraderState::SellAllCargo;
                    }
                }
            },
            TraderState::WaitForArrival => {
                log::trace!("{}:{} -- TraderState::WaitForArrival", self.username, self.ship_id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    log::info!("{} -- Ship traveling to {} has arrived", self.username, self.flight_plan.clone().unwrap().destination);
                    self.state = TraderState::SellAllCargo;
                }
            },
            TraderState::PickRandomLocation => {
                log::trace!("{}:{} -- TraderState::PickRandomLocation", self.username, self.ship_id);
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
                        self.state = TraderState::MoveToLocation;
                    }
                    None => {
                        log::error!("{} -- Unable to find a new random location to move to... trying again", self.username);
                    }
                }
            },
            TraderState::SellAllCargo => {
                log::trace!("{}:{} -- TraderState::SellAllCargo", self.username, self.ship_id);
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let mut new_user_credits = 0;
                for cargo in ship.ship.cargo {
                    log::info!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, ship.ship.location.clone().unwrap());
                    let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, &self.ship_id, cargo.good, cargo.quantity).await?;
                    new_user_credits = sell_order.credits;
                }

                self.state = TraderState::PickBestTrade;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            },
            TraderState::PickBestTrade => {
                log::trace!("{}:{} -- TraderState::PickBestTrade", self.username, self.ship_id);
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let routes = funcs::get_routes_for_ship(
                    self.pg_pool.clone(),
                    &ship.ship
                ).await?;

                log::debug!("{} -- Routes: {:?}", self.username, routes);

                for route in routes {
                    if route.sell_location_symbol != "OE-XV-91-2" && route.purchase_quantity > 500 && route.profit_speed_volume_distance > 0.0 {
                        log::info!("{} -- Trading {} from {} to {} (purchase quantity {}, sell quantity {})", self.username, route.good, route.purchase_location_symbol, route.sell_location_symbol, route.purchase_quantity, route.sell_quantity);

                        self.destination = route.sell_location_symbol.clone();
                        self.route = Some(route);
                        self.state = TraderState::PurchaseMaxGoodForTrading;

                        return Ok(None);
                    }
                }

                log::warn!("{} -- Found no available routes from {}. Randomly picking a new location to move to in this system", self.username, ship.ship.location.unwrap());
                self.state = TraderState::PickRandomLocation;
            },
            TraderState::PurchaseMaxGoodForTrading => {
                log::trace!("{}:{} -- TraderState::PurchaseMaxGoodForTrading", self.username, self.ship_id);
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
                        self.state = TraderState::MoveToLocation;

                        return Ok(Some(PollResult::UpdateCredits(purchase_order.credits)));
                    },
                    Err(e) => {
                        log::error!("{} -- Unable to create purchase order. Picking a new trade. Error: {}", self.username, e);
                        // If there is any error then pick another trade
                        self.state = TraderState::PickBestTrade;
                    }
                }
            },
            TraderState::MoveToLocation => {
                log::trace!("{}:{} -- TraderState::MoveToLocation", self.username, self.ship_id);
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
                self.flight_plan = Some(flight_plan.flight_plan);
                self.state = TraderState::WaitForArrival;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            }
        }

        Ok(None)
    }
}
