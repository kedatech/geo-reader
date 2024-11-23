use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;

#[derive(Serialize, Deserialize)]
pub struct Route {
    pub route_id: i32,
    pub bus_id: i32,
    pub direction_id: Option<i32>,
    pub route_geometry: Value,
    pub distance: Option<f64>,
    pub number_route: String,
    pub code_route: String,
    pub fees: Option<f64>,
    pub special_fees: Option<f64>,
    pub first_trip: Option<SystemTime>,
    pub last_trip: Option<SystemTime>,
    pub frequency: Option<String>,
    pub photo_url: Option<String>,
}
