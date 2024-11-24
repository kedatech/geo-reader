use tokio_postgres::{Client, Error};
use serde_json::Value;
use serde::Serialize;

#[derive(Serialize)]
pub struct RouteResult {
    pub stops: Vec<String>,
    pub route_geometry: Value, // GeoJSON de la ruta
}

pub async fn calculate_route(
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
    client: &Client,
) -> Result<RouteResult, Error> {
    // Encuentra las paradas más cercanas a A y B
    let find_nearest_stop_query = "
        SELECT id, name, ST_Distance(geometry, ST_SetSRID(ST_MakePoint($1, $2), 4326)) AS distance
        FROM stops
        ORDER BY distance
        LIMIT 1;
    ";

    let start_stop_row = client
        .query_one(find_nearest_stop_query, &[&start_lng, &start_lat])
        .await?;
    let start_stop_id: i32 = start_stop_row.get(0);

    let end_stop_row = client
        .query_one(find_nearest_stop_query, &[&end_lng, &end_lat])
        .await?;
    let end_stop_id: i32 = end_stop_row.get(0);

    // Encuentra rutas que conecten las paradas
    let find_routes_query = "
        WITH start_routes AS (
            SELECT r.id AS route_id, r.geometry AS route_geometry
            FROM routes r
            JOIN stops_routes sr ON r.id = sr.route_id
            WHERE sr.stop_id = $1
        ),
        end_routes AS (
            SELECT r.id AS route_id
            FROM routes r
            JOIN stops_routes sr ON r.id = sr.route_id
            WHERE sr.stop_id = $2
        )
        SELECT sr.route_id, ST_AsGeoJSON(sr.route_geometry)::TEXT AS geometry
        FROM start_routes sr
        JOIN end_routes er ON sr.route_id = er.route_id;
    ";

    let route_row = client
        .query_one(find_routes_query, &[&start_stop_id, &end_stop_id])
        .await?;

    let route_geometry: String = route_row.get(1);
    let parsed_geometry: Value = serde_json::from_str(&route_geometry).unwrap_or_else(|_| {
        serde_json::json!({
            "type": "LineString",
            "coordinates": []
        })
    });

    // Devuelve los resultados
    Ok(RouteResult {
        stops: vec![
            start_stop_row.get::<_, String>(1),
            end_stop_row.get::<_, String>(1),
        ],
        route_geometry: parsed_geometry,
    })
}
