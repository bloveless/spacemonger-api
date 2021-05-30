use crate::ship_machines::PollResult;
use spacetraders::client::Client;
use sqlx::PgPool;
use chrono::{DateTime, Utc, Duration};
use crate::{db, funcs};
use spacetraders::shared::Good;
use std::cmp::min;

#[derive(Debug, Clone)]
enum ScoutState {
    CheckIfMoving,
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
    ship_id: String,
    system_symbol: String,
    location_symbol: String,
    state: ScoutState,
    arrival_time: DateTime<Utc>,
    next_harvest_time: DateTime<Utc>,
}

impl Scout {
    pub fn new(client: Client, pg_pool: PgPool, user_id: String, username: String, ship_id: String, system_symbol: String, location_symbol: String) -> Scout {
        Scout {
            client,
            pg_pool,
            user_id,
            username,
            ship_id,
            system_symbol,
            location_symbol,
            state: ScoutState::CheckIfMoving,
            arrival_time: Utc::now(),
            next_harvest_time: Utc::now(),
        }
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self.state {
            ScoutState::CheckIfMoving => {
                log::trace!("{}:{} -- ScoutState::CheckIfMoving", self.username, self.ship_id);
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
                    self.state = ScoutState::WaitForArrival;
                } else {
                    self.state = ScoutState::CheckForCorrectLocation;
                }
            },
            ScoutState::MoveToLocation => {
                log::trace!("{}:{} -- ScoutState::MoveToLocation", self.username, self.ship_id);
                // TODO: This should be in funcs so that scouts and traders can share it
                let ship = self.client.get_my_ship(&self.ship_id).await?;

                let current_fuel = ship.ship.cargo.into_iter()
                    .filter(|c| c.good == Good::Fuel)
                    .fold(0, |acc, c| acc + c.quantity);

                let fuel_required = funcs::get_fuel_required_for_trip(self.pg_pool.clone(), &ship.ship.location.unwrap(), &self.location_symbol, &ship.ship.ship_type).await?;
                let fuel_required = fuel_required.ceil() as i32;

                let mut new_user_credits = 0;
                if current_fuel < fuel_required {
                    log::info!("{} -- Ship destined to {} is filling up with {} fuel", self.username, self.location_symbol, fuel_required);
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

                log::info!("{} -- Ship destined to {} is creating a flight plan", self.username, self.location_symbol);
                let flight_plan = funcs::create_flight_plan(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    &self.ship_id,
                    &self.location_symbol,
                ).await?;

                log::info!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.location_symbol, flight_plan.flight_plan.arrives_at);
                self.arrival_time = flight_plan.flight_plan.arrives_at;
                self.state = ScoutState::WaitForArrival;

                if new_user_credits > 0 {
                    return Ok(Some(PollResult::UpdateCredits(new_user_credits)));
                }
            },
            ScoutState::CheckForCorrectLocation => {
                log::trace!("{}:{} -- ScoutState::CheckForCorrectLocation", self.username, self.ship_id);
                let ships = self.client.get_my_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                if ship.location == Some(self.location_symbol.clone()) {
                    log::trace!("{} -- Ship assigned to harvest market data from {} is at the correct location. Begin harvesting", self.username, self.location_symbol);
                    self.state = ScoutState::HarvestMarketData;
                } else {
                    log::trace!("{} -- Ship destined to {} was as the wrong location. Moving", self.username, self.location_symbol);
                    self.state = ScoutState::MoveToLocation;
                }
            },
            ScoutState::WaitForArrival => {
                log::trace!("{}:{} -- ScoutState::WaitForArrival", self.username, self.ship_id);
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    log::info!("{} -- Ship traveling to {} has arrived", self.username, self.location_symbol);
                    self.state = ScoutState::CheckForCorrectLocation;
                }
            },
            ScoutState::HarvestMarketData => {
                log::trace!("{}:{} -- ScoutState::HarvestMarketData", self.username, self.ship_id);
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
                log::trace!("{}:{} -- ScoutState::Wait", self.username, self.ship_id);
                if Utc::now().ge(&self.next_harvest_time) {
                    log::trace!("{} -- Ship assigned to {} to harvest market data has finished waiting for next harvest time", self.username, self.location_symbol);
                    self.state = ScoutState::HarvestMarketData;
                }
            },
        }

        Ok(None)
    }
}
