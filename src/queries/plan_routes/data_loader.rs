use geo_types::{MultiPolygon, Polygon, Coord};
use serde::de::DeserializeOwned;
use log::{debug, info, error};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};


use super::_structs::{BusStopFeatureCollection, BusStopProperties, DepartmentFeatureCollection, GeoJsonCrs, GeoJsonCrsProperties, GeoJsonFeature, RouteFeatureCollection, RouteProperties};

pub struct DataLoader {
    data_dir: PathBuf,
    departments: DepartmentFeatureCollection,
    bus_stops: BusStopFeatureCollection,
    routes: RouteFeatureCollection,
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
            departments: DepartmentFeatureCollection {
                r#type: String::new(),
                name: String::new(),
                crs: GeoJsonCrs {
                    r#type: String::new(),
                    properties: GeoJsonCrsProperties {
                        name: String::new(),
                    },
                },
                features: Vec::new(),
            },
            bus_stops: BusStopFeatureCollection {
                r#type: String::new(),
                name: String::new(),
                crs: GeoJsonCrs {
                    r#type: String::new(),
                    properties: GeoJsonCrsProperties {
                        name: String::new(),
                    },
                },
                features: Vec::new(),
            },
            routes: RouteFeatureCollection {
                r#type: String::new(),
                name: String::new(),
                crs: GeoJsonCrs {
                    r#type: String::new(),
                    properties: GeoJsonCrsProperties {
                        name: String::new(),
                    },
                },
                features: Vec::new(),
            },
        }
    }

    /// Carga todos los datos necesarios
    pub fn load_all(&mut self) -> Result<(), LoaderError> {
        self.departments = self.load_geojson("LIM DEPARTAMENTALES.geojson")?;
        self.bus_stops = self.load_geojson("Paradas Transporte Colectivo AMSS.geojson")?;
    
        let route_files = [
            "Rutas Interdepartamentales.geojson",
            "Rutas Interurbanas.geojson",
            "Rutas Urbanas.geojson",
        ];
    
        self.routes = route_files
        .iter()
        .map(|&file| -> Result<RouteFeatureCollection, LoaderError> {
            self.load_geojson(file)
        })
        .try_fold(
            RouteFeatureCollection {
                r#type: String::new(),
                name: String::new(),
                crs: GeoJsonCrs {
                    r#type: String::new(),
                    properties: GeoJsonCrsProperties {
                        name: String::new(),
                    },
                },
                features: Vec::new(),
            },
            |mut acc, collection| -> Result<RouteFeatureCollection, LoaderError> {
                let collection = collection?;
                acc.features.extend(collection.features);
                Ok(acc)
            },
        )?;
    
        Ok(())
    }

    /// Carga un archivo GeoJSON específico
    fn load_geojson<T: DeserializeOwned>(&self, filename: &str) -> Result<T, LoaderError> {
        let file_path = self.data_dir.join(filename);
        info!("Loading {}", file_path.display());
    
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);
    
        // Agregar debug logging
        let raw_json: serde_json::Value = serde_json::from_reader(reader)?;
        debug!("Raw JSON structure: {}", raw_json);
    
        // Validar estructura básica
        if !raw_json.is_object() || !raw_json.get("type").is_some() {
            return Err(LoaderError::GeoJson("Invalid GeoJSON structure".into()));
        }
    
        serde_json::from_value(raw_json)
            .map_err(|e| {
                error!("Failed to parse GeoJSON from {}: {}", filename, e);
                LoaderError::Json(e)
            })
    }
    
    // Getters para acceder a los datos cargados
    pub fn departments(&self) -> &DepartmentFeatureCollection {
        &self.departments
    }

    pub fn bus_stops(&self) -> &BusStopFeatureCollection {
        &self.bus_stops
    }

    pub fn routes(&self) -> &RouteFeatureCollection {
        &self.routes
    }

    // Métodos de utilidad para consultas comunes
    pub fn find_routes_by_department(&self, department: &str) -> Vec<&GeoJsonFeature<RouteProperties>> {
        self.routes
            .features
            .iter()
            .filter(|feature| feature.properties.departamento == department)
            .collect()
    }

    pub fn find_stops_by_route(&self, route_code: &str) -> Vec<&BusStopProperties> {
        self.bus_stops
            .features
            .iter()
            .map(|feature| &feature.properties)
            .filter(|stop| stop.ruta == route_code)
            .collect()
    }

    pub fn find_interdepartmental_routes(&self) -> Vec<&RouteProperties> {
        self.routes
            .features
            .iter()
            .map(|feature| &feature.properties)
            .filter(|route| route.subtipo == "INTERDEPARTAMENTAL")
            .collect()
    }
}