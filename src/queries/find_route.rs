use tokio_postgres::{Client, Error};
use crate::queries::_structs::Route;
use serde_json::Value;

pub async fn find_route(
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
    client: &Client,
) -> Result<Vec<Route>, Error> {
    let query = "
        WITH start_point AS (
            SELECT 
                way 
            FROM 
                planet_osm_point 
            WHERE 
                ST_DWithin(
                    way, 
                    ST_SetSRID(ST_Point($1, $2), 4326), 
                    0.01
                )
            ORDER BY 
                ST_Distance(
                    way, 
                    ST_SetSRID(ST_Point($1, $2), 4326)
                )
            LIMIT 1
        ),
        end_point AS (
            SELECT 
                way 
            FROM 
                planet_osm_point 
            WHERE 
                ST_DWithin(
                    way, 
                    ST_SetSRID(ST_Point($3, $4), 4326), 
                    0.01
                )
            ORDER BY 
                ST_Distance(
                    way, 
                    ST_SetSRID(ST_Point($3, $4), 4326)
                )
            LIMIT 1
        )
        SELECT 
            r.id AS route_id,
            r.bus_id,
            r.direction_id,
            ST_AsGeoJSON(r.geometry)::TEXT AS route_geometry,
            NULL AS distance,
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
        WHERE 
            ST_Intersects(r.geometry, (SELECT way FROM start_point)) AND
            ST_Intersects(r.geometry, (SELECT way FROM end_point))
        LIMIT 10;
    ";

    let rows = client.query(query, &[&start_lng, &start_lat, &end_lng, &end_lat]).await?;

    let routes: Vec<Route> = rows
    .iter()
    .map(|row| {
        // Obtén el JSON como un string
        let geometry_json: String = row.get(3);

        // Deserializa el JSON manualmente
        let route_geometry: Value = serde_json::from_str(&geometry_json).unwrap_or(Value::Null);

        Route {
            route_id: row.get(0),
            bus_id: row.get(1),
            direction_id: row.get(2),
            route_geometry, // Ahora es un `serde_json::Value`
            distance: row.get(4),
            number_route: row.get(5),
            code_route: row.get(6),
            fees: row.get(7),
            special_fees: row.get(8),
            first_trip: row.get(9),
            last_trip: row.get(10),
            frequency: row.get(11),
            photo_url: row.get(12),
        }
    })
    .collect();

    Ok(routes)
}
