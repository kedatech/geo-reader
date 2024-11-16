use crate::utils::load_graph_from_db;
use crate::algorithms::astar;
use ordered_float::OrderedFloat;

type Coordinate = (OrderedFloat<f64>, OrderedFloat<f64>);

/// Encuentra la ruta más corta entre dos puntos y devuelve la representación en GeoJSON.
pub async fn find_route_as_geojson( // TODO: no funciona como se espera
    start_lat: f64, start_lon: f64,
    end_lat: f64, end_lon: f64
) -> Result<String, Box<dyn std::error::Error>> {
    let graph = load_graph_from_db().await?;

    // Buscar los nodos más cercanos al punto de inicio y fin.
    let start = find_nearest_node(start_lat, start_lon).await?;
    let end = find_nearest_node(end_lat, end_lon).await?;

    // Definir la heurística para el algoritmo A*.
    let heuristic = |(lat, lon): Coordinate| -> OrderedFloat<f64> {
        OrderedFloat(
            ((lat.into_inner() - end.0.into_inner()).powi(2)
                + (lon.into_inner() - end.1.into_inner()).powi(2))
                .sqrt(),
        )
    };

    // Ejecutar el algoritmo A* con los nodos encontrados.
    match astar(&graph, start, end, heuristic) {
        Some((_, path)) => {
            let geojson = format!(
                "{{\"type\": \"LineString\", \"coordinates\": [{}]}}",
                path.iter()
                    .map(|(lat, lon)| format!("[{}, {}]", lon.into_inner(), lat.into_inner()))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            Ok(geojson)
        }
        None => Err("No route found".into()),
    }
}
async fn find_nearest_node(lat: f64, lon: f64) -> Result<Coordinate, Box<dyn std::error::Error>> {
    let client = crate::db::connect_to_db().await?;

    let query = "
        SELECT
            ST_X(ST_Transform(way, 4326)) AS lon,
            ST_Y(ST_Transform(way, 4326)) AS lat
        FROM planet_osm_point
        WHERE ST_DWithin(
            ST_Transform(way, 4326)::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography, 
            100
        )
        ORDER BY ST_Distance(
            ST_Transform(way, 4326)::geography,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography
        )
        LIMIT 1;
    ";

    let row = client.query_opt(query, &[&lon, &lat]).await?;

    if let Some(row) = row {
        let nearest = (
            OrderedFloat(row.get::<_, f64>(1)), // lat
            OrderedFloat(row.get::<_, f64>(0)), // lon
        );
        Ok(nearest)
    } else {
        Err("No nearby node found".into())
    }
}
