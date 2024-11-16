use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::db::connect_to_db;
use crate::queries::find_places::find_places_by_name;
use crate::queries::nearby_route::get_nearby_routes; // Importa `get_nearby_routes` desde el módulo de consultas

#[derive(Deserialize)]
pub struct PlaceQuery {
    name: String,
}

pub async fn find_places(query: web::Query<PlaceQuery>) -> impl Responder {
    // Conectar a la base de datos
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Database connection error: {}", e)),
    };

    // Ejecutar la consulta de lugares por nombre
    match find_places_by_name(&query.name, &db_client).await {
        Ok(places) => HttpResponse::Ok().json(places),
        Err(e) => {
            eprintln!("Error finding places: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding places: {}", e))
        },
    }
}

#[derive(Deserialize)]
pub struct NearbyRoutesQuery {
    latitude: f64,
    longitude: f64,
    max_distance: f64,
}

// Renombramos la función a `get_nearby_routes_endpoint` para evitar conflictos
pub async fn get_nearby_routes_endpoint(query: web::Query<NearbyRoutesQuery>) -> impl Responder {
    print!("Finding nearby routes...");
    // Conectar a la base de datos
    let db_client = match connect_to_db().await {
        Ok(client) => client,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Database connection error: {}", e)),
    };

    print!("Connected to database...");

    // Llamar a la función de consulta con los parámetros
    match get_nearby_routes(query.latitude, query.longitude, query.max_distance, &db_client).await {
        Ok(routes) => HttpResponse::Ok().json(routes),
        Err(e) => {
            eprintln!("Error finding nearby routes: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding nearby routes: {}", e))
        },
    }
}
