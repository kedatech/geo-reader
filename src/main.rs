use actix_web::{App, HttpServer};
use dotenv::dotenv;
use std::env;
use env_logger::Env;
use log::info;

mod db;
mod api;
mod middlewares;

pub mod queries;
pub mod utils;
pub mod algorithms;

pub use queries::*;
pub use utils::*;
pub use algorithms::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Inicializar el logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    // Cargar variables de entorno
    dotenv().ok();

    let port = env::var("PORT").unwrap_or_else(|_| "8087".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("Starting server on {}", addr);

    HttpServer::new(|| {
        App::new()
            .wrap(middlewares::logger::RequestLogger)  // Agregar el middleware
            .configure(api::config)
    })
    .bind(&addr)?
    .run()
    .await
}