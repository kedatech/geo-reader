use serde::Serialize;
use std::path::Path;
use tantivy::{
    query::{BooleanQuery, Occur, RangeQuery, TermQuery},
    Term,
    collector::TopDocs,
    Index, Document, schema::*,
};
use tracing::{info, error};
use geo::{Point, Polygon, Contains};
use serde_json::Value;

#[derive(Debug, Serialize, Clone)]
pub struct RouteStep {
    pub route_id: i32,
    pub bus_id: i32,
    pub number_route: String,
    pub geometry: Value,
    pub fees: f64,
    pub first_trip: String,
    pub last_trip: String,
    pub frequency: String,
    pub distance: f64,
}

#[derive(Debug, Serialize)]
pub struct PlanRoute {
    pub routes: Vec<RouteStep>,
    pub total_distance: f64,
    pub estimated_time: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum RoutePlanError {
    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de Tantivy: {0}")]
    Tantivy(#[from] tantivy::error::TantivyError),
    #[error("Error de MessagePack: {0}")]
    MessagePack(String),
    #[error("Punto de inicio ({0}, {1}) fuera de El Salvador")]
    StartPointOutOfBounds(f64, f64),
    #[error("Punto de destino ({0}, {1}) fuera de El Salvador")]
    EndPointOutOfBounds(f64, f64),
    #[error("Error en la geometría: {0}")]
    Geometry(String),
    #[error("No se encontraron rutas disponibles")]
    NoRoutesFound,
}

struct DepartmentInfo {
    name: String,
    polygon: Polygon<f64>,
}

pub struct RoutePlanner {
    index: Index,
    departments: Vec<DepartmentInfo>,
}

impl RoutePlanner {
    pub fn new() -> Result<Self, RoutePlanError> {
        let index_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("index");
        
        let index = Index::open_in_dir(index_path)
            .map_err(RoutePlanError::Tantivy)?;
        let departments = Self::load_departments(&index)?;
        
        Ok(RoutePlanner {
            index,
            departments,
        })
    }
    fn load_departments(index: &Index) -> Result<Vec<DepartmentInfo>, RoutePlanError> {
        let reader = index.reader().map_err(RoutePlanError::Tantivy)?;
        let searcher = reader.searcher();
        
        let schema = index.schema();
        let tipo_field = schema.get_field("tipo").unwrap();
        let name_field = schema.get_field("name").unwrap();
        let geometry_field = schema.get_field("geometry").unwrap();
    
        let query = TermQuery::new(
            Term::from_field_text(tipo_field, "departamento"),
            IndexRecordOption::Basic,
        );
    
        let top_docs = searcher.search(&query, &TopDocs::with_limit(100))
            .map_err(RoutePlanError::Tantivy)?;
    
        let departments: Vec<DepartmentInfo> = top_docs
            .iter()
            .filter_map(|(_score, doc_address)| {
                let doc = searcher.doc(*doc_address).ok()?;
                let polygon = Self::extract_polygon(&doc, geometry_field).ok()??;
                let name = doc.get_first(name_field)?.as_text()?.to_string();
                Some(DepartmentInfo { name, polygon })
            })
            .collect();
    
        if departments.is_empty() {
            error!("No se encontraron departamentos en el índice");
            return Err(RoutePlanError::NoRoutesFound);
        }
    
        info!("Cargados {} departamentos del índice Tantivy", departments.len());
        Ok(departments)
    }

    
    fn extract_polygon(doc: &Document, geometry_field: Field) -> Result<Option<Polygon<f64>>, RoutePlanError> {
        if let Some(geometry_value) = doc.get_first(geometry_field) {
            if let Some(geometry_str) = geometry_value.as_text() {
                match serde_json::from_str::<Value>(geometry_str) {
                    Ok(geojson) => {
                        if let Some(coordinates) = geojson.get("coordinates") {
                            if let Some(coords_array) = coordinates.as_array() {
                                if let Some(outer_ring) = coords_array.get(0) {
                                    if let Some(points) = outer_ring.as_array() {
                                        let points: Vec<(f64, f64)> = points
                                            .iter()
                                            .filter_map(|point| {
                                                if let Some(coords) = point.as_array() {
                                                    if coords.len() >= 2 {
                                                        if let (Some(x), Some(y)) = (coords[0].as_f64(), coords[1].as_f64()) {
                                                            return Some((x, y));
                                                        }
                                                    }
                                                }
                                                None
                                            })
                                            .collect();

                                        if !points.is_empty() {
                                            use geo::{LineString, Polygon};
                                            let line_string = LineString::from(points);
                                            return Ok(Some(Polygon::new(line_string, vec![])));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(RoutePlanError::Geometry(format!("Error parsing GeoJSON: {}", e)));
                    }
                }
            }
        }
        Ok(None)
    }

    fn find_containing_department(&self, point: Point<f64>) -> Option<&DepartmentInfo> {
        let result = self.departments.iter()
            .find(|dept| {
                let contains = dept.polygon.contains(&point);
                info!(
                    "Verificando punto ({}, {}) en departamento {}: {}",
                    point.x(), point.y(), dept.name, contains
                );
                contains
            });

        if result.is_none() {
            error!(
                "Punto ({}, {}) no está dentro de ningún departamento",
                point.x(), point.y()
            );
        }

        result
    }

    pub async fn find_route_plans(
        &self,
        start_lat: f64,
        start_lng: f64,
        end_lat: f64,
        end_lng: f64,
    ) -> Result<Vec<PlanRoute>, RoutePlanError> {
        info!(
            "Iniciando planificación de ruta desde ({}, {}) hasta ({}, {})",
            start_lat, start_lng, end_lat, end_lng
        );

        let start_point = Point::new(start_lng, start_lat);
        let end_point = Point::new(end_lng, end_lat);

        let start_dept = self.find_containing_department(start_point)
            .ok_or(RoutePlanError::StartPointOutOfBounds(start_lat, start_lng))?;
        
        let end_dept = self.find_containing_department(end_point)
            .ok_or(RoutePlanError::EndPointOutOfBounds(end_lat, end_lng))?;

        let reader = self.index.reader()
            .map_err(RoutePlanError::Tantivy)?;
        let searcher = reader.searcher();

        let mut clauses = Vec::new();

        if start_dept.name != end_dept.name {
            info!("Puntos en diferentes departamentos: {} -> {}", start_dept.name, end_dept.name);
            let tipo_field = self.index.schema().get_field("tipo").unwrap();
            clauses.push((
                Occur::Should,
                Box::new(TermQuery::new(
                    Term::from_field_text(tipo_field, "interdepartamental"),
                    IndexRecordOption::Basic,
                )) as Box<dyn tantivy::query::Query>,
            ));
        } else {
            info!("Puntos en el mismo departamento: {}", start_dept.name);
        }

        let lat_field = self.index.schema().get_field("latitude").unwrap();
        let lon_field = self.index.schema().get_field("longitude").unwrap();

        const SEARCH_RADIUS: f64 = 0.005;

        clauses.push((
            Occur::Must,
            Box::new(RangeQuery::new_f64(
                lat_field,
                (start_lat - SEARCH_RADIUS)..(start_lat + SEARCH_RADIUS),
            )) as Box<dyn tantivy::query::Query>,
        ));

        clauses.push((
            Occur::Must,
            Box::new(RangeQuery::new_f64(
                lon_field,
                (start_lng - SEARCH_RADIUS)..(start_lng + SEARCH_RADIUS),
            )) as Box<dyn tantivy::query::Query>,
        ));

        let query = BooleanQuery::new(clauses);

        let top_docs = searcher.search(&query, &TopDocs::with_limit(15))
            .map_err(RoutePlanError::Tantivy)?;

        let mut route_plans = Vec::new();
        for (_score, doc_address) in top_docs {
            if let Ok(doc) = searcher.doc(doc_address) {
                if let Some(route) = self.convert_doc_to_route_step(&doc)? {
                    route_plans.push(PlanRoute {
                        routes: vec![route.clone()],
                        total_distance: 0.0,
                        estimated_time: estimate_total_time(&[route]),
                    });
                }
            }
        }

        if route_plans.is_empty() {
            return Err(RoutePlanError::NoRoutesFound);
        }

        info!("Encontrados {} planes de ruta posibles", route_plans.len());
        Ok(route_plans)
    }

    fn convert_doc_to_route_step(&self, doc: &Document) -> Result<Option<RouteStep>, RoutePlanError> {
        let schema = self.index.schema();
        let route_id = doc.get_first(schema.get_field("route_id").unwrap())
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .ok_or_else(|| RoutePlanError::Geometry("Campo route_id no encontrado".to_string()))?;
            
        let bus_id = doc.get_first(schema.get_field("bus_id").unwrap())
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .ok_or_else(|| RoutePlanError::Geometry("Campo bus_id no encontrado".to_string()))?;

        let number_route = doc.get_first(schema.get_field("number_route").unwrap())
            .and_then(|v| v.as_text())
            .ok_or_else(|| RoutePlanError::Geometry("Campo number_route no encontrado".to_string()))?
            .to_string();

        let geometry_str = doc.get_first(schema.get_field("geometry").unwrap())
            .and_then(|v| v.as_text())
            .ok_or_else(|| RoutePlanError::Geometry("Campo geometry no encontrado".to_string()))?;

        let geometry: Value = serde_json::from_str(geometry_str)
            .map_err(|e| RoutePlanError::Geometry(format!("Error parsing geometry: {}", e)))?;

        let fees = doc.get_first(schema.get_field("fees").unwrap())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let first_trip = doc.get_first(schema.get_field("first_trip").unwrap())
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let last_trip = doc.get_first(schema.get_field("last_trip").unwrap())
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let frequency = doc.get_first(schema.get_field("frequency").unwrap())
            .and_then(|v| v.as_text())
            .unwrap_or("")
            .to_string();

        let distance = doc.get_first(schema.get_field("distance").unwrap())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        Ok(Some(RouteStep {
            route_id,
            bus_id,
            number_route,
            geometry,
            fees,
            first_trip,
            last_trip,
            frequency,
            distance,
        }))
    }
}

fn estimate_total_time(routes: &[RouteStep]) -> i32 {
    let transfer_time = ((routes.len() - 1) * 5) as i32;
    let route_time = (routes.len() * 25) as i32;
    transfer_time + route_time
}

pub async fn find_route_plans_tantivy(
    start_lat: f64,
    start_lng: f64,
    end_lat: f64,
    end_lng: f64,
) -> Result<Vec<PlanRoute>, RoutePlanError> {
    info!(
        "Iniciando planificación de ruta con Tantivy desde ({}, {}) hasta ({}, {})",
        start_lat, start_lng, end_lat, end_lng
    );

    let planner = RoutePlanner::new()?;
    planner.find_route_plans(start_lat, start_lng, end_lat, end_lng).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_planner() -> Result<(), RoutePlanError> {
        let planner = RoutePlanner::new()?;
        
        let routes = planner.find_route_plans(
            13.6894,
            -89.1872,
            13.7025,
            -89.2240,
        ).await?;
        
        assert!(!routes.is_empty(), "Debería encontrar al menos una ruta");
        Ok(())
    }

    #[test]
    fn test_invalid_coordinates() {
        let planner = RoutePlanner::new().unwrap();
        
        let result = tokio_test::block_on(planner.find_route_plans(
            0.0, 0.0,
            13.7025, -89.2240,
        ));
        
        assert!(matches!(result, Err(RoutePlanError::StartPointOutOfBounds(_, _))));
    }
}