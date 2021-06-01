use crate::ship_machines::PollResult;
use spacetraders::client::Client;
use sqlx::PgPool;
use chrono::{DateTime, Utc, Duration};
use crate::{db, funcs};
use spacetraders::shared::Good;
use std::cmp::min;
use spacetraders::shared;

#[derive(Debug, Clone)]
enum ScoutState {
    InitializeShip,
    WaitForArrival,
    MoveToLocation,
    CheckForCorrectLocation,
    HarvestMarketData,
    Wait,
}

#[derive(Debug, Clone)]
pub struct Scout {
    client: Client,
    pg_pool: PgPool,
    user_id: String,
    username: String,
    ship: shared::Ship,
    system_symbol: String,
    location_symbol: String,
    state: ScoutState,
    arrival_time: DateTime<Utc>,
    next_harvest_time: DateTime<Utc>,
    flight_plan: Option<shared::FlightPlanData>,
}

impl Scout {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, ship: shared::Ship, system_symbol: String, location_symbol: String) -> Scout {
        Scout {
            client,
            pg_pool,
            user_id,
            username,
            ship,
            system_symbol,
            location_symbol,
            state: ScoutState::InitializeShip,
            arrival_time: Utc::now(),
            next_harvest_time: Utc::now(),
            flight_plan: None,
        }
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self.state {
            ScoutState::InitializeShip => {
                log::trace!("{}:{} -- ScoutState::InitializeShip", self.username, self.ship.id);

                if self.ship.location == None {
                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &self.ship.id)
                        .await.expect("Unable to find flight plan for a ship that is in motion").unwrap();

                    log::info!("{} -- Ship is moving to {}. Waiting for arrival", self.username, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.state = ScoutState::WaitForArrival;
                } else {
                    let mut new_user_credits = 0;
                    for cargo in self.ship.cargo.clone() {
                        log::info!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, self.ship.location.clone().unwrap());
                        let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &self.user_id, &self.ship.id, cargo.good, cargo.quantity).await?;
                        self.ship = sell_order.ship;
                        new_user_credits = sell_order.credits;
                    }

                    self.state = ScoutState::CheckForCorrectLocation;

                    if new_user_credits > 0 {
                        return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                    }
                }
            },
            ScoutState::WaitForArrival => {
                log::trace!("{}:{} -- ScoutState::WaitForArrival", self.username, self.ship.id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    log::info!("{} -- Ship traveling to {} has arrived", self.username, self.location_symbol);
                    self.state = ScoutState::CheckForCorrectLocation;
                    self.ship.location = Some(self.location_symbol.clone());
                }
            },
            ScoutState::CheckForCorrectLocation => {
                log::trace!("{}:{} -- ScoutState::CheckForCorrectLocation", self.username, self.ship.id);

                if self.ship.location == Some(self.location_symbol.clone()) {
                    log::trace!("{} -- Ship assigned to harvest market data from {} is at the correct location. Begin harvesting", self.username, self.location_symbol);
                    self.state = ScoutState::HarvestMarketData;
                } else {
                    log::trace!("{} -- Ship destined to {} was as the wrong location. Moving", self.username, self.location_symbol);
                    self.state = ScoutState::MoveToLocation;
                }
            },
            ScoutState::MoveToLocation => {
                log::trace!("{}:{} -- ScoutState::MoveToLocation", self.username, self.ship.id);

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
                    &self.location_symbol,
                ).await?;

                let mut new_user_credits = 0;
                if additional_fuel_required > 0 {
                    log::info!("{} -- Ship destined to {} is filling up with {} additional fuel", self.username, self.location_symbol, additional_fuel_required);
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

                log::info!("{} -- Ship destined to {} is creating a flight plan", self.username, self.location_symbol);
                let flight_plan = funcs::create_flight_plan(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    &self.ship.id,
                    &self.location_symbol,
                ).await?;
                self.ship.location = None;

                log::info!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.location_symbol, flight_plan.flight_plan.arrives_at);
                self.arrival_time = flight_plan.flight_plan.arrives_at;
                self.state = ScoutState::WaitForArrival;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            },
            ScoutState::HarvestMarketData => {
                log::trace!("{}:{} -- ScoutState::HarvestMarketData", self.username, self.ship.id);
                let marketplace_data = self.client.get_location_marketplace(&self.location_symbol).await?;

                log::trace!("{} -- Ship assigned to {} has received marketplace data", self.username, self.location_symbol);

                for datum in marketplace_data.marketplace {
                    db::persist_market_data(
                        self.pg_pool.clone(),
                        &self.location_symbol,
                        &datum,
                    )
                        .await.expect("Unable to save market data");
                }

                self.state = ScoutState::Wait;
                self.next_harvest_time = Utc::now() + Duration::minutes(3);
                log::trace!("{} -- Ship assigned to {} will check market data again at {}", self.username, self.location_symbol, self.next_harvest_time);
            },
            ScoutState::Wait => {
                log::trace!("{}:{} -- ScoutState::Wait", self.username, self.ship.id);
                if Utc::now().ge(&self.next_harvest_time) {
                    log::trace!("{} -- Ship assigned to {} to harvest market data has finished waiting for next harvest time", self.username, self.location_symbol);
                    self.state = ScoutState::HarvestMarketData;
                }
            },
        }

        Ok(None)
    }
}
