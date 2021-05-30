mod trader;
mod scout;

use spacetraders::client::Client;
use sqlx::PgPool;
use std::fmt::Debug;
use crate::ship_machines::trader::Trader;
use crate::ship_machines::scout::Scout;

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
pub enum ShipMachine {
    Trader(Trader),
    Scout(Scout),
}

impl ShipMachine {
    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        match self {
            ShipMachine::Trader(trader) => trader.poll().await,
            ShipMachine::Scout(scout) => scout.poll().await,
        }
    }
}

pub fn new_trader_machine(client: Client, pg_pool: PgPool, username: String, ship_id: String, user_id: String) -> ShipMachine {
    ShipMachine::Trader(Trader::new(
        client,
        pg_pool,
        user_id,
        username,
        ship_id,
    ))
}

pub fn new_scout_machine(client: Client, pg_pool: PgPool, username: String, ship_id: String, user_id: String, system_symbol: String, location_symbol: String) -> ShipMachine {
    ShipMachine::Scout(Scout::new(
        client,
        pg_pool,
        user_id,
        username,
        ship_id,
        system_symbol,
        location_symbol,
    ))
}
