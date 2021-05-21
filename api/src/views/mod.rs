mod index;
mod users;
mod market_data;
mod locations;
mod systems;

use actix_web::web;

// function that will be called on new Application to configure views for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    // index
    cfg.service(index::index);

    // users
    cfg.service(users::users);
    cfg.service(users::user_stats);

    // market data
    cfg.service(market_data::latest);

    // systems
    cfg.service(systems::info);
    cfg.service(systems::routes);
    cfg.service(systems::goods);

    // locations
    cfg.service(locations::goods);
    cfg.service(locations::market_data);
    cfg.service(locations::goods_market_data);
    cfg.service(locations::routes);
}
