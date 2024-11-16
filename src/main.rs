use actix_web::{App, HttpServer};
use dotenv::dotenv;
use std::env;
mod db;
mod api;

pub mod queries;
pub mod utils;
pub mod algorithms;

pub use queries::*;
pub use utils::*;
pub use algorithms::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);

    HttpServer::new(|| {
        App::new()
            .configure(api::config) // Configura la API aquí
    })
    .bind(&addr)?
    .run()
    .await
}
