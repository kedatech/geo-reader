use crate::db::connect_to_db;
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use tokio_postgres::Error;

/// Alias para facilitar el manejo de coordenadas.
type Coordinate = (OrderedFloat<f64>, OrderedFloat<f64>);
type Graph = BTreeMap<Coordinate, BTreeMap<Coordinate, OrderedFloat<f64>>>;

/// Carga el grafo desde la base de datos PostgreSQL.
pub async fn load_graph_from_db() -> Result<Graph, Error> {
    let client = connect_to_db().await?;

    let query = "
        SELECT
            ST_X(ST_Transform(ST_StartPoint(way), 4326)) AS start_lon,
            ST_Y(ST_Transform(ST_StartPoint(way), 4326)) AS start_lat,
            ST_X(ST_Transform(ST_EndPoint(way), 4326)) AS end_lon,
            ST_Y(ST_Transform(ST_EndPoint(way), 4326)) AS end_lat,
            ST_Length(ST_Transform(way, 4326)::geography) AS distance
        FROM planet_osm_line;
    ";

    let rows = client.query(query, &[]).await?;
    let mut graph: Graph = BTreeMap::new();

    for row in rows {
        let start = (
            OrderedFloat(row.get::<_, f64>(1)),
            OrderedFloat(row.get::<_, f64>(0)),
        );
        let end = (
            OrderedFloat(row.get::<_, f64>(3)),
            OrderedFloat(row.get::<_, f64>(2)),
        );
        let distance = OrderedFloat(row.get::<_, f64>(4));

        // Inserción en el grafo (bidireccional).
        graph.entry(start).or_default().insert(end, distance);
        graph.entry(end).or_default().insert(start, distance);
    }

    Ok(graph)
}
