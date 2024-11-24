use crate::db::connect_to_db;
use crate::queries::{
    find_places::find_places_by_name,
    nearby_route::get_nearby_routes,
    find_by_number::get_routes_by_number,
    find_route::find_route,
    plan_route::find_route_plans
};
use actix_web::{web, HttpResponse, Responder};
use log::info;
use serde::Deserialize; // Importa `get_nearby_routes` desde el módulo de consultas

#[derive(Deserialize)]
pub struct PlaceQuery {
    name: String,
}

// ! FIND PLACES
pub async fn find_places(query: web::Query<PlaceQuery>) -> impl Responder {
    // Conectar a la base de datos
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    // Ejecutar la consulta de lugares por nombre
    match find_places_by_name(&query.name, &db_client).await {
        Ok(places) => HttpResponse::Ok().json(places),
        Err(e) => {
            eprintln!("Error finding places: {}", e);
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
    print!("Finding nearby routes...");
    // Conectar a la base de datos
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    print!("Connected to database...");

    // Llamar a la función de consulta con los parámetros
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
            eprintln!("Error finding nearby routes: {}", e);
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
    // Conectar a la base de datos
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    // Ejecutar la consulta para obtener rutas por número
    match get_routes_by_number(query.number_route.clone(), &db_client).await {
        Ok(routes) => HttpResponse::Ok().json(routes),
        Err(e) => {
            eprintln!("Error fetching routes by number: {}", e);
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
            eprintln!("Error finding route: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding route: {}", e))
        }
    }
}




// ! PLAN ROUTES
#[derive(Deserialize)]
pub struct PlanRoutesQuery {
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
}

pub async fn plan_routes(query: web::Query<PlanRoutesQuery>) -> impl Responder {
    info!("Planning routes...");

    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database connection error: {}", e))
        }
    };

    info!("Connected to database...");

    match find_route_plans(
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
            eprintln!("Error planning routes: {}", e);
            HttpResponse::InternalServerError().body(format!("Error planning routes: {}", e))
        }
    }
}
