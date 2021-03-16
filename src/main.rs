mod funcs;
mod db;

use spacetraders::{game::Game, shared};
use tokio::time::Duration;
use std::cmp::min;
use std::convert::TryFrom;
use std::env;
use dotenv::dotenv;
use spacetraders::shared::LoanType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let args: Vec<String> = env::args().collect();
    let postgres_host = env::var("POSTGRES_HOST").unwrap();
    let postgres_username = env::var("POSTGRES_USERNAME").unwrap();
    let postgres_password = env::var("POSTGRES_PASSWORD").unwrap();
    let postgres_database = env::var("POSTGRES_DATABASE").unwrap();

    let mut client = db::get_client(postgres_host, postgres_username, postgres_password, postgres_database).await?;

    db::setup_tables(&mut client).await?;

    // 1. get user
    let current_user = db::get_current_user(&mut client).await?;
    println!("Current user: {:?}", &current_user);

    let game = Game::new(
        current_user.username,
        current_user.token,
    );

    // 2. if the user doesn't have enough credits take out a startup loan
    let mut current_user_info = game.get_user_info().await?;
    println!("Current user info {:?}", current_user_info);
    if current_user_info.user.credits == 0 {
        // assume that if the user has 0 credits that the user needs to take out a loan
        current_user_info = game.request_new_loan(LoanType::Startup).await?;
    }

    // 3. if the user doesn't have any ships then buy the fastest one that the user can afford
    if current_user_info.user.ships.len() == 0 {
        let available_ships = game.get_ships_for_sale().await?;
        let mut fastest_ship = None;
        let mut fastest_ship_speed = 0;
        let mut fastest_ship_location = "".to_string();
        let mut fastest_ship_price = 0;

        for available_ship in &available_ships.ships {
            for purchase_location in &available_ship.purchase_locations {
                if available_ship.speed > fastest_ship_speed && current_user_info.user.credits > purchase_location.price {
                    fastest_ship_speed = available_ship.speed;
                    fastest_ship = Some(available_ship);
                    fastest_ship_location = purchase_location.location.to_owned();
                    fastest_ship_price = purchase_location.price;
                }
            }
        }

        if let Some(ship) = fastest_ship {
            println!("Buying {} for {} at location {}", ship.ship_type.to_owned(), fastest_ship_price, fastest_ship_location);
            current_user_info = game.purchase_ship(fastest_ship_location, ship.ship_type.to_owned()).await?;
        } else {
            panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
        }
    }

    // Now that the user account is setup lets begin scanning the system
    // there should be some criteria to skip this scan but for now we can
    // just have the ship scan at the beginning of every run.

    // potentially we only need to make a full system scan if there are any locations that we don't have any market data for



    // TODO: deal with the loan

    if args[1] == "scan-system" {
        println!("-------------------------------------------------------------------------------");
        println!("BEGINNING SYSTEM SCAN ---------------------------------------------------------");
        println!("-------------------------------------------------------------------------------");

        let fastest_ship = funcs::get_fastest_ship(&game).await?.unwrap();
        println!("Fastest Ship: {:?}", &fastest_ship);

        funcs::scan_system(&game, fastest_ship.clone(), &mut client).await?;
    }

    if args[1] == "run-trade-route" {
        println!("-------------------------------------------------------------------------------");
        println!("RUNNING TRADE ROUTE -----------------------------------------------------------");
        println!("-------------------------------------------------------------------------------");

        let mut fastest_ship = funcs::get_fastest_ship(&game).await?.unwrap();
        let pickup_location = "OE-A1";
        let sell_location = "OE-A1-M1";
        let good_symbol = shared::Good::Research;

        loop {
            // get ship to 20 fuel
            // let ship_info = game.get_your_ships().await?;
            // let mut ship_info = ship_info.ships.iter().find(|s| s.id == fastest_ship.id).unwrap().to_owned();

            let ship_fuel = fastest_ship.cargo.iter().fold(0, |sum, cargo| if cargo.good == shared::Good::Fuel { sum + cargo.quantity } else { sum });
            println!("Current ship fuel: {}", ship_fuel);

            if ship_fuel < 20 {
                println!("Purchasing {} fuel", 20 - ship_fuel);
                game.create_purchase_order(fastest_ship.clone(), shared::Good::Fuel, 20 - ship_fuel).await?;
                fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
            }

            // go to pickup location
            println!("Going to pickup location {}", pickup_location);
            let flight_plan = funcs::create_flight_plan(&game, &mut client, &fastest_ship, pickup_location.to_string()).await?;
            fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
            println!("Flight plan: {:?}", &flight_plan);
            println!("Waiting {} seconds", flight_plan.flight_plan.time_remaining_in_seconds + 2);

            tokio::time::sleep(Duration::from_secs(u64::try_from(flight_plan.flight_plan.time_remaining_in_seconds + 2).unwrap())).await;

            // get ship to 20 fuel
            let ship_fuel = fastest_ship.cargo.iter().fold(0, |sum, cargo| if cargo.good == shared::Good::Fuel { sum + cargo.quantity } else { sum });
            println!("Current ship fuel: {}", ship_fuel);

            if ship_fuel < 20 {
                println!("Purchasing {} fuel", 20 - ship_fuel);
                game.create_purchase_order(fastest_ship.clone(), shared::Good::Fuel, 20 - ship_fuel).await?;
                fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
            }

            // buy as much good as possible
            let current_cargo = fastest_ship.cargo.iter().fold(0, |sum, cargo| sum + cargo.quantity);
            let available_room = fastest_ship.max_cargo - current_cargo;
            println!("Current cargo: {}, available room: {}", current_cargo, available_room);

            let user_credits = game.get_user_info().await?.user.credits;
            println!("Current user credits: {}", user_credits);
            let marketplace_info = game.get_location_marketplace(pickup_location.to_string()).await?;

            let good_cost = marketplace_info.planet.marketplace.iter().find(|d| d.symbol == good_symbol).unwrap().price_per_unit;
            println!("Good cost: {}", good_cost);
            let max_good_to_buy = user_credits / good_cost;
            println!("Max good to buy: {}", max_good_to_buy);

            let actual_good_to_buy = min(max_good_to_buy, available_room);
            println!("Actual good to buy: {}", actual_good_to_buy);

            if actual_good_to_buy > 0 {
                let purchase_response = game.create_purchase_order(fastest_ship.clone(), good_symbol.clone(), actual_good_to_buy).await?;
                fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
                println!("Good purchase response: {:?}", purchase_response);
            } else {
                println!("Not purchasing anything...");
            }

            // go to OE-A1-M1
            println!("Going to sell location: {}", sell_location);
            let flight_plan = funcs::create_flight_plan(&game, &mut client, &fastest_ship, sell_location.to_string()).await?;
            fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
            println!("Flight plan: {:?}", &flight_plan);
            println!("Waiting {} seconds", flight_plan.flight_plan.time_remaining_in_seconds + 2);

            tokio::time::sleep(Duration::from_secs(u64::try_from(flight_plan.flight_plan.time_remaining_in_seconds + 2).unwrap())).await;

            // sell all good
            let current_good_in_cargo = fastest_ship.cargo.iter().fold(0, |sum, cargo| if cargo.good == good_symbol { sum + cargo.quantity } else { sum });

            if current_good_in_cargo > 0 {
                println!("Selling {} good cargo", current_good_in_cargo);
                let sell_response = game.create_sell_order(fastest_ship.id.to_string(), good_symbol.clone(), current_good_in_cargo).await?;
                fastest_ship = funcs::get_ship(&game, fastest_ship.id).await?.unwrap();
                println!("Sell response: {:?}", sell_response);
            } else {
                println!("Didn't find any good in cargo to sell...");
            }
        }
    }

    Ok(())
}
