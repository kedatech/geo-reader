use actix_web::web;

mod handlers;
use handlers::{find_places, get_nearby_routes_endpoint, get_routes_by_number_endpoint};

pub fn config(cfg: &mut web::ServiceConfig) {
    print!("Configuring routes...");

    cfg.service(
        web::scope("/api")
            .route("/places", web::get().to(find_places))
            .route("/nearby_routes", web::get().to(get_nearby_routes_endpoint))
            .route("/by_number", web::get().to(get_routes_by_number_endpoint)),
    );
}
