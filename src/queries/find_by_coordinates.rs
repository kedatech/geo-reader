use crate::db::connect_to_db;
use tokio_postgres::Error;

/// Find a place by exact coordinates.
pub async fn find_by_coordinates(
    lat: f64, 
    lon: f64
) -> Result<Vec<String>, Error> {
    let client = connect_to_db().await?;

    let query = "
        SELECT name 
        FROM planet_osm_point 
        WHERE ST_AsText(way) = ST_AsText(ST_SetSRID(ST_MakePoint($1, $2), 4326))
        LIMIT 1;
    ";

    let rows = client.query(query, &[&lon, &lat]).await?;
    let places: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

    Ok(places)
}
