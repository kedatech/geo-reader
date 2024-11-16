use tokio_postgres::{Client, Error};
use std::time::SystemTime;

/// Consulta rutas cercanas a una ubicación dada y devuelve un array de resultados.
pub async fn get_nearby_routes(
    latitude: f64,
    longitude: f64,
    max_distance: f64,
    client: &Client
) -> Result<Vec<(i32, i32, Option<i32>, String, f64, String, String, Option<f64>, Option<f64>, Option<SystemTime>, Option<SystemTime>, Option<String>, Option<String>)>, Error> {
    let query = "
        WITH point AS (
            SELECT ST_SetSRID(ST_MakePoint($1, $2), 4326) AS geom
        )
        SELECT 
            r.id AS route_id,
            r.bus_id,
            r.direction_id,
            ST_AsGeoJSON(r.geometry)::TEXT AS route_geometry,  -- Devuelve como texto
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
            distance;
    ";

    let rows = client.query(query, &[&longitude, &latitude, &max_distance]).await?;

    let routes: Vec<(i32, i32, Option<i32>, String, f64, String, String, Option<f64>, Option<f64>, Option<SystemTime>, Option<SystemTime>, Option<String>, Option<String>)> = rows.iter()
        .map(|row| (
            row.get(0),   // route_id
            row.get(1),   // bus_id
            row.get(2),   // direction_id (como Option<i32>)
            row.get(3),   // route_geometry como String
            row.get(4),   // distance
            row.get(5),   // number_route
            row.get(6),   // code_route
            row.get(7),   // fees (como Option<f64>)
            row.get(8),   // special_fees (como Option<f64>)
            row.get(9),   // first_trip (como Option<SystemTime>)
            row.get(10),  // last_trip (como Option<SystemTime>)
            row.get::<_, Option<String>>(11),  // frequency (como Option<String>)
            row.get(12)   // photo_url (como Option<String>)
        ))
        .collect();

    Ok(routes)
}
