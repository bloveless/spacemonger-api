use crate::ship_machines::PollResult;
use spacetraders::client::Client;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::{db, funcs};
use crate::db::DbRoute;
use spacetraders::shared;
use spacetraders::shared::Good;
use std::cmp::min;
use rand::seq::SliceRandom;

#[derive(Debug, Clone)]
enum TraderState {
    InitializeShip,
    WaitForArrival,
    PickBestTrade,
    // MoveToLocation,
    ExecuteTrade,
    // SellAllCargo,
    // PurchaseMaxGoodForTrading,
    MoveToRandomLocation,
    // PickRandomLocation,
}

#[derive(Debug, Clone)]
pub struct Trader {
    client: Client,
    pg_pool: PgPool,
    user_id: String,
    username: String,
    ship: shared::Ship,
    state: TraderState,
    arrival_time: DateTime<Utc>,
    route: Option<DbRoute>,
    flight_plan: Option<shared::FlightPlanData>,
}

impl Trader {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, ship: shared::Ship) -> Trader {
        Trader {
            client,
            pg_pool,
            user_id,
            username,
            ship,
            state: TraderState::InitializeShip,
            arrival_time: Utc::now(),
            route: None,
            flight_plan: None,
        }
    }

    pub fn get_ship_id(&self) -> String {
        self.ship.id.clone()
    }

    pub async fn reset(&mut self) -> anyhow::Result<()> {
        log::info!("{}:{} -- Ship is being reset", self.username, self.ship.id);

        // First we will abandon all cargo
        for cargo in &self.ship.cargo {
            self.client.jettison_cargo(self.ship.id.clone(), cargo.good, cargo.quantity).await?;
        }

        // Next we will re-initialize the ship which will wait for the ship to arrive and restart
        // it's loop
        self.state = TraderState::InitializeShip;

        Ok(())
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self.state {
            TraderState::InitializeShip => {
                log::trace!("{}:{} -- TraderState::InitializeShip", self.username, self.ship.id);

                if self.ship.location == None {
                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &self.ship.id)
                        .await.expect("Unable to find flight plan for a ship that is in motion").unwrap();

                    log::info!("{} -- Ship is moving to {}. Waiting for arrival", self.username, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.flight_plan = Some(flight_plan);
                    self.state = TraderState::WaitForArrival;
                } else {
                    let mut new_user_credits = 0;
                    for cargo in self.ship.cargo.clone() {
                        if cargo.quantity > 0 {
                            log::info!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, self.ship.location.clone().unwrap());
                            let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, &self.ship.id, cargo.good, cargo.quantity).await?;
                            self.ship = sell_order.ship;
                            new_user_credits = sell_order.credits;
                        }
                    }

                    self.state = TraderState::PickBestTrade;

                    if new_user_credits > 0 {
                        return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                    }
                }
            },
            TraderState::WaitForArrival => {
                log::trace!("{}:{} -- TraderState::WaitForArrival", self.username, self.ship.id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    log::info!("{} -- Ship traveling to {} has arrived", self.username, self.flight_plan.clone().unwrap().destination);
                    self.ship.location = Some(self.flight_plan.clone().unwrap().destination);
                    self.state = TraderState::PickBestTrade;
                }
            },
            TraderState::MoveToRandomLocation => {
                log::trace!("{}:{} -- TraderState::MoveToRandomLocation", self.username, self.ship.id);

                if self.ship.location.is_none() {
                    log::warn!("{} -- We can't pick a random location while a ship is in motion... trying again later", self.username);
                    return Ok(None);
                }

                let current_ship_location = self.ship.location.clone().unwrap();

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

                        let current_fuel = self.ship.cargo.clone().into_iter()
                            .filter(|c| c.good == Good::Fuel)
                            .fold(0, |acc, c| acc + c.quantity);

                        let additional_fuel_required = funcs::get_additional_fuel_required_for_trip(
                            self.pg_pool.clone(),
                            self.client.clone(),
                            &self.ship.id,
                            &self.ship.ship_type,
                            current_fuel,
                            &self.ship.location.clone().unwrap(),
                            location,
                        ).await?;

                        let mut new_user_credits = 0;
                        if additional_fuel_required > 0 {
                            log::info!("{} -- Ship destined to {} is filling up with {} additional fuel", self.username, location, additional_fuel_required);
                            let purchase_order = funcs::create_purchase_order(
                                self.client.clone(),
                                self.pg_pool.clone(),
                                &self.user_id,
                                &self.ship.id,
                                Good::Fuel,
                                // Don't ever try and buy more fuel than the ship can hold
                                min(additional_fuel_required, self.ship.space_available),
                            ).await?;

                            new_user_credits = purchase_order.credits;
                            self.ship = purchase_order.ship;
                        }

                        log::info!("{} -- Ship destined to {} is creating a flight plan", self.username, location);
                        let flight_plan = funcs::create_flight_plan(
                            self.client.clone(),
                            self.pg_pool.clone(),
                            &self.user_id,
                            &self.ship.id,
                            &location,
                        ).await?;
                        self.ship.location = None;
                        self.flight_plan = Some(flight_plan.flight_plan.clone());
                        self.ship.cargo = self.ship.cargo.clone().into_iter().map(|mut c| {
                            if c.good == Good::Fuel {
                                c.quantity -= flight_plan.flight_plan.fuel_consumed;
                            }

                            c
                        }).collect();

                        log::info!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, location, flight_plan.flight_plan.arrives_at);
                        self.arrival_time = flight_plan.flight_plan.arrives_at;
                        self.flight_plan = Some(flight_plan.flight_plan);
                        self.state = TraderState::WaitForArrival;

                        if new_user_credits > 0 {
                            return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                        }
                    }
                    None => {
                        log::error!("{} -- Unable to find a new random location to move to... trying again", self.username);
                    }
                }
            },
            TraderState::PickBestTrade => {
                log::trace!("{}:{} -- TraderState::PickBestTrade", self.username, self.ship.id);

                let origin = self.ship.location.clone().unwrap();

                let routes = funcs::get_routes_for_ship(
                    self.pg_pool.clone(),
                    &origin,
                        self.ship.speed
                ).await?;

                log::debug!("{} -- Routes: {:?}", self.username, routes);

                for route in routes {
                    if route.sell_location_symbol != "OE-XV-91-2" && route.purchase_quantity > 500 && route.profit_speed_volume_distance > 0.0 {
                        log::info!("{} -- Trading {} from {} to {} (purchase quantity {}, sell quantity {})", self.username, route.good, route.purchase_location_symbol, route.sell_location_symbol, route.purchase_quantity, route.sell_quantity);

                        self.route = Some(route);
                        self.state = TraderState::ExecuteTrade;

                        return Ok(None);
                    }
                }

                log::warn!("{} -- Found no available routes from {}. Randomly picking a new location to move to in this system", self.username, origin);
                self.state = TraderState::MoveToRandomLocation;
            },
            TraderState::ExecuteTrade => {
                log::trace!("{}:{} -- TraderState::Execute", self.username, self.ship.id);

                if self.route.is_none() {
                    log::warn!("{}:{} -- Tried to execute a trade without a route. Picking a new route", self.username, self.ship.id);
                    // Somehow we ended up here without a route... go back and pick a route
                    self.state = TraderState::PickBestTrade;
                }

                // After we arrive at a location
                let route = self.route.clone().unwrap();

                // Just a safe-guard at the cost of an extra API call to make sure that we have the
                // most current cargo manifest for the ship.
                // TODO: Ideally we wouldn't need this here but the fuel we used to get here
                //       has changed the cargo and we don't have that change yet.
                let ship = self.client.get_my_ship(&self.ship.id).await?;
                self.ship = ship.ship;

                // sell everything (we can't omit fuel here because sometimes the best trade is fuel)
                let mut new_user_credits = 0;
                for cargo in self.ship.cargo.clone() {
                    if cargo.quantity > 0 {
                        log::info!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, self.ship.location.clone().unwrap());
                        let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, &self.ship.id, cargo.good, cargo.quantity).await?;
                        self.ship = sell_order.ship;
                        new_user_credits = sell_order.credits;
                    }
                }

                let current_fuel = self.ship.cargo.clone().into_iter().filter(|c| c.good == Good::Fuel).fold(0, |acc, c| acc + c.quantity);

                let additional_fuel_required = funcs::get_additional_fuel_required_for_trip(
                    self.pg_pool.clone(),
                    self.client.clone(),
                    &self.ship.id,
                    &self.ship.ship_type,
                    current_fuel,
                    &route.purchase_location_symbol,
                    &route.sell_location_symbol,
                ).await?;

                if additional_fuel_required > 0 {
                    log::info!("{} -- Ship destined to {} is filling up with {} additional fuel", self.username, route.sell_location_symbol, additional_fuel_required);
                    let purchase_order = funcs::create_purchase_order(
                        self.client.clone(),
                        self.pg_pool.clone(),
                        &self.user_id,
                        &self.ship.id,
                        Good::Fuel,
                        // Don't ever try and buy more fuel than the ship can hold
                        min(additional_fuel_required, self.ship.space_available),
                    ).await?;

                    new_user_credits = purchase_order.credits;
                    self.ship = purchase_order.ship;
                }

                log::info!(
                    "{} -- Purchasing {} {} for trading (volume per unit {}). Purchase price at {} is {}. Sell price at {} is {}",
                    self.username,
                    self.ship.space_available / route.good.get_volume(),
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
                    &self.ship.id,
                    route.good,
                    self.ship.space_available / route.good.get_volume(),
                ).await {
                    Ok(purchase_order) => {
                        log::info!("{} -- Ship destined to {} is creating a flight plan", self.username, route.sell_location_symbol);
                        let flight_plan = funcs::create_flight_plan(
                            self.client.clone(),
                            self.pg_pool.clone(),
                            &self.user_id,
                            &self.ship.id,
                            &route.sell_location_symbol,
                        ).await?;
                        self.ship.location = None;
                        self.ship.cargo = self.ship.cargo.clone().into_iter().map(|mut c| {
                            if c.good == Good::Fuel {
                                c.quantity -= flight_plan.flight_plan.fuel_consumed;
                            }

                            c
                        }).collect();

                        log::info!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, route.sell_location_symbol, flight_plan.flight_plan.arrives_at);
                        self.arrival_time = flight_plan.flight_plan.arrives_at;
                        self.flight_plan = Some(flight_plan.flight_plan);
                        self.state = TraderState::WaitForArrival;

                        return Ok(Some(PollResult::UpdateCredits(purchase_order.credits)));
                    },
                    Err(e) => {
                        log::error!("{} -- Unable to create purchase order. Picking a new trade. Error: {}", self.username, e);
                        // If there is any error then pick another trade
                        self.state = TraderState::PickBestTrade;
                    }
                }
            },
        }

        Ok(None)
    }
}
