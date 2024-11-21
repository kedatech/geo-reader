use std::time::SystemTime;
use serde::Serialize;

#[derive(Serialize)]
pub struct Route {
    pub route_id: i32,
    pub bus_id: i32,
    pub direction_id: Option<i32>,
    pub route_geometry: String,
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