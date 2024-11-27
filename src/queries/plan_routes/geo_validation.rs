use geo::{Point, MultiPolygon, Coord};
use geo::algorithm::contains::Contains;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use tracing::{error};
use crate::plan_routes::_structs::*;
use geo_types::Polygon;

pub struct GeoValidator {
    departments: Vec<DepartmentBoundary>,
}

#[derive(Debug)]
pub struct DepartmentBoundary {
    name: String,
    boundary: MultiPolygon<f64>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub origin_department: Option<String>,
    pub destination_department: Option<String>,
    pub is_interdepartmental: bool,
    pub distance_to_boundary: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid coordinates")]
    InvalidCoordinates,
    #[error("Point outside country boundaries")]
    OutsideCountry,
    #[error("Department not found")]
    DepartmentNotFound,
    #[error("Geometry error: {0}")]
    GeometryError(String),
}

impl GeoValidator {
    pub fn new(department_collection: &DepartmentFeatureCollection) -> Self {
        let department_boundaries = department_collection.features
            .iter()
            .map(|feature| {
                let geometry = match &feature.geometry {
                    GeoJsonGeometry::Polygon { coordinates } => {
                        let exterior: Vec<Coord<f64>> = coordinates[0].iter()
                            .map(|coord| Coord { x: coord[0], y: coord[1] })
                            .collect();
                        let interiors: Vec<Vec<Coord<f64>>> = coordinates[1..]
                            .iter()
                            .map(|interior| {
                                interior.iter()
                                    .map(|coord| Coord { x: coord[0], y: coord[1] })
                                    .collect()
                            })
                            .collect();
                        MultiPolygon(vec![Polygon::new(
                            exterior.into(),
                            interiors.into_iter().map(|i| i.into()).collect()
                        )])
                    },
                    _ => MultiPolygon(vec![]),
                };

                DepartmentBoundary {
                    name: feature.properties.nam.clone(),
                    boundary: geometry,
                }
            })
            .collect();

        Self { departments: department_boundaries }
    }

    pub fn validate_point(&self, point: Point<f64>) -> Result<Option<String>, ValidationError> {
        for dept in &self.departments {
            if dept.boundary.contains(&point) {
                return Ok(Some(dept.name.clone()));
            }
        }

        let min_distance = self.departments
            .iter()
            .map(|dept| dept.boundary.euclidean_distance(&point))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f64::MAX);

        if min_distance < 0.01 {
            Ok(None)
        } else {
            Err(ValidationError::OutsideCountry)
        }
    }

    pub fn validate_route(&self, origin: Point<f64>, destination: Point<f64>) -> Result<ValidationResult, ValidationError> {
        let origin_dept = self.validate_point(origin)?;
        let dest_dept = self.validate_point(destination)?;

        let distance_to_boundary = self.departments
            .iter()
            .map(|dept| dept.boundary.euclidean_distance(&origin))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f64::MAX);

        let is_interdepartmental = match (&origin_dept, &dest_dept) {
            (Some(orig), Some(dest)) => orig != dest,
            _ => false,
        };

        Ok(ValidationResult {
            is_valid: origin_dept.is_some() && dest_dept.is_some(),
            origin_department: origin_dept,
            destination_department: dest_dept,
            is_interdepartmental,
            distance_to_boundary,
        })
    }

    pub fn get_route_departments(&self, route_feature: &GeoJsonFeature<RouteProperties>) -> Vec<String> {
        let coords = match &route_feature.geometry {
            GeoJsonGeometry::LineString { coordinates } => coordinates,
            _ => return vec![],
        };

        self.departments
            .iter()
            .filter(|dept| {
                coords.iter().any(|coord| {
                    let point = Point::new(coord[0], coord[1]);
                    dept.boundary.contains(&point)
                })
            })
            .map(|dept| dept.name.clone())
            .collect()
    }

    pub fn is_near_boundary(&self, point: Point<f64>, max_distance: f64) -> bool {
        self.departments
            .iter()
            .any(|dept| dept.boundary.euclidean_distance(&point) <= max_distance)
    }

    pub fn get_nearest_department(&self, point: Point<f64>) -> Result<String, ValidationError> {
        self.departments
            .iter()
            .map(|dept| {
                let distance = dept.boundary.euclidean_distance(&point);
                (dept.name.clone(), distance)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(name, _)| name)
            .ok_or(ValidationError::DepartmentNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_validation() {
        let point = Point::new(-89.2182, 13.6929);
        
        // Use empty DepartmentFeatureCollection for tests
        let empty_collection = DepartmentFeatureCollection {
            r#type: String::new(),
            name: String::new(),
            crs: GeoJsonCrs {
                r#type: String::new(),
                properties: GeoJsonCrsProperties {
                    name: String::new(),
                },
            },
            features: vec![],
        };
        
        let validator = GeoValidator::new(&empty_collection);
        let result = validator.validate_point(point);
        assert!(result.is_ok());
    }

    #[test]
    fn test_route_validation() {
        let origin = Point::new(-89.2182, 13.6929);
        let destination = Point::new(-89.1821, 13.7084);
        
        // Use empty DepartmentFeatureCollection for tests
        let empty_collection = DepartmentFeatureCollection {
            r#type: String::new(),
            name: String::new(),
            crs: GeoJsonCrs {
                r#type: String::new(),
                properties: GeoJsonCrsProperties {
                    name: String::new(),
                },
            },
            features: vec![],
        };
        
        let validator = GeoValidator::new(&empty_collection);
        let result = validator.validate_route(origin, destination);
        assert!(result.is_ok());
    }
}
