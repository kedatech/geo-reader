use geo::{Point, LineString, Coord};
use geo::algorithm::euclidean_distance::EuclideanDistance;
use log::{debug, info, warn};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::plan_routes::_structs::*;

pub struct SpatialSearch {
    bus_stops: HashMap<String, Vec<BusStopProperties>>,
    routes: HashMap<String, GeoJsonFeature<RouteProperties>>,
    route_intersections: HashMap<String, Vec<TransferPoint>>,
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
    pub fn new(routes: Vec<GeoJsonFeature<RouteProperties>>, bus_stops: Vec<BusStopProperties>) -> Self {
        let bus_stops_map = bus_stops
            .into_iter()
            .fold(HashMap::<String, Vec<BusStopProperties>>::new(), |mut acc, stop| {
                acc.entry(stop.ruta.clone())
                   .or_default()
                   .push(stop);
                acc
             });

        let routes_map = routes
            .into_iter()
            .map(|route| (route.properties.codigo_de.clone(), route))
            .collect();

        let mut search = Self {
            bus_stops: bus_stops_map,
            routes: routes_map,
            route_intersections: HashMap::new(),
        };

        search.precalculate_intersections();
        search
    }

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

    fn find_best_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
    ) -> Option<TransferPoint> {
        if let Some(transfer) = self.find_direct_transfer(route1, route2) {
            return Some(transfer);
        }

        if let Some(transfer) = self.find_near_transfer(route1, route2, 0.005) {
            return Some(transfer);
        }

        self.find_proximate_transfer(route1, route2, 0.01)
    }

    fn find_direct_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
    ) -> Option<TransferPoint> {
        if let (Some(stops1), Some(stops2)) = (
            self.bus_stops.get(&route1.properties.codigo_de),
            self.bus_stops.get(&route2.properties.codigo_de)
        ) {
            for stop1 in stops1 {
                for stop2 in stops2 {
                    if stop1.latitud == stop2.latitud && stop1.longitud == stop2.longitud {
                        return Some(TransferPoint {
                            location: Point::new(stop1.longitud, stop1.latitud),
                            bus_stop: Some(stop1.clone()),
                            distance_to_route: 0.0,
                            transfer_type: TransferType::Direct,
                            from_route: route1.properties.codigo_de.clone(),
                            to_route: route2.properties.codigo_de.clone(),
                        });
                    }
                }
            }
        }
        None
    }

    fn find_near_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
        max_distance: f64,
    ) -> Option<TransferPoint> {
        if let (Some(stops1), Some(stops2)) = (
            self.bus_stops.get(&route1.properties.codigo_de),
            self.bus_stops.get(&route2.properties.codigo_de)
        ) {
            let mut best_transfer = None;
            let mut min_distance = max_distance;

            for stop1 in stops1 {
                for stop2 in stops2 {
                    let point1 = Point::new(stop1.longitud, stop1.latitud);
                    let point2 = Point::new(stop2.longitud, stop2.latitud);
                    let distance = point1.euclidean_distance(&point2);
                    
                    if distance < min_distance {
                        min_distance = distance;
                        best_transfer = Some(TransferPoint {
                            location: point1,
                            bus_stop: Some(stop1.clone()),
                            distance_to_route: distance,
                            transfer_type: TransferType::Near,
                            from_route: route1.properties.codigo_de.clone(),
                            to_route: route2.properties.codigo_de.clone(),
                        });
                    }
                }
            }
            best_transfer
        } else {
            None
        }
    }

    fn find_proximate_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
        max_distance: f64,
    ) -> Option<TransferPoint> {
        let mut best_transfer = None;
        let mut min_distance = max_distance;

        if let (GeoJsonGeometry::LineString { coordinates: coords1 }, GeoJsonGeometry::LineString { coordinates: coords2 }) 
            = (&route1.geometry, &route2.geometry) 
        {
            for coord1 in coords1 {
                let point1 = Point::new(coord1[0], coord1[1]);
                
                for coord2 in coords2 {
                    let point2 = Point::new(coord2[0], coord2[1]);
                    let distance = point1.euclidean_distance(&point2);
                    
                    if distance < min_distance {
                        min_distance = distance;
                        best_transfer = Some(TransferPoint {
                            location: point1,
                            bus_stop: None,
                            distance_to_route: min_distance,
                            transfer_type: TransferType::Proximate,
                            from_route: route1.properties.codigo_de.clone(),
                            to_route: route2.properties.codigo_de.clone(),
                        });
                    }
                }
            }
        }

        best_transfer
    }

    pub fn find_routes_to_destination(
        &self,
        origin: Point,
        destination: Point,
        max_transfers: i32,
        max_route_distance: f64,
    ) -> Result<Vec<RoutePlan>, SearchError> {
        let origin_routes = self.find_nearby_routes(origin, max_route_distance);
        if origin_routes.is_empty() {
            return Err(SearchError::NoRoutesNearOrigin);
        }

        let destination_routes = self.find_nearby_routes(destination, max_route_distance);
        if destination_routes.is_empty() {
            return Err(SearchError::NoRoutesNearDestination);
        }

        debug!(
            "Found {} routes near origin and {} near destination",
            origin_routes.len(),
            destination_routes.len()
        );

        let mut route_plans = self.find_all_possible_routes(
            &origin_routes,
            &destination_routes,
            origin,
            destination,
            max_transfers,
        )?;

        route_plans.sort_by(|a, b| {
            let transfers_cmp = a.transfers_count.cmp(&b.transfers_count);
            if transfers_cmp != std::cmp::Ordering::Equal {
                return transfers_cmp;
            }
            
            a.total_distance.partial_cmp(&b.total_distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(route_plans.into_iter().take(3).collect())
    }

    fn find_nearby_routes(&self, point: Point, max_distance: f64) -> Vec<&GeoJsonFeature<RouteProperties>> {
        self.routes
            .values()
            .par_bridge()
            .filter(|route| {
                if let GeoJsonGeometry::LineString { coordinates } = &route.geometry {
                    coordinates.iter().any(|coord| {
                        let route_point = Point::new(coord[0], coord[1]);
                        route_point.euclidean_distance(&point) <= max_distance
                    })
                } else {
                    false
                }
            })
            .collect()
    }

    fn find_all_possible_routes(
        &self,
        origin_routes: &[&GeoJsonFeature<RouteProperties>],
        destination_routes: &[&GeoJsonFeature<RouteProperties>],
        origin: Point,
        destination: Point,
        max_transfers: i32,
    ) -> Result<Vec<RoutePlan>, SearchError> {
        let mut plans = Vec::new();
        let mut visited = HashSet::new();

        for start_route in origin_routes {
            let mut current_plan = RoutePlan::new();
            visited.clear();
            visited.insert(start_route.properties.codigo_de.clone());

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

    fn explore_route_path(
        &self,
        current_route: &GeoJsonFeature<RouteProperties>,
        destination_routes: &[&GeoJsonFeature<RouteProperties>],
        destination: Point,
        transfers_left: i32,
        visited: &mut HashSet<String>,
        current_plan: &mut RoutePlan,
        all_plans: &mut Vec<RoutePlan>,
    ) {
        if destination_routes.contains(&current_route) {
            if let Some(end_point) = self.find_closest_point_on_route(current_route, destination) {
                current_plan.add_segment(RouteSegment {
                    route: current_route.properties.clone(),
                    transfer_type: TransferType::Direct,
                    transfer_point: TransferPoint {
                        location: end_point,
                        bus_stop: None,
                        distance_to_route: end_point.euclidean_distance(&destination),
                        transfer_type: TransferType::Direct,
                        from_route: current_route.properties.codigo_de.clone(),
                        to_route: String::new(),
                    },
                    segment_distance: self.calculate_route_distance(current_route, end_point)
                        .unwrap_or(0.0),
                });
                all_plans.push(current_plan.clone());
            }
            return;
        }

        if transfers_left <= 0 {
            return;
        }

        if let Some(transfers) = self.route_intersections.get(&current_route.properties.codigo_de) {
            for transfer in transfers {
                if let Some(next_route) = self.routes.get(&transfer.to_route) {
                    if !visited.contains(&next_route.properties.codigo_de) {
                        visited.insert(next_route.properties.codigo_de.clone());
                        current_plan.add_segment(RouteSegment {
                            route: current_route.properties.clone(),
                            transfer_point: transfer.clone(),
                            transfer_type: transfer.transfer_type.clone(),
                            segment_distance: self.calculate_route_distance(current_route, transfer.location)
                                .unwrap_or(0.0),
                        });

                        self.explore_route_path(
                            next_route,
                            destination_routes,
                            destination,
                            transfers_left - 1,
                            visited,
                            current_plan,
                            all_plans,
                        );

                        visited.remove(&next_route.properties.codigo_de);
                        current_plan.routes.pop();
                    }
                }
            }
        }
    }

    fn find_closest_point_on_route(
        &self,
        route: &GeoJsonFeature<RouteProperties>,
        point: Point,
    ) -> Option<Point> {
        if let GeoJsonGeometry::LineString { coordinates } = &route.geometry {
            coordinates
                .iter()
                .min_by(|a, b| {
                    let pa = Point::new(a[0], a[1]);
                    let pb = Point::new(b[0], b[1]);
                    pa.euclidean_distance(&point)
                        .partial_cmp(&pb.euclidean_distance(&point))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|coord| Point::new(coord[0], coord[1]))
        } else {
            None
        }
    }

    fn calculate_route_distance(
        &self,
        route: &GeoJsonFeature<RouteProperties>,
        point: Point,
    ) -> Option<f64> {
        if let GeoJsonGeometry::LineString { coordinates } = &route.geometry {
            let line: Vec<(f64, f64)> = coordinates
                .iter()
                .map(|coord| (coord[0], coord[1]))
                .collect();
            let linestring = LineString::from(line);
            Some(linestring.euclidean_distance(&point))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Implementar pruebas
    #[test]
    fn test_find_nearby_routes() {
        // Implementar test
    }

    #[test]
    fn test_route_finding() {
        // Implementar test
    }
}
