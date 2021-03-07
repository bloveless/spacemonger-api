use spacetraders::{game::Game, shared::LoanType, shared::Good};
use spacetraders::requests::RequestNewLoanRequest;
use serde_json::json;
use spacetraders::shared::LocationType;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game = Game::new(
        "bloveless".to_owned(),
        "954a9a58-2764-4522-a3c6-876a28e21b02".to_owned()
    );

    println!("## GET_GAME_STATUS ----------------------------------------------------------------");

    match game.get_game_status().await {
        Ok(game_status) => println!("Status: {:?}", game_status),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    println!("## GET_USER_INFO ------------------------------------------------------------------");

    match game.get_user_info().await {
        Ok(user_info) => println!("User info: {:?}", user_info),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    println!("## GET_AVAILABLE_LOANS ------------------------------------------------------------");

    match game.get_available_loans().await {
        Ok(available_loans) => println!("Available Loans: {:?}", available_loans),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    println!("## TAKE_OUT_A_LOAN ----------------------------------------------------------------");

    match game.request_new_loan(LoanType::Startup).await {
        Ok(account_status) => println!("Account status after loan: {:?}", account_status),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    println!("## GET_SHIPS_FOR_SALE -------------------------------------------------------------");

    match game.get_ships_for_sale("MK-I".to_string()).await {
        Ok(ships_for_sale) => println!("Ships for sale: {:?}", ships_for_sale),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    // println!("## PURCHASE_SHIP ------------------------------------------------------------------");
    //
    // match game.purchase_ship("OE-D2".to_string(), "GR-MK-I".to_string()).await {
    //     Ok(user_info) => println!("User info: {:?}", user_info),
    //     Err(e) => println!("CAUGHT ERROR: {}", e),
    // }

    // println!("## CREATE_PURCHASE_ORDER ----------------------------------------------------------");
    //
    // match game.create_purchase_order("cklx92wta502620t889jz5powu0".to_string(), Good::Fuel, 20).await {
    //     Ok(purchase_order) => println!("Purchase Order: {:?}", purchase_order),
    //     Err(e) => println!("CAUGHT ERROR: {}", e),
    // }

    println!("## GET_AVAILABLE_LOCATIONS --------------------------------------------------------");

    match game.get_locations_in_system("OE".to_string(), LocationType::Planet).await {
        Ok(available_locations) => println!("Available locations: {:?}", available_locations),
        Err(e) => println!("CAUGHT ERROR: {}", e),
    }

    // println!("## CREATE_FLIGHT_PLAN -------------------------------------------------------------");
    //
    // match game.create_flight_plan("cklx92wta502620t889jz5powu0".to_string(), "OE-G4".to_string()).await {
    //     Ok(flight_plan) => println!("Flight plan: {:?}", flight_plan),
    //     Err(e) => println!("CAUGHT ERROR: {}", e),
    // }

    println!("## GET_SYSTEMS_INFO ---------------------------------------------------------------");

    match game.get_systems_info().await {
        Ok(systems_info) => println!("Systems info: {:?}", systems_info),
        Err(e) => println!("CAUGHT ERROR: {}", e)
    }

    println!("## GET_YOUR_SHIPS -----------------------------------------------------------------");

    match game.get_your_ships().await {
        Ok(your_ships) => println!("Your ships: {:?}", your_ships),
        Err(e) => println!("CAUGHT ERROR: {}", e)
    }

    println!("## GET_LOCATION_INFO --------------------------------------------------------------");

    match game.get_location_info("OE-BG1".to_string()).await {
        Ok(location_info) => println!("Location info: {:?}", location_info),
        Err(e) => println!("CAUGHT ERROR: {}", e)
    }


    Ok(())
}
