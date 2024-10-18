use crate::db::connect_to_db;
use tokio_postgres::Error;

/// Consulta lugares por nombre.
pub async fn find_places_by_name(name: &str) -> Result<Vec<String>, Error> {
    let client = connect_to_db().await?;

    let query = "
        SELECT name 
        FROM planet_osm_point 
        WHERE name ILIKE $1 
        LIMIT 10;
    ";

    let rows = client.query(query, &[&format!("%{}%", name)]).await?;
    let places: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

    Ok(places)
}
