use actix_web::{web, HttpResponse, Responder};
use crate::db::connect_to_db;
use crate::plan_routes::{
    index::{RoutePlanner, PlanningError},
    _structs::{RoutePlan, TransferType},
    data_loader::DataLoader,
    geo_validation::GeoValidator,
    spatial_search::SpatialSearch,
};
use crate::queries::{
    find_places::find_places_by_name,
    nearby_route::get_nearby_routes,
    find_by_number::get_routes_by_number,
    find_route::find_route
};
use geo_types::Point;
use log::{info, error, debug};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use std::path::PathBuf;
use std::fs::create_dir_all;
use crate::plan_routes::index::PlanningConfig;

// ==================== Estructuras de Datos ====================

#[derive(Deserialize)]
pub struct PlanRoutesQuery {
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
}

#[derive(Serialize, Deserialize, Debug)]  // Agregamos Debug para logging
pub struct PlanningResponse {
    success: bool,
    message: Option<String>,
    routes: Option<Vec<RoutePlanResponse>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RoutePlanResponse {
    segments: Vec<RouteSegmentResponse>,
    total_distance: f64,
    transfers_count: i32,
    is_interdepartmental: bool,
    estimated_time: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)] 
pub struct RouteSegmentResponse {
    route_code: String,
    route_name: String,
    transfer_type: String,
    transfer_point: TransferPointResponse,
    segment_distance: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransferPointResponse {
    latitude: f64,
    longitude: f64,
    stop_name: Option<String>,
    distance: f64,
}

// ==================== Planificador Global ====================

lazy_static! {
    static ref ROUTE_PLANNER: Arc<Mutex<Option<RoutePlanner>>> = Arc::new(Mutex::new(None));
}

// ==================== Funciones de Utilidad ====================

fn is_valid_coordinates(lat: f64, lng: f64) -> bool {
    // Límites aproximados de El Salvador
    const MIN_LAT: f64 = 13.0;
    const MAX_LAT: f64 = 14.5;
    const MIN_LNG: f64 = -90.2;
    const MAX_LNG: f64 = -87.5;

    lat >= MIN_LAT && lat <= MAX_LAT && lng >= MIN_LNG && lng <= MAX_LNG
}

fn estimate_travel_time(plan: &RoutePlan) -> i32 {
    let base_time = (plan.total_distance * 3600.0 / 30.0) as i32;  // tiempo en segundos
    let transfer_time = plan.transfers_count * 5 * 60;  // tiempo en segundos
    let total_seconds = if plan.is_interdepartmental {
        (base_time as f64 * 1.2) as i32 + transfer_time
    } else {
        base_time + transfer_time
    };
    (total_seconds + 59) / 60
}
fn convert_plan_to_response(plan: RoutePlan) -> RoutePlanResponse {
    let plan_clone = plan.clone();
    let segments = plan.routes.into_iter()
        .map(|segment| RouteSegmentResponse {
            route_code: segment.route.codigo_de.unwrap_or_default(),
            route_name: segment.route.nombre_de.unwrap_or_default(),
            transfer_type: match segment.transfer_type {
                TransferType::Direct => "Directo".to_string(),
                TransferType::Near => "Cercano".to_string(),
                TransferType::Proximate => "Próximo".to_string(),
            },
            transfer_point: TransferPointResponse {
                latitude: segment.transfer_point.location.y(),
                longitude: segment.transfer_point.location.x(), 
                stop_name: segment.transfer_point.bus_stop
                    .and_then(|stop| stop.nam.clone()),
                distance: segment.transfer_point.distance_to_route,
            },
            segment_distance: segment.segment_distance,
        })
        .collect();

    RoutePlanResponse {
        segments,
        total_distance: plan_clone.total_distance,
        transfers_count: plan_clone.transfers_count,
        is_interdepartmental: plan_clone.is_interdepartmental,
        estimated_time: estimate_travel_time(&plan_clone),
    }
}

/// Inicializa el planificador de rutas cargando los datos necesarios
pub async fn initialize_planner() -> Result<(), Box<dyn std::error::Error>> {
    info!("Initializing route planner...");
    
    // Configurar directorios
    let data_dir = PathBuf::from("./data");
    let cache_dir = PathBuf::from("./cache");
    
    // Crear directorio de cache si no existe
    if !cache_dir.exists() {
        info!("Creating cache directory...");
        create_dir_all(&cache_dir)?;
    }

    // Cargar datos
    let mut data_loader = DataLoader::new(&data_dir);
    data_loader.load_all()?;
    info!("Data loaded successfully");
    
    // Inicializar componentes
    let validator = GeoValidator::new(data_loader.departments());
    
    info!("Initializing spatial search with cache...");
    let search = SpatialSearch::new(
        data_loader.routes().features.clone(),
        data_loader.bus_stops().features.iter()
            .map(|feature| feature.properties.clone())
            .collect(),
        Some(cache_dir.clone())
    );
    
    // Configurar el planificador
    let config = PlanningConfig {
        max_route_distance: 0.05,     // ~5km en grados
        max_transfer_distance: 0.01,   // ~1km en grados
        max_transfers: 10,
        results_limit: 3,
    };
    
    info!("Creating route planner...");
    let planner = RoutePlanner::new(validator, search, Some(config));
    
    // Actualizar la instancia global
    let mut planner_guard = ROUTE_PLANNER.lock().await;
    *planner_guard = Some(planner);
    
    info!("Route planner initialized successfully");
    Ok(())
}


// ==================== Handler Principal ====================

pub async fn plan_routes(query: web::Query<PlanRoutesQuery>) -> impl Responder {
    info!("Planning routes from ({}, {}) to ({}, {})", 
          query.start_lat, query.start_lng, query.end_lat, query.end_lng);

    if !is_valid_coordinates(query.start_lat, query.start_lng) 
        || !is_valid_coordinates(query.end_lat, query.end_lng) {
        error!("Invalid coordinates provided");
        return HttpResponse::BadRequest().json(PlanningResponse {
            success: false,
            message: Some("Coordinates must be within El Salvador bounds".into()),
            routes: None,
        });
    }

    let planner_guard = ROUTE_PLANNER.lock().await;
    let planner = match planner_guard.as_ref() {
        Some(p) => p,
        None => {
            error!("Route planner not initialized");
            return HttpResponse::InternalServerError().json(PlanningResponse {
                success: false,
                message: Some("Route planning system not initialized".into()),
                routes: None,
            });
        }
    };

    let origin = Point::new(query.start_lng, query.start_lat);
    let destination = Point::new(query.end_lng, query.end_lat);

    match planner.plan_route(origin, destination) {
        Ok(plans) => {
            let response_plans: Vec<RoutePlanResponse> = plans.into_iter()
                .map(convert_plan_to_response)
                .collect();

            debug!("Found {} possible route plans", response_plans.len());

            if response_plans.is_empty() {
                HttpResponse::NotFound().json(PlanningResponse {
                    success: false,
                    message: Some("No valid routes found between the specified points".into()),
                    routes: None,
                })
            } else {
                HttpResponse::Ok().json(PlanningResponse {
                    success: true,
                    message: None,
                    routes: Some(response_plans),
                })
            }
        }
        Err(e) => {
            error!("Error planning route: {:?}", e);
            let error_message = e.to_string();

            HttpResponse::InternalServerError().json(PlanningResponse {
                success: false,
                message: Some(error_message),
                routes: None,
            })
        }
    }
}

// ! Routes

#[derive(Deserialize)]
pub struct PlaceQuery {
    name: String,
}

// ! FIND PLACES
pub async fn find_places(query: web::Query<PlaceQuery>) -> impl Responder {
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    match find_places_by_name(&query.name, &db_client).await {
        Ok(places) => HttpResponse::Ok().json(places),
        Err(e) => {
            error!("Error finding places: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding places: {}", e))
        }
    }
}

#[derive(Deserialize)]
pub struct NearbyRoutesQuery {
    latitude: f64,
    longitude: f64,
    max_distance: f64,
}

// ! GET NEARBY ROUTES
pub async fn get_nearby_routes_endpoint(query: web::Query<NearbyRoutesQuery>) -> impl Responder {
    info!("Finding nearby routes...");
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    info!("Connected to database...");

    match get_nearby_routes(
        query.latitude,
        query.longitude,
        query.max_distance,
        &db_client,
    )
    .await
    {
        Ok(routes) => HttpResponse::Ok().json(routes),
        Err(e) => {
            error!("Error finding nearby routes: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding nearby routes: {}", e))
        }
    }
}

#[derive(Deserialize)]
pub struct RouteByNumberQuery {
    number_route: String,
}

// ! GET ROUTES BY NUMBER
pub async fn get_routes_by_number_endpoint(
    query: web::Query<RouteByNumberQuery>,
) -> impl Responder {
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    match get_routes_by_number(query.number_route.clone(), &db_client).await {
        Ok(routes) => HttpResponse::Ok().json(routes),
        Err(e) => {
            error!("Error fetching routes by number: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Error fetching routes by number: {}", e))
        }
    }
}

// ! FIND BUS ROUTE
#[derive(Deserialize)]
pub struct RouteQuery {
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
}

pub async fn find_bus_route(query: web::Query<RouteQuery>) -> impl Responder {
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    match find_route(
        query.start_lat,
        query.start_lng,
        query.end_lat,
        query.end_lng,
        &db_client,
    )
    .await
    {
        Ok(routes) => HttpResponse::Ok().json(routes),
        Err(e) => {
            error!("Error finding route: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding route: {}", e))
        }
    }
}


// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, web, http::StatusCode};

    #[actix_web::test]
    async fn test_plan_routes_invalid_coordinates() {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::get().to(plan_routes)))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!("/?start_lat=0&start_lng=0&end_lat=0&end_lng=0"))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: PlanningResponse = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body.message.is_some());
        assert!(body.routes.is_none());
    }

    #[actix_web::test]
    async fn test_plan_routes_valid_coordinates() {
        let app = test::init_service(
            App::new().service(web::resource("/").route(web::get().to(plan_routes)))
        ).await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/?start_lat=13.6929&start_lng=-89.2182&end_lat=13.7084&end_lng=-89.1821"
            ))
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR); // El planificador no está inicializado

        let body: PlanningResponse = test::read_body_json(resp).await;
        assert!(!body.success);
        assert!(body.message.is_some());
        assert!(body.routes.is_none());
    }

    #[actix_web::test]
    async fn test_coordinate_validation() {
        assert!(is_valid_coordinates(13.6929, -89.2182)); // San Salvador
        assert!(!is_valid_coordinates(0.0, 0.0));         // Fuera de El Salvador
        assert!(!is_valid_coordinates(15.0, -89.0));      // Fuera de El Salvador
    }
}