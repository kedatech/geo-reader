use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::{
    db::connect_to_db,
    queries::find_places::find_places_by_name,
};

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

    // Llama a la función find_places_by_name y procesa el resultado
    match find_places_by_name(&query.name, &db_client).await {
        Ok(places) => HttpResponse::Ok().json(places),
        Err(e) => {
            eprintln!("Error finding places: {}", e);
            HttpResponse::InternalServerError().body(format!("Error finding places: {}", e))
        },
    }
}
