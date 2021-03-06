use crate::ship_machines::{PollResult, MachineType};
use spacetraders::client::Client;
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::{db, funcs};
use crate::db::DbRoute;
use spacetraders::shared;
use spacetraders::shared::Good;
use std::cmp::min;
use rand::seq::SliceRandom;
use crate::ship_machines::{system_change::SystemChange, ShipMachine};

#[derive(Debug, Clone)]
enum TraderState {
    InitializeShip,
    WaitForArrival,
    PickBestTrade,
    // MoveToLocation,
    ExecuteTrade,
    // PurchaseMaxGoodForTrading,
    MoveToRandomLocation,
    // PickRandomLocation,
}

#[derive(Debug, Clone)]
pub struct Trader {
    pub client: Client,
    pub pg_pool: PgPool,
    pub user_id: String,
    pub username: String,
    pub system: String,
    pub ship: shared::Ship,
    state: TraderState,
    arrival_time: DateTime<Utc>,
    route: Option<DbRoute>,
    flight_plan: Option<shared::FlightPlanData>,
}

impl Trader {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, system: String, ship: shared::Ship) -> Trader {
        Trader {
            client,
            pg_pool,
            user_id,
            username,
            system,
            ship,
            state: TraderState::InitializeShip,
            arrival_time: Utc::now(),
            route: None,
            flight_plan: None,
        }
    }

    pub async fn reset(&mut self) -> anyhow::Result<()> {
        log::info!("{}:{} -- Ship is being reset", self.username, self.ship.id);

        let ship = self.client.get_my_ship(&self.ship.id).await?;
        self.ship = ship.ship;

        // First we will jettison all cargo
        for cargo in &self.ship.cargo {
            self.client.jettison_cargo(&self.ship.id, cargo.good, cargo.quantity).await?;
        }

        self.ship.cargo.clear();

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

                    log::info!("{}:{} -- Ship is moving to {}. Waiting for arrival", self.username, self.ship.id, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.flight_plan = Some(flight_plan);
                    self.state = TraderState::WaitForArrival;
                } else {
                    let mut new_user_credits = 0;
                    for cargo in self.ship.cargo.clone() {
                        if cargo.quantity > 0 {
                            log::info!("{}:{} -- Selling {} goods {} at {}", self.username, self.ship.id, cargo.quantity, cargo.good, self.ship.location.clone().unwrap());
                            let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, cargo.good, cargo.quantity, &mut self.ship).await?;
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
                    log::info!("{}:{} -- Ship traveling to {} has arrived", self.username, self.ship.id, self.flight_plan.clone().unwrap().destination);
                    self.ship.location = Some(self.flight_plan.clone().unwrap().destination);
                    self.state = TraderState::PickBestTrade;
                }
            },
            TraderState::MoveToRandomLocation => {
                log::trace!("{}:{} -- TraderState::MoveToRandomLocation", self.username, self.ship.id);

                if self.ship.location.is_none() {
                    log::warn!("{}:{} -- We can't pick a random location while a ship is in motion... trying again later", self.username, self.ship.id);
                    return Ok(None);
                }

                let current_ship_location = self.ship.location.clone().unwrap();

                let locations = db::get_system_locations_from_location(self.pg_pool.clone(), &current_ship_location).await?;

                log::debug!("{}:{} -- Found locations in system to randomly pick from {:?}", self.username, self.ship.id, locations);

                let new_location = locations.choose(&mut rand::thread_rng());

                match new_location {
                    Some(location) => {
                        // Don't try and send a ship to it's current location
                        if *location == current_ship_location {
                            log::debug!("{}:{} -- Tried to send ship to it's current location... picking another location", self.username, self.ship.id);
                            return Ok(None);
                        }

                        log::info!("{}:{} -- Randomly picked {} to start trading at", self.username, self.ship.id, location);

                        let current_fuel = self.ship.cargo.iter()
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
                            log::info!("{}:{} -- Ship destined to {} is filling up with {} additional fuel", self.username, self.ship.id, location, additional_fuel_required);
                            let purchase_order = funcs::create_purchase_order(
                                self.client.clone(),
                                self.pg_pool.clone(),
                                &self.user_id,
                                Good::Fuel,
                                // Don't ever try and buy more fuel than the ship can hold
                                min(additional_fuel_required, self.ship.space_available),
                                &mut self.ship,
                            ).await?;

                            new_user_credits = purchase_order.credits;
                        }

                        log::info!("{}:{} -- Ship destined to {} is creating a flight plan", self.username, self.ship.id, location);
                        let flight_plan = funcs::create_flight_plan(
                            self.client.clone(),
                            self.pg_pool.clone(),
                            &self.user_id,
                            &location,
                            &mut self.ship
                        ).await?;

                        self.flight_plan = Some(flight_plan.flight_plan.clone());

                        log::info!("{}:{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.ship.id, location, flight_plan.flight_plan.arrives_at);
                        self.arrival_time = flight_plan.flight_plan.arrives_at;
                        self.flight_plan = Some(flight_plan.flight_plan);
                        self.state = TraderState::WaitForArrival;

                        if new_user_credits > 0 {
                            return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                        }
                    }
                    None => {
                        log::error!("{}:{} -- Unable to find a new random location to move to... trying again", self.username, self.ship.id);
                    }
                }
            },
            TraderState::PickBestTrade => {
                log::trace!("{}:{} -- TraderState::PickBestTrade", self.username, self.ship.id);

                let mut new_user_credits = 0;
                for cargo in self.ship.cargo.clone() {
                    if cargo.quantity > 0 {
                        log::info!("{}:{} -- Selling {} goods {} at {}", self.username, self.ship.id, cargo.quantity, cargo.good, self.ship.location.clone().unwrap());
                        let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, cargo.good, cargo.quantity, &mut self.ship).await?;
                        new_user_credits = sell_order.credits;
                    }
                }

                // TODO: This is the optimal place for making changes to the current ship state
                // I.E. if a ship is in system OE but the DB says it should be in XV then
                //      we should convert the ship here and return
                let db_ship = db::get_ship(self.pg_pool.clone(), &self.user_id, &self.ship.id).await?;
                if db_ship.system != self.system {
                    log::trace!("{}:{} -- TraderState::ConvertToNewMachine", self.username, self.ship.id);
                    log::trace!("{}:{} -- Detected that the ship is in {} and needs to move to {}", self.username, self.ship.id, self.system, db_ship.system);
                    return Ok(Some(PollResult::ConvertToNewMachine(MachineType::SystemChange(self.into()))))
                }

                let origin = self.ship.location.clone().unwrap();

                let routes = funcs::get_routes_for_ship(
                    self.pg_pool.clone(),
                    &origin,
                        self.ship.speed
                ).await?;

                log::debug!("{}:{} -- Routes: {:?}", self.username, self.ship.id, routes);

                for route in routes {
                    if route.sell_location != "OE-XV-91-2" && route.purchase_quantity > 500 && route.profit_speed_volume_distance > 0.0 {
                        log::info!("{}:{} -- Trading {} from {} to {} (purchase quantity {}, sell quantity {})", self.username, self.ship.id, route.good, route.purchase_location, route.sell_location, route.purchase_quantity, route.sell_quantity);

                        self.route = Some(route);
                        self.state = TraderState::ExecuteTrade;

                        return Ok(None);
                    }
                }

                log::warn!("{}:{} -- Found no available routes from {}. Randomly picking a new location to move to in this system", self.username, self.ship.id, origin);
                self.state = TraderState::MoveToRandomLocation;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
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

                let current_fuel = self.ship.cargo.iter().filter(|c| c.good == Good::Fuel).fold(0, |acc, c| acc + c.quantity);

                let additional_fuel_required = funcs::get_additional_fuel_required_for_trip(
                    self.pg_pool.clone(),
                    self.client.clone(),
                    &self.ship.id,
                    &self.ship.ship_type,
                    current_fuel,
                    &route.purchase_location,
                    &route.sell_location,
                ).await?;

                let mut new_user_credits = 0;
                if additional_fuel_required > 0 {
                    log::info!("{}:{} -- Ship destined to {} is filling up with {} additional fuel", self.username, self.ship.id, route.sell_location, additional_fuel_required);
                    let purchase_order = funcs::create_purchase_order(
                        self.client.clone(),
                        self.pg_pool.clone(),
                        &self.user_id,
                        Good::Fuel,
                        // Don't ever try and buy more fuel than the ship can hold
                        min(additional_fuel_required, self.ship.space_available),
                        &mut self.ship,
                    ).await?;

                    new_user_credits = purchase_order.credits;
                    self.ship = purchase_order.ship;
                }

                log::debug!("{}:{} -- Current space available {}", self.username, self.ship.id, self.ship.space_available);

                log::info!(
                    "{}:{} -- Purchasing {} {} for trading (volume per unit {}). Purchase price at {} is {}. Sell price at {} is {}",
                    self.username,
                    self.ship.id,
                    self.ship.space_available / route.good.get_volume(),
                    route.good,
                    route.good.get_volume(),
                    route.purchase_location,
                    route.purchase_price_per_unit,
                    route.sell_location,
                    route.sell_price_per_unit
                );

                match funcs::create_purchase_order(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    route.good,
                    self.ship.space_available / route.good.get_volume(),
                    &mut self.ship,
                ).await {
                    Ok(purchase_order) => {
                        self.ship = purchase_order.ship;

                        log::info!("{}:{} -- Ship destined to {} is creating a flight plan", self.username, self.ship.id, route.sell_location);
                        let flight_plan = funcs::create_flight_plan(
                            self.client.clone(),
                            self.pg_pool.clone(),
                            &self.user_id,
                            &route.sell_location,
                            &mut self.ship,
                        ).await?;

                        log::info!("{}:{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.ship.id, route.sell_location, flight_plan.flight_plan.arrives_at);
                        self.arrival_time = flight_plan.flight_plan.arrives_at;
                        self.flight_plan = Some(flight_plan.flight_plan);
                        self.state = TraderState::WaitForArrival;

                        return Ok(Some(PollResult::UpdateCredits(purchase_order.credits)));
                    },
                    Err(e) => {
                        log::error!("{}:{} -- Unable to create purchase order. Picking a new trade. Error: {}", self.username, self.ship.id, e);
                        // If there is any error then pick another trade
                        self.state = TraderState::PickBestTrade;
                    }
                }

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            },
        }

        Ok(None)
    }
}

impl From<&mut SystemChange> for Trader {
    fn from(system_change: &mut SystemChange) -> Self {
        Trader {
            client: system_change.client.clone(),
            pg_pool: system_change.pg_pool.clone(),
            user_id: system_change.user_id.clone(),
            username: system_change.username.clone(),
            system: system_change.system.clone(),
            ship: system_change.ship.clone(),
            state: TraderState::InitializeShip,
            arrival_time: Utc::now(),
            route: None,
            flight_plan: None
        }
    }
}
