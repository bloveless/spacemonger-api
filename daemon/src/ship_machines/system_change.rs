use spacetraders::client::Client;
use sqlx::PgPool;
use spacetraders::shared;
use chrono::{DateTime, Utc};
use spacetraders::shared::Good;
use crate::ship_machines::PollResult;
use crate::{db, funcs};
use std::cmp::min;
use crate::ship_machines::trader::Trader;

#[derive(Debug, Clone)]
enum SystemChangeState {
    InitializeShip,
    WaitForArrival,
    MoveToWormhole,
    WaitForArrivalAtWormhole,
    Warp,
    WaitForWarp,
}


#[derive(Debug, Clone)]
pub struct SystemChange {
    pub client: Client,
    pub pg_pool: PgPool,
    pub user_id: String,
    pub username: String,
    pub ship: shared::Ship,
    pub system: String,
    state: SystemChangeState,
    arrival_time: DateTime<Utc>,
    flight_plan: Option<shared::FlightPlanData>,
}

impl SystemChange {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, system: String, ship: shared::Ship) -> SystemChange {
        SystemChange {
            client,
            pg_pool,
            user_id,
            username,
            ship,
            system,
            state: SystemChangeState::InitializeShip,
            arrival_time: Utc::now(),
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
        self.state = SystemChangeState::InitializeShip;

        Ok(())
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self.state {
            SystemChangeState::InitializeShip => {
                log::trace!("{}:{} -- SystemChangeState::InitializeShip", self.username, self.ship.id);

                if self.ship.location == None {
                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &self.ship.id)
                        .await.expect("Unable to find flight plan for a ship that is in motion").unwrap();

                    log::info!("{}:{} -- Ship is moving to {}. Waiting for arrival", self.username, self.ship.id, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.flight_plan = Some(flight_plan);
                    self.state = SystemChangeState::WaitForArrival;
                } else {
                    self.state = SystemChangeState::MoveToWormhole;
                }
            },
            SystemChangeState::WaitForArrival => {
                log::trace!("{}:{} -- SystemChangeState::WaitForArrival", self.username, self.ship.id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    if let Some(flight_plan) = self.flight_plan.clone() {
                        log::info!("{}:{} -- Ship traveling to {} has arrived", self.username, self.ship.id, flight_plan.destination);
                        self.state = SystemChangeState::MoveToWormhole;
                        self.ship.location = Some(flight_plan.destination);
                    }
                }
            },
            SystemChangeState::MoveToWormhole => {
                log::trace!("{}:{} -- SystemChangeState::MoveToWormhole", self.username, self.ship.id);
                if self.ship.location == None {
                    // We shouldn't be here without our ship having a location.
                    // Let's just restart and wait for the ship to stop somewhere
                    self.state = SystemChangeState::InitializeShip;
                    return Ok(None);
                }

                let mut new_user_credits = 0;
                for c in &self.ship.cargo.clone() {
                    let sell_order = funcs::create_sell_order(
                        self.client.clone(),
                        self.pg_pool.clone(),
                        &self.user_id,
                        c.good,
                        c.quantity,
                        &mut self.ship,
                    ).await?;

                    new_user_credits = sell_order.credits;
                    self.ship = sell_order.ship;
                }

                if let Some(location) = &self.ship.location {
                    let wormhole = db::get_wormhole_from_location_to_system(self.pg_pool.clone(), location, &self.system).await?;

                    let current_fuel = self.ship.cargo.iter().filter(|c| c.good == Good::Fuel).fold(0, |acc, c| acc + c.quantity);

                    let additional_fuel_required = funcs::get_additional_fuel_required_for_trip(
                        self.pg_pool.clone(),
                        self.client.clone(),
                        &self.ship.id,
                        &self.ship.ship_type,
                        current_fuel,
                        &location,
                        &wormhole,
                    ).await?;

                    if additional_fuel_required > 0 {
                        log::info!("{}:{} -- Ship destined to {} is filling up with {} additional fuel", self.username, self.ship.id, wormhole, additional_fuel_required);
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

                    let flight_plan = funcs::create_flight_plan(self.client.clone(), self.pg_pool.clone(), &self.user_id, &wormhole, &mut self.ship).await?;
                    self.arrival_time = flight_plan.flight_plan.arrives_at;
                    self.flight_plan = Some(flight_plan.flight_plan);
                    self.state = SystemChangeState::WaitForArrivalAtWormhole;
                }

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            },
            SystemChangeState::WaitForArrivalAtWormhole => {
                log::trace!("{}:{} -- SystemChangeState::WaitForArrivalAtWormhole", self.username, self.ship.id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    if let Some(flight_plan) = self.flight_plan.clone() {
                        log::info!("{}:{} -- Ship traveling to {} has arrived", self.username, self.ship.id, flight_plan.destination);
                        self.state = SystemChangeState::MoveToWormhole;
                        self.ship.location = Some(flight_plan.destination);
                    }
                }
            },
            SystemChangeState::Warp => {
                log::trace!("{}:{} -- SystemChangeState::Warp", self.username, self.ship.id);
                let flight_plan = self.client.attempt_warp_jump(self.ship.id.to_string()).await?;
                self.arrival_time = flight_plan.flight_plan.arrives_at;
                self.flight_plan = Some(flight_plan.flight_plan);
                self.state = SystemChangeState::WaitForWarp;
            },
            SystemChangeState::WaitForWarp => {
                log::trace!("{}:{} -- SystemChangeState::WaitForWarp", self.username, self.ship.id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    if let Some(flight_plan) = self.flight_plan.clone() {
                        log::info!("{}:{} -- Ship traveling to {} has arrived", self.username, self.ship.id, flight_plan.destination);
                        self.state = SystemChangeState::MoveToWormhole;
                        self.ship.location = Some(flight_plan.destination);
                    }
                }
            }
        }

        Ok(None)
    }
}

impl From<&mut Trader> for SystemChange {
    fn from(trader: &mut Trader) -> Self {
        SystemChange {
            client: trader.client.clone(),
            pg_pool: trader.pg_pool.clone(),
            user_id: trader.user_id.clone(),
            username: trader.username.clone(),
            ship: trader.ship.clone(),
            system: trader.system.clone(),
            state: SystemChangeState::InitializeShip,
            arrival_time: Utc::now(),
            flight_plan: None
        }
    }
}
