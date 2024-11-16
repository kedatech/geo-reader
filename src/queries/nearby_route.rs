use tokio_postgres::{Client, Error};
use std::time::SystemTime;
use serde::Serialize;

#[derive(Serialize)]
pub struct Route {
    route_id: i32,
    bus_id: i32,
    direction_id: Option<i32>,
    route_geometry: String,
    distance: f64,
    number_route: String,
    code_route: String,
    fees: Option<f64>,
    special_fees: Option<f64>,
    first_trip: Option<SystemTime>,
    last_trip: Option<SystemTime>,
    frequency: Option<String>,
    photo_url: Option<String>,
}


pub async fn get_nearby_routes(
    latitude: f64,
    longitude: f64,
    max_distance: f64,
    client: &Client
) -> Result<Vec<Route>, Error> {
    let query = "
        WITH point AS (
            SELECT ST_SetSRID(ST_MakePoint($1, $2), 4326) AS geom
        )
        SELECT 
            r.id AS route_id,
            r.bus_id,
            r.direction_id,
            ST_AsGeoJSON(r.geometry)::TEXT AS route_geometry,
            ST_Distance(p.geom, r.geometry) AS distance,
            b.number_route,
            b.code_route,
            b.fees,
            b.special_fees,
            b.first_trip,
            b.last_trip,
            b.frequency::TEXT,
            b.photo_url
        FROM 
            routes r
        JOIN 
            buses b ON r.bus_id = b.id
        JOIN 
            point p ON ST_DWithin(p.geom, r.geometry, $3)
        ORDER BY 
            distance
        LIMIT 10;
    ";

    let rows = client.query(query, &[&longitude, &latitude, &max_distance]).await?;

    // Mapeamos las filas a la estructura `Route`
    let routes: Vec<Route> = rows.iter()
        .map(|row| Route {
            route_id: row.get(0),
            bus_id: row.get(1),
            direction_id: row.get(2),
            route_geometry: row.get(3),
            distance: row.get(4),
            number_route: row.get(5),
            code_route: row.get(6),
            fees: row.get(7),
            special_fees: row.get(8),
            first_trip: row.get(9),
            last_trip: row.get(10),
            frequency: row.get(11),
            photo_url: row.get(12),
        })
        .collect();

    Ok(routes)
}
