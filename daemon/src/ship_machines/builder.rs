use spacetraders::client::Client;
use sqlx::PgPool;
use crate::ship_machines::{ShipAssignment, ShipMachine};
use crate::ship_machines::trader::Trader;
use crate::ship_machines::scout::Scout;
use crate::ship_machines::system_change::SystemChange;
use spacetraders::shared;

#[derive(Debug, Clone)]
pub struct ShipMachineBuilder {
    client: Option<Client>,
    pg_pool: Option<PgPool>,
    assignment: Option<ShipAssignment>,

    // Only one of these should be present at any time
    trader_machine: Option<Trader>,
    scout_machine: Option<Scout>,
    system_change_machine: Option<SystemChange>,

    user_id: Option<String>,
    username: Option<String>,
    system: Option<String>,
    location: Option<String>,
    ship: Option<shared::Ship>,
}

impl ShipMachineBuilder {
    pub fn new() -> ShipMachineBuilder {
        ShipMachineBuilder {
            client: None,
            pg_pool: None,
            assignment: None,
            trader_machine: None,
            scout_machine: None,
            system_change_machine: None,
            user_id: None,
            username: None,
            system: None,
            location: None,
            ship: None,
        }
    }

    pub fn client(&mut self, client: Client) -> &mut ShipMachineBuilder {
        self.client = Some(client);
        self
    }

    pub fn pg_pool(&mut self, pg_pool: PgPool) -> &mut Self {
        let mut new = self;
        new.pg_pool = Some(pg_pool);
        new
    }

    pub fn user_id(&mut self, user_id: String) -> &mut Self {
        let mut new = self;
        new.user_id = Some(user_id);
        new
    }

    pub fn username(&mut self, username: String) -> &mut Self {
        let mut new = self;
        new.username = Some(username);
        new
    }

    pub fn system(&mut self, system: String) -> &mut Self {
        let mut new = self;
        new.system = Some(system);
        new
    }

    pub fn location(&mut self, location: String) -> &mut Self {
        let mut new = self;
        new.location = Some(location);
        new
    }

    pub fn assignment(&mut self, assignment: ShipAssignment) -> &mut Self {
        let mut new = self;
        new.assignment = Some(assignment);
        new
    }

    pub fn ship(&mut self, ship: shared::Ship) -> &mut Self {
        let mut new = self;
        new.ship = Some(ship);
        new
    }

    pub fn build(&self) -> anyhow::Result<ShipMachine> {
        let mut trader_machine = None;
        let mut scout_machine = None;
        let mut system_change_machine = None;

        let client = self.client.as_ref().expect("client is required");
        let pg_pool = self.pg_pool.as_ref().expect("pg_pool is required");
        let ship = self.ship.as_ref().expect("ship is required");
        let user_id = self.user_id.as_ref().expect("user_id is required");
        let username = self.username.as_ref().expect("username is required");
        let system = self.system.as_ref().expect("system is required");

        match self.assignment.as_ref().expect("a ship assignment is required when building a ship") {
            ShipAssignment::Trader => {
                trader_machine = Some(Trader::new(
                    client.clone(),
                    pg_pool.clone(),
                    user_id.clone(),
                    username.clone(),
                    system.clone(),
                    ship.clone(),
                ));
            }
            ShipAssignment::Scout => {
                scout_machine = Some(Scout::new(
                    client.clone(),
                    pg_pool.clone(),
                    user_id.clone(),
                    username.clone(),
                    system.clone(),
                    self.location.as_ref().expect("location is required when building a scout").clone(),
                    ship.clone(),
                ));
            }
            ShipAssignment::SystemChange => {
                system_change_machine = Some(SystemChange::new(
                    client.clone(),
                    pg_pool.clone(),
                    user_id.clone(),
                    username.clone(),
                    system.clone(),
                    ship.clone(),
                ));
            }
        }

        Ok(
            ShipMachine {
                client: client.clone(),
                pg_pool: pg_pool.clone(),
                username: username.clone(),
                user_id: user_id.clone(),
                ship_id: ship.id.clone(),
                system: system.clone(),
                location: self.location.as_ref().unwrap_or(&"".to_string()).clone(),
                trader_machine,
                scout_machine,
                system_change_machine,
            }
        )
    }
}
