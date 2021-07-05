pub(crate) mod builder;
mod trader;
mod scout;
mod system_change;

use spacetraders::client::Client;
use sqlx::PgPool;
use std::fmt::Debug;
use crate::ship_machines::trader::Trader;
use crate::ship_machines::scout::Scout;
use crate::ship_machines::system_change::SystemChange;

#[derive(Debug, Clone)]
pub enum PollResult {
    UpdateCredits(i32),
    ConvertToNewMachine(MachineType),
}

#[derive(Debug, Clone)]
pub enum ShipAssignment {
    Trader,
    Scout,
    SystemChange,
}

#[derive(Debug, Clone)]
pub enum MachineType {
    Trader(Trader),
    Scout(Scout),
    SystemChange(SystemChange),
}

#[derive(Debug, Clone)]
pub struct ShipMachine {
    client: Client,
    pg_pool: PgPool,
    trader_machine: Option<Trader>,
    scout_machine: Option<Scout>,
    system_change_machine: Option<SystemChange>,

    user_id: String,
    username: String,
    ship_id: String,
    system: String,
    location: String,
}

// This should be a sort of operator pattern. It will maintain it's current state but also reload
// the desired state occasionally. If the desired state and the current state are different then
// the ship will take action to progress to it's desired state.
// I.E. If the ship is currently trading in the OE system but the desired state is trading
//      in the XV system the ship will finish it's current trade and convert itself to a
//      system_change machine and move to the correct location. After it has arrived it will
//      convert back to a trader machine and will start trading in the XV system.
impl ShipMachine {
    pub fn get_ship_id(&self) -> &str {
        &self.ship_id
    }

    pub async fn poll(&mut self) -> anyhow::Result<Option<PollResult>> {
        if let Some(trader_machine) = &mut self.trader_machine {
            return match trader_machine.poll().await.unwrap().unwrap() {
                PollResult::UpdateCredits(new_credits) => Ok(Some(PollResult::UpdateCredits(new_credits))),
                PollResult::ConvertToNewMachine(new_machine) => match new_machine {
                    MachineType::Trader(trader) => {
                        self.scout_machine = None;
                        self.system_change_machine = None;
                        self.trader_machine = Some(trader);

                        Ok(None)
                    },
                    MachineType::Scout(scout) => {
                        self.scout_machine = Some(scout);
                        self.system_change_machine = None;
                        self.trader_machine = None;

                        Ok(None)
                    }
                    MachineType::SystemChange(system_change) => {
                        self.scout_machine = None;
                        self.system_change_machine = Some(system_change);
                        self.trader_machine = None;

                        Ok(None)
                    }
                }
            }
        }

        if let Some(scout_machine) = &mut self.scout_machine {
            return scout_machine.poll().await;
        }

        if let Some(system_change_machine) = &mut self.system_change_machine {
            return system_change_machine.poll().await;
        }

        unreachable!("Shouldn't have made it here. This means that a ship machine didn't have an underlying machine attached to it")
    }

    pub async fn reset(&mut self) -> anyhow::Result<()> {
        if let Some(trader_machine) = &mut self.trader_machine {
            return trader_machine.reset().await;
        }

        if let Some(scout_machine) = &mut self.scout_machine {
            return scout_machine.reset().await;
        }

        if let Some(system_change_machine) = &mut self.system_change_machine {
            return system_change_machine.reset().await;
        }

        unreachable!("Shouldn't have made it here. This means that a ship machine didn't have an underlying machine attached to it")
    }
}
