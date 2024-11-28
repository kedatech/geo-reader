use actix_web::web;

pub mod handlers;
use handlers::{
    find_bus_route, 
    find_places, 
    get_nearby_routes_endpoint, 
    get_routes_by_number_endpoint, 
    plan_routes
};

/// Inicialización del planificador de rutas
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    use handlers::initialize_planner;
    initialize_planner().await?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use actix_web::http::StatusCode;

    #[actix_web::test]
    async fn test_routes_configuration() {
        let app = test::init_service(
            App::new().configure(config)
        ).await;

        // Test plan_routes endpoint
        let req = test::TestRequest::get()
            .uri("/api/plan_routes?start_lat=13.6929&start_lng=-89.2182&end_lat=13.7084&end_lng=-89.1821")
            .to_request();
        let resp = test::call_service(&app, req).await;
        
        // Debería devolver error interno ya que el planificador no está inicializado en tests
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}