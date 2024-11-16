use actix_web::{web, HttpResponse, Responder};
use serde::Deserialize;
use crate::db::connect_to_db;

#[derive(Deserialize)]
pub struct PlaceQuery {
    name: String,
}

pub async fn find_places(query: web::Query<PlaceQuery>) -> impl Responder {
    let db_client = connect_to_db().await.unwrap();
    // Llama a tu función find_places_by_name y procesa el resultado
    match find_places_by_name(&query.name, &db_client).await {
        Ok(places) => HttpResponse::Ok().json(places),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

