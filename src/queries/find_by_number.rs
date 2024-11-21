use tokio_postgres::{Client, Error};
use crate::queries::_structs::Route;

pub async fn get_routes_by_number(
    number_route: String,
    client: &Client,
) -> Result<Vec<Route>, Error> {
    let query = "
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
            b.number_route = $1
        LIMIT 10;
    ";

    let rows = client.query(query, &[&number_route]).await?;

    let routes: Vec<Route> = rows.iter()
        .map(|row| Route {
            route_id: row.get(0),
            bus_id: row.get(1),
            direction_id: row.get(2),
            route_geometry: row.get(3),
            distance: None,
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
