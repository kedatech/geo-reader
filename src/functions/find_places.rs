use crate::db::connect_to_db;
use tokio_postgres::Error;

/// Consulta lugares por nombre y devuelve un array de resultados.
pub async fn find_places_by_name(name: &str) -> Result<Vec<(String, f64, f64)>, Error> {
    let client = connect_to_db().await?;

    let query = "
        SELECT 
            name, 
            ST_X(ST_Transform(way, 4326)) AS longitude,
            ST_Y(ST_Transform(way, 4326)) AS latitude
        FROM 
            planet_osm_point 
        WHERE 
            name ILIKE $1 
        LIMIT 10;
    ";

    let rows = client.query(query, &[&format!("%{}%", name)]).await?;
    
    // Mapeamos los resultados en un vector de tuplas (nombre, longitud, latitud).
    let places: Vec<(String, f64, f64)> = rows.iter()
        .map(|row| (
            row.get::<_, String>(0),  // Nombre
            row.get::<_, f64>(1),     // Longitud
            row.get::<_, f64>(2)      // Latitud
        ))
        .collect();

    Ok(places)
}
