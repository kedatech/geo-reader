use tokio_postgres::{Client, Error};
use crate::queries::_structs::{PlanRoute, RouteStep};
use serde_json::Value;
use log::{info, warn};

/// Encuentra planes de ruta desde un punto de inicio hasta un punto de destino.
pub async fn find_route_plans(
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
    client: &Client,
) -> Result<Vec<PlanRoute>, Error> {
    info!(
        "Starting route planning from ({}, {}) to ({}, {})",
        start_lat, start_lng, end_lat, end_lng
    );

    let query = "
        WITH 
        start_point AS (
            SELECT ST_SetSRID(ST_MakePoint($1, $2), 4326) AS geom
        ),
        end_point AS (
            SELECT ST_SetSRID(ST_MakePoint($3, $4), 4326) AS geom
        ),
        start_routes AS (
            SELECT DISTINCT ON (r.id)
                r.id,
                r.bus_id,
                r.geometry,
                b.number_route,
                b.fees,
                b.first_trip::TEXT,
                b.last_trip::TEXT,
                b.frequency::TEXT,
                ST_Distance(start_point.geom, r.geometry) as distance_from_start,
                ST_Distance(end_point.geom, r.geometry) as distance_to_end
            FROM routes r
            CROSS JOIN start_point
            CROSS JOIN end_point
            JOIN buses b ON r.bus_id = b.id
            WHERE ST_DWithin(start_point.geom, r.geometry, 0.005)
            ORDER BY r.id, distance_from_start
            LIMIT 10
        ),
        connecting_routes AS (
            SELECT DISTINCT r2.id,
                r2.bus_id,
                r2.geometry,
                b2.number_route,
                b2.fees,
                b2.first_trip::TEXT,
                b2.last_trip::TEXT,
                b2.frequency::TEXT,
                sr.id as previous_route_id,
                ST_Distance(r2.geometry, end_point.geom) as distance_to_end
            FROM start_routes sr
            CROSS JOIN end_point
            JOIN routes r2 ON (
                ST_DWithin(sr.geometry, r2.geometry, 0.001) OR
                ST_Intersects(sr.geometry, r2.geometry)
            )
            JOIN buses b2 ON r2.bus_id = b2.id
            WHERE r2.id != sr.id
            AND ST_DWithin(r2.geometry, end_point.geom, 0.01)
        ),
        possible_routes AS (
            SELECT 
                ARRAY[id] as route_sequence,
                distance_to_end as total_distance,
                1 as num_transfers
            FROM start_routes
            WHERE ST_DWithin(geometry, (SELECT geom FROM end_point), 0.005)
            
            UNION ALL
            
            SELECT 
                ARRAY[sr.id, cr.id] as route_sequence,
                sr.distance_from_start + cr.distance_to_end as total_distance,
                2 as num_transfers
            FROM start_routes sr
            JOIN connecting_routes cr ON cr.previous_route_id = sr.id
            WHERE cr.distance_to_end < sr.distance_to_end
        )
        SELECT 
            r.id,
            r.bus_id,
            b.number_route,
            ST_AsGeoJSON(r.geometry)::TEXT as geometry,
            b.fees,
            b.first_trip::TEXT,
            b.last_trip::TEXT,
            b.frequency::TEXT,
            pr.total_distance
        FROM possible_routes pr
        CROSS JOIN UNNEST(pr.route_sequence) WITH ORDINALITY AS t(route_id, route_order)
        JOIN routes r ON r.id = t.route_id
        JOIN buses b ON r.bus_id = b.id
        ORDER BY pr.total_distance, t.route_order
        LIMIT 15;
    ";

    info!("Executing query...");
    let rows = client
        .query(query, &[&start_lng, &start_lat, &end_lng, &end_lat])
        .await
        .map_err(|e| {
            warn!("Query execution failed: {}", e);
            e
        })?;

    info!("Query executed. Found {} rows", rows.len());

    let mut route_plans: Vec<PlanRoute> = Vec::new();
    let mut current_route_steps: Vec<RouteStep> = Vec::new();
    let mut current_total_distance = -1.0_f64;

    for row in rows {
        let route_step = RouteStep {
            route_id: row.get(0),
            bus_id: row.get(1),
            number_route: row.get(2),
            geometry: serde_json::from_str(row.get::<_, String>(3).as_str())
                .unwrap_or(Value::Null),
            fees: row.get(4),
            first_trip: row.get(5),
            last_trip: row.get(6),
            frequency: row.get(7),
            distance: row.get(8),
        };

        if (current_total_distance - route_step.distance).abs() > 0.0001 {
            if !current_route_steps.is_empty() {
                route_plans.push(PlanRoute {
                    routes: current_route_steps.clone(), // Clonamos aquí
                    total_distance: current_total_distance,
                    estimated_time: estimate_total_time(&current_route_steps),
                });
                current_route_steps.clear();
            }
            current_total_distance = route_step.distance;
        }

        current_route_steps.push(route_step);
    }

    if !current_route_steps.is_empty() {
        route_plans.push(PlanRoute {
            routes: current_route_steps, // No se mueve porque ya lo clonamos en iteraciones previas
            total_distance: current_total_distance,
            estimated_time: 0,
        });
    }

    info!("Found {} possible route plans", route_plans.len());
    Ok(route_plans)
}

/// Estima el tiempo total para completar una ruta basada en los pasos.
fn estimate_total_time(routes: &[RouteStep]) -> i32 {
    let transfer_time = ((routes.len() - 1) * 5) as i32; // 5 minutos por transferencia
    let route_time = (routes.len() * 25) as i32; // 25 minutos por ruta
    transfer_time + route_time
}
