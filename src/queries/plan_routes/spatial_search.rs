use chrono::{DateTime, Utc};
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::{LineString, Point};
use log::{debug, error, info};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::path::PathBuf;

use crate::plan_routes::_structs::{
    BusStopProperties, GeoJsonFeature, GeoJsonGeometry, RouteProperties, TransferPoint,
    TransferType,
};

// Estructura para el cache de intersecciones
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteIntersectionCache {
    version: u32,
    last_updated: DateTime<Utc>,
    intersections: HashMap<String, Vec<TransferPoint>>,
}

impl RouteIntersectionCache {
    pub fn new(intersections: HashMap<String, Vec<TransferPoint>>) -> Self {
        Self {
            version: 1,
            last_updated: Utc::now(),
            intersections,
        }
    }

    pub fn save_to_file(&self, cache_dir: &PathBuf) -> std::io::Result<()> {
        create_dir_all(cache_dir)?;
        let cache_file = cache_dir.join("route_intersections.cache");
        let file = File::create(cache_file)?;

        bincode::serialize_into(file, self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub fn load_from_file(cache_dir: &PathBuf) -> std::io::Result<Option<Self>> {
        let cache_file = cache_dir.join("route_intersections.cache");

        if !cache_file.exists() {
            return Ok(None);
        }

        let file = File::open(cache_file)?;

        match bincode::deserialize_from(file) {
            Ok(cache) => Ok(Some(cache)),
            Err(e) => {
                error!("Failed to load cache: {}", e);
                Ok(None)
            }
        }
    }
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
    #[error("Cache error: {0}")]
    CacheError(String),
}

pub struct SpatialSearch {
    bus_stops: HashMap<String, Vec<BusStopProperties>>,
    routes: HashMap<String, GeoJsonFeature<RouteProperties>>,
    route_intersections: HashMap<String, Vec<TransferPoint>>,
    cache_dir: PathBuf,
}

impl SpatialSearch {
    pub fn new(
        routes: Vec<GeoJsonFeature<RouteProperties>>,
        bus_stops: Vec<BusStopProperties>,
        cache_dir: Option<PathBuf>,
    ) -> Self {
        let bus_stops_map = bus_stops.into_iter().fold(HashMap::<String, Vec<BusStopProperties>>::new(), |mut acc, stop| {
            if let Some(ruta) = stop.ruta.clone() {
                acc.entry(ruta).or_default().push(stop);
            }
            acc
        });

        let routes_map = routes
            .into_iter()
            .filter_map(|route| {
                route
                    .properties
                    .codigo_de
                    .clone()
                    .map(|codigo_de| (codigo_de, route))
            })
            .collect();

        let cache_dir = cache_dir.unwrap_or_else(|| PathBuf::from("./cache"));

        let mut search = Self {
            bus_stops: bus_stops_map,
            routes: routes_map,
            route_intersections: HashMap::new(),
            cache_dir,
        };

        // Intentar cargar del cache
        match search.load_intersections_cache() {
            Ok(Some(cache)) => {
                info!("Loaded route intersections from cache");
                search.route_intersections = cache.intersections;
            }
            Ok(None) => {
                info!("Cache not found, precalculating intersections");
                search.precalculate_intersections();
                if let Err(e) = search.save_intersections_cache() {
                    error!("Failed to save intersection cache: {}", e);
                }
            }
            Err(e) => {
                error!("Error loading cache: {}", e);
                search.precalculate_intersections();
            }
        }

        search
    }

    fn load_intersections_cache(&self) -> Result<Option<RouteIntersectionCache>, SearchError> {
        RouteIntersectionCache::load_from_file(&self.cache_dir)
            .map_err(|e| SearchError::CacheError(e.to_string()))
    }

    fn save_intersections_cache(&self) -> Result<(), SearchError> {
        let cache = RouteIntersectionCache::new(self.route_intersections.clone());
        cache
            .save_to_file(&self.cache_dir)
            .map_err(|e| SearchError::CacheError(e.to_string()))
    }

    fn precalculate_intersections(&mut self) {
        info!("Pre-calculating route intersections");

        let route_codes: Vec<String> = self.routes.keys().cloned().collect();
        let total_routes = route_codes.len();

        // Usar un thread pool para paralelizar el cálculo
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap();

        let intersections: HashMap<String, Vec<TransferPoint>> = pool.install(|| {
            route_codes
                .par_iter()
                .map(|route_code| {
                    let mut route_intersections = Vec::new();
                    let route1 = &self.routes[route_code];

                    // Filtrar rutas que podrían intersectar basándonos en un bounding box
                    let potential_intersections = self.find_potential_intersections(route1);

                    for other_code in potential_intersections {
                        if route_code == &other_code {
                            continue;
                        }

                        let route2 = &self.routes[&other_code];
                        if let Some(transfer) = self.find_best_transfer(route1, route2) {
                            route_intersections.push(transfer);
                        }
                    }

                    info!(
                        "Processed intersections for route {} ({}/{} total)",
                        route_code,
                        route_intersections.len(),
                        total_routes
                    );

                    (route_code.clone(), route_intersections)
                })
                .collect()
        });

        self.route_intersections = intersections;
        info!("Intersection pre-calculation completed");
    }

    fn find_potential_intersections(&self, route: &GeoJsonFeature<RouteProperties>) -> Vec<String> {
        // Calcular bounding box de la ruta
        let bbox = match &route.geometry {
            GeoJsonGeometry::LineString { coordinates } => {
                let mut min_x = f64::MAX;
                let mut min_y = f64::MAX;
                let mut max_x = f64::MIN;
                let mut max_y = f64::MIN;

                for coord in coordinates {
                    min_x = min_x.min(coord[0]);
                    min_y = min_y.min(coord[1]);
                    max_x = max_x.max(coord[0]);
                    max_y = max_y.max(coord[1]);
                }

                (min_x, min_y, max_x, max_y)
            }
            _ => return vec![],
        };

        // Expandir el bounding box un poco para considerar rutas cercanas (≈1km)
        let (min_x, min_y, max_x, max_y) =
            (bbox.0 - 0.01, bbox.1 - 0.01, bbox.2 + 0.01, bbox.3 + 0.01);

        self.routes
            .iter()
            .filter_map(|(code, other_route)| {
                if let GeoJsonGeometry::LineString { coordinates } = &other_route.geometry {
                    let intersects = coordinates.iter().any(|coord| {
                        coord[0] >= min_x
                            && coord[0] <= max_x
                            && coord[1] >= min_y
                            && coord[1] <= max_y
                    });

                    if intersects {
                        Some(code.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    fn find_best_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
    ) -> Option<TransferPoint> {
        // Primero intentar encontrar una parada directa
        if let Some(transfer) = self.find_direct_transfer(route1, route2) {
            return Some(transfer);
        }

        // Luego buscar paradas cercanas (≤500m)
        if let Some(transfer) = self.find_near_transfer(route1, route2, 0.005) {
            return Some(transfer);
        }

        // Finalmente buscar puntos próximos (≤1km)
        self.find_proximate_transfer(route1, route2, 0.01)
    }

    fn find_direct_transfer(
        &self,
        route1: &GeoJsonFeature<RouteProperties>,
        route2: &GeoJsonFeature<RouteProperties>,
    ) -> Option<TransferPoint> {
        if let (Some(stops1), Some(stops2)) = (
            self.bus_stops
                .get(route1.properties.codigo_de.as_ref().unwrap()),
            self.bus_stops
                .get(route2.properties.codigo_de.as_ref().unwrap()),
        ) {
            for stop1 in stops1 {
                for stop2 in stops2 {
                    if stop1.latitud == stop2.latitud && stop1.longitud == stop2.longitud {
                        return Some(TransferPoint {
                            location: Point::new(stop1.longitud.unwrap(), stop1.latitud.unwrap()),
                            bus_stop: Some(stop1.clone()),
                            distance_to_route: 0.0,
                            transfer_type: TransferType::Direct,
                            from_route: route1.properties.codigo_de.clone().unwrap_or_default(),
                            to_route: route2.properties.codigo_de.clone().unwrap_or_default(),
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
            self.bus_stops
                .get(route1.properties.codigo_de.as_ref().unwrap()),
            self.bus_stops
                .get(route2.properties.codigo_de.as_ref().unwrap()),
        ) {
            let mut best_transfer = None;
            let mut min_distance = max_distance;

            for stop1 in stops1 {
                for stop2 in stops2 {
                    if let (Some(long1), Some(lat1), Some(long2), Some(lat2)) =
                        (stop1.longitud, stop1.latitud, stop2.longitud, stop2.latitud)
                    {
                        let point1 = Point::new(long1, lat1);
                        let point2 = Point::new(long2, lat2);
                        let distance = point1.euclidean_distance(&point2);

                        if distance < min_distance {
                            min_distance = distance;
                            best_transfer = Some(TransferPoint {
                                location: point1,
                                bus_stop: Some(stop1.clone()),
                                distance_to_route: distance,
                                transfer_type: TransferType::Near,
                                from_route: route1.properties.codigo_de.clone().unwrap_or_default(),
                                to_route: route2.properties.codigo_de.clone().unwrap_or_default(),
                            });
                        }
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

        if let (
            GeoJsonGeometry::LineString {
                coordinates: coords1,
            },
            GeoJsonGeometry::LineString {
                coordinates: coords2,
            },
        ) = (&route1.geometry, &route2.geometry)
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
                            distance_to_route: distance,
                            transfer_type: TransferType::Proximate,
                            from_route: route1.properties.codigo_de.clone().unwrap_or_default(),
                            to_route: route2.properties.codigo_de.clone().unwrap_or_default(),
                        });
                    }
                }
            }
        }

        best_transfer
    }

    pub fn find_routes_to_destination(
        &self,
        origin: Point<f64>,
        destination: Point<f64>,
        max_transfers: i32,
        max_route_distance: f64,
    ) -> Result<Vec<crate::plan_routes::_structs::RoutePlan>, SearchError> {
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

        // Ordenar rutas por número de transbordos y distancia total
        route_plans.sort_by(|a, b| {
            let transfers_cmp = a.transfers_count.cmp(&b.transfers_count);
            if transfers_cmp != std::cmp::Ordering::Equal {
                return transfers_cmp;
            }

            a.total_distance
                .partial_cmp(&b.total_distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(route_plans.into_iter().take(3).collect())
    }

    fn find_nearby_routes(
        &self,
        point: Point<f64>,
        max_distance: f64,
    ) -> Vec<&GeoJsonFeature<RouteProperties>> {
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
        origin: Point<f64>,
        destination: Point<f64>,
        max_transfers: i32,
    ) -> Result<Vec<crate::plan_routes::_structs::RoutePlan>, SearchError> {
        let mut plans = Vec::new();
        let mut visited = HashSet::new();

        for start_route in origin_routes {
            let mut current_plan = crate::plan_routes::_structs::RoutePlan::new();
            visited.clear();
            visited.insert(start_route.properties.codigo_de.clone().unwrap_or_default());

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
        destination: Point<f64>,
        transfers_left: i32,
        visited: &mut HashSet<String>,
        current_plan: &mut crate::plan_routes::_structs::RoutePlan,
        all_plans: &mut Vec<crate::plan_routes::_structs::RoutePlan>,
    ) {
        // Si llegamos a una ruta de destino, agregar el plan
        if destination_routes.contains(&current_route) {
            if let Some(end_point) = self.find_closest_point_on_route(current_route, destination) {
                let segment = crate::plan_routes::_structs::RouteSegment {
                    route: current_route.properties.clone(),
                    transfer_type: TransferType::Direct,
                    transfer_point: TransferPoint {
                        location: end_point,
                        bus_stop: None,
                        distance_to_route: end_point.euclidean_distance(&destination),
                        transfer_type: TransferType::Direct,
                        from_route: current_route
                            .properties
                            .codigo_de
                            .clone()
                            .unwrap_or_default(),
                        to_route: String::new(),
                    },
                    segment_distance: self
                        .calculate_route_distance(current_route, end_point)
                        .unwrap_or(0.0),
                };

                current_plan.add_segment(segment);
                all_plans.push(current_plan.clone());
                return;
            }
        }

        // Si no quedan transferencias disponibles, retornar
        if transfers_left <= 0 {
            return;
        }

        // Explorar las intersecciones con otras rutas
        if let Some(transfers) = self
            .route_intersections
            .get(current_route.properties.codigo_de.as_ref().unwrap())
        {
            for transfer in transfers {
                if let Some(next_route) = self.routes.get(&transfer.to_route) {
                    if !visited.contains(next_route.properties.codigo_de.as_ref().unwrap()) {
                        visited.insert(next_route.properties.codigo_de.clone().unwrap_or_default());

                        let segment = crate::plan_routes::_structs::RouteSegment {
                            route: current_route.properties.clone(),
                            transfer_point: transfer.clone(),
                            transfer_type: transfer.transfer_type.clone(),
                            segment_distance: self
                                .calculate_route_distance(current_route, transfer.location)
                                .unwrap_or(0.0),
                        };

                        current_plan.add_segment(segment);

                        self.explore_route_path(
                            next_route,
                            destination_routes,
                            destination,
                            transfers_left - 1,
                            visited,
                            current_plan,
                            all_plans,
                        );

                        visited.remove(next_route.properties.codigo_de.as_ref().unwrap());
                        current_plan.routes.pop();
                    }
                }
            }
        }
    }

    fn find_closest_point_on_route(
        &self,
        route: &GeoJsonFeature<RouteProperties>,
        point: Point<f64>,
    ) -> Option<Point<f64>> {
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
        point: Point<f64>,
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

    // TODO: Implementar pruebas unitarias
    #[test]
    fn test_find_nearby_routes() {
        // Implementar test
    }

    #[test]
    fn test_route_finding() {
        // Implementar test
    }
}
