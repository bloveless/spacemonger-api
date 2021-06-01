use anyhow::anyhow;
use spacetraders::client::{self, HttpClient, Client};
use spacetraders::errors::SpaceTradersClientError;
use sqlx::PgPool;
use spacetraders::responses;
use crate::db;
use spacetraders::shared::Good;
use crate::db::DbRoute;
use regex::Regex;

pub async fn is_api_in_maintenance_mode(http_client: HttpClient) -> bool {
    let game_status = client::get_game_status(http_client.clone()).await;

    if game_status.is_err() {
        let game_status_error = game_status.err().unwrap();

        return matches!(game_status_error, SpaceTradersClientError::ServiceUnavailable)
    }

    false
}

pub async fn create_flight_plan(client: Client, pg_pool: PgPool, user_id: &str, ship_id: &str, destination: &str) -> anyhow::Result<responses::FlightPlan> {
    let flight_plan = client.create_flight_plan(ship_id.to_string(), destination.to_string()).await?;

    db::persist_flight_plan(pg_pool, user_id, ship_id, &flight_plan).await?;

    Ok(flight_plan)
}

pub async fn create_purchase_order(client: Client, pg_pool: PgPool, user_id: &str, ship_id: &str, good: Good, quantity: i32) -> anyhow::Result<responses::PurchaseOrder> {
    if quantity > 0 {
        let purchase_order = client.create_purchase_order(ship_id.to_string(), good, quantity).await?;

        db::persist_transaction(pg_pool.clone(), "purchase", user_id, &purchase_order).await?;

        Ok(purchase_order)
    } else {
        Err(anyhow!("Refusing to try and create a purchase order with zero quantity"))
    }
}

pub async fn create_sell_order(client: Client, pg_pool: PgPool, user_id: &str, ship_id: &str, good: Good, quantity: i32) -> anyhow::Result<responses::PurchaseOrder> {
    if quantity > 0 {
        let sell_order = client.create_sell_order(ship_id.to_string(), good, quantity).await?;

        db::persist_transaction(pg_pool.clone(), "sell", user_id, &sell_order).await?;

        Ok(sell_order)
    } else {
        Err(anyhow!("Refusing to try and create a sell order with zero quantity"))
    }
}

pub async fn get_additional_fuel_required_for_trip(pg_pool: PgPool, http_client: Client, ship_id: &str, ship_type: &str, current_fuel: i32, origin: &str, destination: &str) -> anyhow::Result<i32> {
    let db_fuel_required = db::get_fuel_required(pg_pool.clone(), origin, destination, ship_type).await?;

    // if we already have already made this flight before then we know exactly how much fuel is required
    if let Some(db_fuel_required) = db_fuel_required {
        return Ok(db_fuel_required - current_fuel)
    }

    match http_client.create_flight_plan(ship_id.to_string(), destination.to_string()).await {
        Ok(_) => {
            log::error!("This request should have failed. This method should only be called with ships that have zero fuel");

            Err(anyhow!("Request shouldn't have succeeded. Ship is now in motion"))
        },
        Err(e) => match e {
            SpaceTradersClientError::ApiError(error_message) => {
                let re = Regex::new(r"You require (\d+?) more FUEL").unwrap();

                let mut additional_fuel_required = 0;
                for cap in re.captures_iter(&error_message.error.message) {
                    additional_fuel_required = cap[1].parse::<i32>().unwrap();
                }

                Ok(additional_fuel_required)
            },
            e => {
                Err(anyhow!("Expected a \"not enough fuel\" error, but got {} instead", e))
            },
        }
    }

    // TODO: Rather than try and guess the formula lets attempt to create the flight plan
    //       (after ensuring that the ship has no fuel) and then parse the response to get the
    //       actual fuel requirements. After the flight plan is written to the db then we will
    //       just use that value instead of this process again.

    // let distance_between = db::get_distance_between_locations(pg_pool, origin, destination).await?;

    // https://discord.com/channels/792864705139048469/792864705139048472/839919413742272572
    // floor((cargo - fuelRequired) / volume) * (sell - buy) / time
    // https://discord.com/channels/792864705139048469/792864705139048472/836090525307371541
    // time = distance * (2 / speed) + 60
    // let planet_penalty = if distance_between.origin_location_type == "Planet" { 2.0 } else { 0.0 };
    // let fuel_required: f64 = (distance_between.distance.round() / 4.0) + planet_penalty + 1.0;

    // let ship_fuel_penalty = match ship_type {
    //     "GR-MK-II" => 1.0,
    //     "GR-MK-III" => 2.0,
    //     _ => 0.0,
    // };

    // Ok((fuel_required + ship_fuel_penalty).ceil() as i32)
}

pub async fn get_routes_for_ship(pg_pool: PgPool, location_symbol: &str, ship_speed: i32) -> anyhow::Result<Vec<DbRoute>> {
    // TODO: Getting the best route only from the location that the ship currently is in locks
    //       the ship into trade loops. It might be better to search the entire system for the
    //       best route and then find the best trade to that location before beginning a trade
    //       route. That way we move around the system a little more
    match db::get_routes_from_location(pg_pool.clone(), location_symbol, ship_speed).await {
        Ok(routes) => return Ok(routes),
        Err(e) => panic!("Unable to get routes for ship {:?}", e),
    };
}
