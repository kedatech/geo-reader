use actix_web::web;

mod handlers;
use handlers::{find_places, get_nearby_routes_endpoint};

pub fn config(cfg: &mut web::ServiceConfig) {

    print!("Configuring routes...");
    
    cfg.service(
        web::scope("/api")
            .route("/places", web::get().to(find_places))
            .route("/nearby_routes", web::get().to(get_nearby_routes_endpoint)) 
    );
}
