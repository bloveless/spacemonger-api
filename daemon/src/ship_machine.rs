use spacetraders::client::Client;
use sqlx::PgPool;
use spacetraders::shared::Good;
use chrono::{DateTime, Utc, Duration};
use crate::{db, funcs};
use crate::db::DbRoute;
use std::cmp::max;

#[derive(Debug, Clone)]
pub enum TickResult {
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
                ShipAssignment::Scout { system_symbol: _, location_symbol } => location_symbol,
            },
            state: ShipState::CheckIfMoving,
            arrival_time: Utc::now(),
            next_harvest_time: Utc::now(),
            route: None,
        }
    }

    pub async fn tick(&mut self) -> Result<Option<TickResult>, Box<dyn std::error::Error>> {
        match self.state {
            ShipState::CheckIfMoving => {
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                if ship.location == None {
                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(self.pg_pool.clone(), &self.ship_id)
                        .await.expect("Unable to find flight plan for a ship that is in motion").unwrap();

                    println!("{} -- Ship is moving to {}. Waiting for arrival", self.username, flight_plan.destination);
                    self.arrival_time = flight_plan.arrives_at;
                    self.state = ShipState::WaitForArrival;
                } else {
                    self.state = ShipState::CheckForCorrectLocation;
                }
            }
            ShipState::CheckForCorrectLocation => {
                let ships = self.client.get_your_ships().await?;
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
                            println!("{} -- Ship assigned to harvest market data from {} is at the correct location. Begin harvesting", self.username, location_symbol);
                            self.state = ShipState::HarvestMarketData;
                        } else {
                            println!("{} -- Ship destined to {} was as the wrong location. Moving", self.username, self.destination);
                            self.state = ShipState::MoveToLocation;
                        }
                    }
                }
            }
            ShipState::WaitForArrival => {
                // We have arrived
                if Utc::now().ge(&self.arrival_time) {
                    println!("{} -- Ship traveling to {} has arrived", self.username, self.destination);
                    self.state = ShipState::CheckForCorrectLocation;
                }
            }
            ShipState::MoveToLocation => {
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                let fuel_required = funcs::get_fuel_required_for_trip(self.pg_pool.clone(), &ship.location.unwrap(), &self.destination, &ship.ship_type).await?;
                let current_fuel = ship.cargo.into_iter()
                    .filter(|c| c.good == Good::Fuel)
                    .fold(0, |acc, c| acc + c.quantity);
                let fuel_required = fuel_required.ceil() as i32 - current_fuel;

                let mut new_user_credits = 0;
                if current_fuel < fuel_required {
                    println!("{} -- Ship destined to {} is filling up with {} fuel", self.username, self.destination, fuel_required);
                    let purchase_order = funcs::create_purchase_order(
                        self.client.clone(),
                        self.pg_pool.clone(),
                        &self.ship_id,
                        Good::Fuel,
                        fuel_required - current_fuel,
                    ).await?;

                    new_user_credits = purchase_order.credits;
                }

                println!("{} -- Ship destined to {} is creating a flight plan", self.username, self.destination);
                let flight_plan = funcs::create_flight_plan(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &self.user_id,
                    &self.ship_id,
                    &self.destination,
                ).await?;

                println!("{} -- Ship destined to {} is scheduled for arrival at {}", self.username, self.destination, flight_plan.flight_plan.arrives_at);
                self.arrival_time = flight_plan.flight_plan.arrives_at;
                self.state = ShipState::WaitForArrival;

                if new_user_credits > 0 {
                    return Ok(Some(TickResult::UpdateCredits(new_user_credits)));
                }
            }
            ShipState::HarvestMarketData => match self.assignment.clone() {
                ShipAssignment::Trader => panic!("ShipState::HarvestMarketData -- Traders do not harvest market data... yet"),
                ShipAssignment::Scout { system_symbol: _, location_symbol } => {
                    let marketplace_data = self.client.get_location_marketplace(&self.destination).await?;

                    println!("{} -- Ship assigned to {} has received marketplace data", self.username, location_symbol);

                    for datum in marketplace_data.location.marketplace {
                        db::persist_market_data(
                            self.pg_pool.clone(),
                            &location_symbol,
                            &datum,
                        )
                            .await.expect("Unable to save market data");
                    }

                    self.state = ShipState::Wait;
                    self.next_harvest_time = Utc::now() + Duration::minutes(3);
                    println!("{} -- Ship assigned to {} will check market data again at {}", self.username, location_symbol, self.next_harvest_time);
                }
            },
            ShipState::Wait => match self.assignment.clone() {
                ShipAssignment::Trader => panic!("ShipState::Wait -- Traders to not harvest market data... yet"),
                ShipAssignment::Scout { system_symbol: _, location_symbol } => {
                    if Utc::now().ge(&self.next_harvest_time) {
                        println!("{} -- Ship assigned to {} to harvest market data has finished waiting for next harvest time", self.username, location_symbol);
                        self.state = ShipState::HarvestMarketData;
                    }
                }
            },
            ShipState::SellAllCargo => {
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                let mut new_user_credits = 0;
                for cargo in ship.cargo {
                    println!("{} -- Selling {} goods {} at {}", self.username, cargo.quantity, cargo.good, ship.location.clone().unwrap());
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");
                    let sell_order = funcs::create_sell_order(self.client.clone(), self.pg_pool.clone(), &ship.id, cargo.good, cargo.quantity).await?;

                    new_user_credits = sell_order.credits;
                }

                self.state = ShipState::PickBestTrade;

                if new_user_credits > 0 {
                    return Ok(Some(TickResult::UpdateCredits(new_user_credits)));
                }
            }
            ShipState::PickBestTrade => {
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                let origin = &ship.location.expect("Ship must be docked in order to get trade routes for it");

                let routes = funcs::get_routes_for_ship(
                    self.pg_pool.clone(),
                    origin,
                ).await?;

                for route in routes {
                    if route.sell_location_symbol != "OE-XV-91-2" && route.purchase_quantity > 500 {
                        println!("{} -- Trading {} from {} to {} (purchase quantity {}, sell quantity {})", self.username, route.good, route.purchase_location_symbol, route.sell_location_symbol, route.purchase_quantity, route.sell_quantity);

                        self.route = Some(route.to_owned());
                        self.state = ShipState::PurchaseMaxGoodForTrading;

                        return Ok(None);
                    }
                }

                println!("{} -- Found no available routes from {}. Will check again later to see if there are any routes available", self.username, origin);
            }
            ShipState::PurchaseMaxGoodForTrading => {
                let ships = self.client.get_your_ships().await?;
                let ship = ships.ships
                    .into_iter()
                    .find(|s| s.id == self.ship_id)
                    .expect("Tried to control a ship which doesn't belong to this user");

                let route = self.route.clone().unwrap();

                let fuel_required = funcs::get_fuel_required_for_trip(
                    self.pg_pool.clone(),
                    &ship.location.unwrap(),
                    &route.sell_location_symbol,
                    &ship.ship_type,
                ).await?;

                let current_fuel = ship.cargo.into_iter()
                    .filter(|c| c.good == Good::Fuel)
                    .fold(0, |acc, c| acc + c.quantity);

                println!("{} -- Ship space available {}, fuel required {}, current fuel {}", self.username, ship.space_available, fuel_required.ceil() as i32, current_fuel);

                let fuel_required = max(fuel_required.ceil() as i32 - current_fuel, 0);

                let room_available_for_trading = ship.space_available - fuel_required;

                let volume_per_unit = (db::get_good_volume(self.pg_pool.clone(), route.good).await)
                    .unwrap_or(1);

                println!(
                    "{} -- Purchasing {} {} for trading (volume per unit {}). Purchase price at {} is {}. Sell price at {} is {}",
                    self.username,
                    room_available_for_trading / volume_per_unit,
                    route.good,
                    volume_per_unit,
                    route.purchase_location_symbol,
                    route.purchase_price_per_unit,
                    route.sell_location_symbol,
                    route.sell_price_per_unit
                );

                let purchase_order = funcs::create_purchase_order(
                    self.client.clone(),
                    self.pg_pool.clone(),
                    &ship.id,
                    route.good,
                    room_available_for_trading / volume_per_unit,
                ).await?;

                self.destination = route.sell_location_symbol;
                self.state = ShipState::MoveToLocation;

                return Ok(Some(TickResult::UpdateCredits(purchase_order.credits)));
            }
        }

        Ok(None)
    }
}

// trader machine
// move to purchase location <-- start here
// make purchase based on best route --> move to sell location
// move to sell location --> wait
// wait --> sell good
// sell good --> make purchase basted on best route

//     loop {
//         // main user will find the best trade from their current location
//         // we are only working with one ship so we'll just user the main users first ship
//         let routes = user.get_routes_for_ship(0).await?;
//         let best_route = routes.get(0);
//
//         // TODO: It is possible that the best_route doesn't have enough quantity to make a full
//         //       trade so maybe pick the second best route if there isn't enough quantity at the
//         //       from location
//
//         if let Some(best_route) = best_route {
//             println!("Good to trade {:?}", best_route.good);
//             println!("Location to trade at {}", best_route.sell_location_symbol);
//
//             // fill ship with enough fuel to get to the other location
//             let fuel_required = best_route.fuel_required.ceil() as i32;
//
//             // We don't have to move the ship to the purchase location because we got the routes
//             // specifically for the location that this ship is already in
//
//             println!("Route requires {} fuel", fuel_required);
//
//             user.ensure_ship_has_enough_fuel(0, fuel_required).await?;
//
//             // make purchase
//             user.fill_ship_with_max_good(0, best_route.good).await?;
//
//             user.send_ship_to_location(0, best_route.sell_location_symbol.clone()).await?;
//
//             // sell and repeat
//             user.sell_max_good(0, best_route.good).await?;
//
//             // TODO: Keep track of users credits through time
//             //       Keep track of users flight plans, purchases, sells, and routes (cdv)
//
//             // TODO: Automated upgrades. When credits hits X amount purchase a new ship
//             //       When credits hits X amount upgrade a type of ship to another type of ship
//             //       This will also require us to actually manage more that one ship at a time
//         }
//     }
//
//     futures::future::join_all(ship_handles).await;
// }));
