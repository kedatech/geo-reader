use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

/// Representa una ruta completa en el sistema.
#[derive(Serialize, Deserialize)]
pub struct Route {
    pub route_id: i32,              // ID único de la ruta
    pub bus_id: i32,                // ID del bus asociado
    pub direction_id: Option<i32>,  // Dirección (si aplica)
    pub route_geometry: Value,      // Geometría de la ruta en formato GeoJSON
    pub distance: Option<f64>,      // Distancia total, si es relevante
    pub number_route: String,       // Número de la ruta (por ejemplo, "48")
    pub code_route: String,         // Código interno de la ruta
    pub fees: Option<f64>,          // Tarifa estándar
    pub special_fees: Option<f64>,  // Tarifas especiales (si aplica)
    pub first_trip: Option<SystemTime>, // Hora del primer viaje
    pub last_trip: Option<SystemTime>,  // Hora del último viaje
    pub frequency: Option<String>,  // Frecuencia en minutos u otra representación
    pub photo_url: Option<String>,  // URL de la foto asociada
}

/// Representa un conjunto de pasos en una ruta planificada.
#[derive(Serialize)]
pub struct PlanRoute {
    pub routes: Vec<RouteStep>,     // Pasos individuales que forman la ruta
    pub total_distance: f64,        // Distancia total en metros
    pub estimated_time: i32,        // Tiempo estimado total en minutos
}

/// Representa un paso individual en una ruta.
#[derive(Serialize, Clone)] // Agregado `Clone` para permitir clonar los pasos
pub struct RouteStep {
    pub route_id: i32,              // ID único del paso
    pub bus_id: i32,                // ID del bus asociado
    pub number_route: String,       // Número de la ruta
    pub geometry: Value,            // Geometría del paso en formato GeoJSON
    pub distance: f64,              // Distancia para este paso
    pub fees: Option<f64>,          // Tarifa aplicable para este paso
    pub first_trip: Option<String>, // Hora del primer viaje (en formato ISO 8601)
    pub last_trip: Option<String>,  // Hora del último viaje (en formato ISO 8601)
    pub frequency: Option<String>,  // Frecuencia de los buses en esta ruta
}
