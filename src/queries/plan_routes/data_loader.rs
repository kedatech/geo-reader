use crate::middlewares::logger;
use crate::plan_routes::_structs::*;
use actix_web::error;
use geojson::{FeatureCollection, GeoJson};
use log::{debug, info, error};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

pub struct DataLoader {
    data_dir: PathBuf,
    departments: Vec<Department>,
    bus_stops: Vec<BusStop>,
    routes: Vec<Route>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("GeoJSON parsing error: {0}")]
    GeoJson(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl DataLoader {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            departments: Vec::new(),
            bus_stops: Vec::new(),
            routes: Vec::new(),
        }
    }

    /// Carga todos los datos necesarios
    pub fn load_all(&mut self) -> Result<(), LoaderError> {
        // Cargar departamentos
        self.departments = self.load_geojson("LIM DEPARTAMENTALES.geojson")?;

        // Cargar paradas
        self.bus_stops = self.load_geojson("Paradas Transporte Colectivo AMSS.geojson")?;

        // Cargar todas las rutas
        let mut routes = Vec::new();
        for route_file in [
            "Rutas Interdepartamentales.geojson",
            "Rutas Interurbanas.geojson",
            "Rutas Urbanas.geojson",
        ] {
            let mut file_routes: Vec<Route> = self.load_geojson(route_file)?;
            routes.append(&mut file_routes);
        }
        self.routes = routes;

        Ok(())
    }

    /// Carga un archivo GeoJSON específico
    fn load_geojson<T: DeserializeOwned>(&self, filename: &str) -> Result<Vec<T>, LoaderError> {
        let file_path = self.data_dir.join(filename);
        info!("Loading {}", file_path.display());

        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);

        let feature_collection: FeatureCollection = match serde_json::from_reader(reader) {
            Ok(geojson) => geojson,
            Err(e) => {
                error!("Failed to parse GeoJSON from {}: {}", filename, e);
                return Err(LoaderError::Json(e));
            }
        };

        debug!(
            "Found {} features in {}",
            feature_collection.features.len(),
            filename
        );

        let features = feature_collection
            .features
            .into_iter()
            .map(|feature| {
                let value = json!({"properties": feature.properties, "geometry": feature.geometry});
                serde_json::from_value(value).map_err(|e| {
                    error!("Failed to deserialize feature: {}", e);
                    LoaderError::Json(e)
                })
            })
            .collect::<Result<Vec<T>, LoaderError>>()?;

        Ok(features)
    }
    // Getters para acceder a los datos cargados
    pub fn departments(&self) -> &[Department] {
        &self.departments
    }

    pub fn bus_stops(&self) -> &[BusStop] {
        &self.bus_stops
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    // Métodos de utilidad para consultas comunes
    pub fn find_routes_by_department(&self, department: &str) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|route| route.departamento == department)
            .collect()
    }

    pub fn find_stops_by_route(&self, route_code: &str) -> Vec<&BusStop> {
        self.bus_stops
            .iter()
            .filter(|stop| stop.route == route_code)
            .collect()
    }

    pub fn find_interdepartmental_routes(&self) -> Vec<&Route> {
        self.routes
            .iter()
            .filter(|route| route.subtipo == RouteSubtype::Interdepartmental)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_departments() {
        let loader = DataLoader::new("./data");
        let departments = loader.load_geojson::<Department>("LIM DEPARTAMENTALES.geojson");
        assert!(departments.is_ok());
    }

    #[test]
    fn test_load_stops() {
        let loader = DataLoader::new("./data");
        let stops = loader.load_geojson::<BusStop>("Paradas Transporte Colectivo AMSS.geojson");
        assert!(stops.is_ok());
    }

    #[test]
    fn test_load_routes() {
        let loader = DataLoader::new("./data");
        let routes = loader.load_geojson::<Route>("Rutas Interdepartamentales.geojson");
        assert!(routes.is_ok());
    }
}
