use geo::{Point, MultiPolygon, Coord};
use geo::algorithm::contains::Contains;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use tracing::{debug, error, info};
use crate::plan_routes::_structs::*;

pub struct GeoValidator {
    departments: Vec<DepartmentBoundary>,
}

#[derive(Debug)]
pub struct DepartmentBoundary {
    name: String,
    boundary: MultiPolygon,
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
    pub fn new(departments: Vec<Department>) -> Self {
        let department_boundaries = departments
            .into_iter()
            .map(|dept| DepartmentBoundary {
                name: dept.nam.clone(),
                boundary: dept.geometry,
            })
            .collect();

        Self {
            departments: department_boundaries,
        }
    }

    /// Valida un punto y retorna el departamento al que pertenece
    pub fn validate_point(&self, point: Point) -> Result<Option<String>, ValidationError> {
        // Primero verificamos si el punto está dentro de algún departamento
        for dept in &self.departments {
            if dept.boundary.contains(&point) {
                debug!("Point found in department: {}", dept.name);
                return Ok(Some(dept.name.clone()));
            }
        }

        // Si no está dentro de ningún departamento, calculamos la distancia al más cercano
        let min_distance = self.departments
            .iter()
            .map(|dept| dept.boundary.euclidean_distance(&point))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f64::MAX);

        // Si la distancia es menor a un umbral (por ejemplo, 1km en grados)
        if min_distance < 0.01 { // aproximadamente 1km
            debug!("Point near department boundary: {:.6}", min_distance);
            Ok(None)
        } else {
            error!("Point outside country boundaries: {:.6}", min_distance);
            Err(ValidationError::OutsideCountry)
        }
    }

    /// Valida un par de puntos y determina si el viaje es interdepartamental
    pub fn validate_route(&self, origin: Point, destination: Point) -> Result<ValidationResult, ValidationError> {
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

        let result = ValidationResult {
            is_valid: origin_dept.is_some() && dest_dept.is_some(),
            origin_department: origin_dept,
            destination_department: dest_dept,
            is_interdepartmental,
            distance_to_boundary,
        };

        debug!("Route validation result: {:?}", result);
        Ok(result)
    }

    /// Encuentra los departamentos que intersectan una ruta
    pub fn get_route_departments(&self, route: &Route) -> Vec<String> {
        let coords = route.geometry.coords().collect::<Vec<_>>();
        
        self.departments
            .iter()
            .filter(|dept| {
                coords.iter().any(|coord| {
                    let point = Point::new(coord.x, coord.y);
                    dept.boundary.contains(&point)
                })
            })
            .map(|dept| dept.name.clone())
            .collect()
    }

    /// Determina si un punto está cerca de un límite departamental
    pub fn is_near_boundary(&self, point: Point, max_distance: f64) -> bool {
        let is_near = self.departments
            .iter()
            .any(|dept| dept.boundary.euclidean_distance(&point) <= max_distance);

        debug!("Point near boundary check: {}", is_near);
        is_near
    }

    /// Obtiene el departamento más cercano a un punto
    pub fn get_nearest_department(&self, point: Point) -> Result<String, ValidationError> {
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
        // Coordenadas de ejemplo en San Salvador
        let point = Point::new(-89.2182, 13.6929);
        
        // Necesitarías cargar los datos reales de departamentos para esta prueba
        let validator = GeoValidator::new(vec![]); // Mock data
        
        let result = validator.validate_point(point);
        assert!(result.is_ok());
    }

    #[test]
    fn test_route_validation() {
        let origin = Point::new(-89.2182, 13.6929); // San Salvador
        let destination = Point::new(-89.1821, 13.7084); // Diferente punto
        
        let validator = GeoValidator::new(vec![]); // Mock data
        
        let result = validator.validate_route(origin, destination);
        assert!(result.is_ok());
    }
}