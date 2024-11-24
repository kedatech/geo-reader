use actix_web::web;

mod handlers;
use handlers::{
    find_bus_route, find_places, get_nearby_routes_endpoint, get_routes_by_number_endpoint, plan_routes
};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/places", web::get().to(find_places))
            .route("/nearby_routes", web::get().to(get_nearby_routes_endpoint))
            .route("/by_number", web::get().to(get_routes_by_number_endpoint))
            .route("/bus_route", web::get().to(find_bus_route))
            .route("/plan_routes", web::get().to(plan_routes)),
    );
}