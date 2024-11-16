use crate::db::connect_to_db;
use tokio_postgres::Error;

/// Find nearby places within a radius.
pub async fn find_nearby_places(
    lat: f64, 
    lon: f64, 
    radius: f64
) -> Result<Vec<String>, Error> {
    let client = connect_to_db().await?;

    let query = "
        SELECT name 
        FROM planet_osm_point 
        WHERE ST_DWithin(
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
            way::geography,
            $3
        )m
        LIMIT 10;
    ";

    let rows = client.query(query, &[&lon, &lat, &radius]).await?;
    let places: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

    Ok(places)
}
