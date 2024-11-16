use actix_web::web;

mod handlers;
use handlers::find_places; // Importa la función desde handlers

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/places", web::get().to(find_places))
            // Agrega otras rutas aquí si las tienes
    );
}
