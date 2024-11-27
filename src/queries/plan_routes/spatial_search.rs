use geo::{Point, LineString, Coord};
use geo::algorithm::euclidean_distance::EuclideanDistance;
use tracing::{debug, info, warn};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::plan_routes::_structs::*;

pub struct SpatialSearch {
    bus_stops: HashMap<String, Vec<BusStop>>,  // paradas indexadas por código de ruta
    routes: HashMap<String, Route>,            // rutas indexadas por código
    route_intersections: HashMap<String, Vec<TransferPoint>>, // puntos de transferencia pre-calculados
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("No routes found near origin")]
    NoRoutesNearOrigin,
    #[error("No routes found near destination")]
    NoRoutesNearDestination,
    #[error("No valid path found")]
    NoValidPath,
    #[error("Max transfers exceeded")]
    MaxTransfersExceeded,
    #[error("Distance calculation error: {0}")]
    DistanceError(String),
}

impl SpatialSearch {
    pub fn new(routes: Vec<Route>, bus_stops: Vec<BusStop>) -> Self {
        // Indexar paradas por código de ruta
        let bus_stops_map: HashMap<String, Vec<BusStop>> = bus_stops
            .into_iter()
            .fold(HashMap::new(), |mut acc, stop| {
                acc.entry(stop.route.clone())
                   .or_default()
                   .push(stop);
                acc
            });

        // Indexar rutas por código
        let routes_map: HashMap<String, Route> = routes
            .into_iter()
            .map(|route| (route.codigo_de.clone(), route))
            .collect();

        let mut search = Self {
            bus_stops: bus_stops_map,
            routes: routes_map,
            route_intersections: HashMap::new(),
        };

        // Pre-calcular intersecciones
        search.precalculate_intersections();
        search
    }

    /// Pre-calcula los puntos de intersección entre rutas
    fn precalculate_intersections(&mut self) {
        info!("Pre-calculating route intersections");
        
        let route_codes: Vec<String> = self.routes.keys().cloned().collect();
        
        for route_code in &route_codes {
            let mut intersections = Vec::new();
            let route1 = &self.routes[route_code];

            for other_code in &route_codes {
                if route_code == other_code {
                    continue;
                }

                let route2 = &self.routes[other_code];
                if let Some(transfer) = self.find_best_transfer(route1, route2) {
                    intersections.push(transfer);
                }
            }

            self.route_intersections.insert(route_code.clone(), intersections);
        }
        
        info!("Intersection pre-calculation completed");
    }

    /// Encuentra el mejor punto de transferencia entre dos rutas
    fn find_best_transfer(&self, route1: &Route, route2: &Route) -> Option<TransferPoint> {
        // Primero buscamos paradas compartidas (transferencia directa)
        if let Some(transfer) = self.find_direct_transfer(route1, route2) {
            return Some(transfer);
        }

        // Luego buscamos paradas cercanas (≤500m)
        if let Some(transfer) = self.find_near_transfer(route1, route2, 0.005) { // ~500m
            return Some(transfer);
        }

        // Finalmente buscamos puntos donde las rutas pasan cerca (≤1km)
        self.find_proximate_transfer(route1, route2, 0.01) // ~1km
    }

    /// Busca transferencias directas (misma parada)
    fn find_direct_transfer(&self, route1: &Route, route2: &Route) -> Option<TransferPoint> {
        if let (Some(stops1), Some(stops2)) = (
            self.bus_stops.get(&route1.codigo_de),
            self.bus_stops.get(&route2.codigo_de)
        ) {
            for stop1 in stops1 {
                for stop2 in stops2 {
                    if stop1.geometry == stop2.geometry {
                        return Some(TransferPoint {
                            location: stop1.geometry,
                            bus_stop: Some(stop1.clone()),
                            distance_to_route: 0.0,
                            transfer_type: TransferType::Direct,
                            from_route: route1.codigo_de.clone(),
                            to_route: route2.codigo_de.clone(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Busca transferencias cercanas (paradas a menos de max_distance)
    fn find_near_transfer(&self, route1: &Route, route2: &Route, max_distance: f64) -> Option<TransferPoint> {
        if let (Some(stops1), Some(stops2)) = (
            self.bus_stops.get(&route1.codigo_de),
            self.bus_stops.get(&route2.codigo_de)
        ) {
            let mut best_transfer = None;
            let mut min_distance = max_distance;

            for stop1 in stops1 {
                for stop2 in stops2 {
                    let distance = stop1.geometry.euclidean_distance(&stop2.geometry);
                    if distance < min_distance {
                        min_distance = distance;
                        best_transfer = Some(TransferPoint {
                            location: stop1.geometry,
                            bus_stop: Some(stop1.clone()),
                            distance_to_route: distance,
                            transfer_type: TransferType::Near,
                            from_route: route1.codigo_de.clone(),
                            to_route: route2.codigo_de.clone(),
                        });
                    }
                }
            }
            best_transfer
        } else {
            None
        }
    }

    /// Busca transferencias próximas (rutas que pasan cerca)
    fn find_proximate_transfer(&self, route1: &Route, route2: &Route, max_distance: f64) -> Option<TransferPoint> {
        let mut best_transfer = None;
        let mut min_distance = max_distance;

        // Muestrear puntos de las rutas para encontrar los más cercanos
        for coord1 in route1.geometry.coords() {
            let point1 = Point::new(coord1.x, coord1.y);
            
            for coord2 in route2.geometry.coords() {
                let point2 = Point::new(coord2.x, coord2.y);
                let distance = point1.euclidean_distance(&point2);
                
                if distance < min_distance {
                    min_distance = distance;
                    best_transfer = Some(TransferPoint {
                        location: point1,
                        bus_stop: None,  // No hay parada en este caso
                        distance_to_route: min_distance,
                        transfer_type: TransferType::Proximate,
                        from_route: route1.codigo_de.clone(),
                        to_route: route2.codigo_de.clone(),
                    });
                }
            }
        }

        best_transfer
    }

    // Continúa en la siguiente parte...
    pub fn find_routes_to_destination(
        &self,
        origin: Point,
        destination: Point,
        max_transfers: i32,
        max_route_distance: f64,
    ) -> Result<Vec<RoutePlan>, SearchError> {
        // Encontrar rutas cercanas al origen
        let origin_routes = self.find_nearby_routes(origin, max_route_distance);
        if origin_routes.is_empty() {
            return Err(SearchError::NoRoutesNearOrigin);
        }

        // Encontrar rutas cercanas al destino
        let destination_routes = self.find_nearby_routes(destination, max_route_distance);
        if destination_routes.is_empty() {
            return Err(SearchError::NoRoutesNearDestination);
        }

        debug!(
            "Found {} routes near origin and {} near destination",
            origin_routes.len(),
            destination_routes.len()
        );

        // Buscar todas las posibles rutas con sus transbordos
        let mut route_plans = self.find_all_possible_routes(
            &origin_routes,
            &destination_routes,
            origin,
            destination,
            max_transfers,
        )?;

        // Ordenar por criterios de optimización
        route_plans.sort_by(|a, b| {
            // Primero por número de transbordos
            let transfers_cmp = a.transfers_count.cmp(&b.transfers_count);
            if transfers_cmp != std::cmp::Ordering::Equal {
                return transfers_cmp;
            }
            
            // Luego por distancia total
            a.total_distance.partial_cmp(&b.total_distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Tomar los 3 mejores planes
        Ok(route_plans.into_iter().take(3).collect())
    }

    /// Encuentra rutas cercanas a un punto
    fn find_nearby_routes(&self, point: Point, max_distance: f64) -> Vec<&Route> {
        self.routes
            .values()
            .par_bridge() // Paralelizar la búsqueda
            .filter(|route| {
                route.geometry
                    .coords()
                    .any(|coord| {
                        let route_point = Point::new(coord.x, coord.y);
                        route_point.euclidean_distance(&point) <= max_distance
                    })
            })
            .collect()
    }

    /// Encuentra todas las posibles rutas entre origen y destino
    fn find_all_possible_routes(
        &self,
        origin_routes: &[&Route],
        destination_routes: &[&Route],
        origin: Point,
        destination: Point,
        max_transfers: i32,
    ) -> Result<Vec<RoutePlan>, SearchError> {
        let mut plans = Vec::new();
        let mut visited = HashSet::new();

        // Para cada ruta cercana al origen
        for start_route in origin_routes {
            // Iniciar una nueva búsqueda desde esta ruta
            let mut current_plan = RoutePlan::new();
            visited.clear();
            visited.insert(start_route.codigo_de.clone());

            self.explore_route_path(
                start_route,
                destination_routes,
                destination,
                max_transfers,
                &mut visited,
                &mut current_plan,
                &mut plans,
            );
        }

        if plans.is_empty() {
            return Err(SearchError::NoValidPath);
        }

        Ok(plans)
    }

    /// Exploración recursiva de caminos posibles
    fn explore_route_path(
        &self,
        current_route: &Route,
        destination_routes: &[&Route],
        destination: Point,
        transfers_left: i32,
        visited: &mut HashSet<String>,
        current_plan: &mut RoutePlan,
        all_plans: &mut Vec<RoutePlan>,
    ) {
        // Verificar si la ruta actual llega al destino
        if destination_routes.contains(&current_route) {
            // Calcular punto final y añadir el segmento final
            if let Some(end_point) = self.find_closest_point_on_route(current_route, destination) {
                current_plan.add_segment(RouteSegment {
                    route: current_route.clone(),
                    transfer_type: TransferType::Direct,
                    transfer_point: TransferPoint {
                        location: end_point,
                        bus_stop: None,
                        distance_to_route: end_point.euclidean_distance(&destination),
                        transfer_type: TransferType::Direct,
                        from_route: current_route.codigo_de.clone(),
                        to_route: "".to_string(),
                    },
                    segment_distance: current_route.geometry.euclidean_distance(&destination),
                });
                all_plans.push(current_plan.clone());
            }
            return;
        }

        // Si no quedan transferencias disponibles, retornar
        if transfers_left <= 0 {
            return;
        }

        // Obtener posibles transferencias desde la ruta actual
        if let Some(transfers) = self.route_intersections.get(&current_route.codigo_de) {
            for transfer in transfers {
                let next_route = self.routes.get(&transfer.to_route);
                if let Some(next_route) = next_route {
                    if !visited.contains(&next_route.codigo_de) {
                        // Añadir la ruta actual al plan
                        visited.insert(next_route.codigo_de.clone());
                        current_plan.add_segment(RouteSegment {
                            route: current_route.clone(),
                            transfer_point: transfer.clone(),
                            transfer_type: transfer.transfer_type.clone(),
                            segment_distance: current_route.geometry.euclidean_distance(&transfer.location),
                        });

                        // Explorar el siguiente camino
                        self.explore_route_path(
                            next_route,
                            destination_routes,
                            destination,
                            transfers_left - 1,
                            visited,
                            current_plan,
                            all_plans,
                        );

                        // Backtracking
                        visited.remove(&next_route.codigo_de);
                        current_plan.routes.pop();
                    }
                }
            }
        }
    }

    /// Encuentra el punto más cercano en una ruta a un punto dado
    fn find_closest_point_on_route(&self, route: &Route, point: Point) -> Option<Point> {
        route.geometry
            .coords()
            .min_by(|a, b| {
                let pa = Point::new(a.x, a.y);
                let pb = Point::new(b.x, b.y);
                pa.euclidean_distance(&point)
                    .partial_cmp(&pb.euclidean_distance(&point))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|coord| Point::new(coord.x, coord.y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_nearby_routes() {
        // Implementar pruebas
    }

    #[test]
    fn test_route_finding() {
        // Implementar pruebas
    }
}