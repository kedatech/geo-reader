use geo::Coord;
use serde::{Deserialize, Serialize};
use geo_types::{Point, MultiPolygon, Polygon, LineString};
use serde::de::{self, Deserializer, Visitor, SeqAccess};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoJsonFeatureCollection<T> {
    pub r#type: String,
    pub name: String,
    pub crs: GeoJsonCrs,
    pub features: Vec<GeoJsonFeature<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoJsonCrs {
    pub r#type: String,
    pub properties: GeoJsonCrsProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoJsonCrsProperties {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoJsonFeature<T> {
    pub r#type: String,
    pub properties: T,
    pub geometry: GeoJsonGeometry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum GeoJsonGeometry {
    Point { coordinates: Vec<f64> },
    LineString { coordinates: Vec<Vec<f64>> },
    Polygon { coordinates: Vec<Vec<Vec<f64>>> },
    MultiPolygon { coordinates: Vec<Vec<Vec<Vec<f64>>>> },
    MultiLineString { coordinates: Vec<Vec<Vec<f64>>> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepartmentProperties {
    #[serde(rename = "FCODE")]
    pub fcode: Option<String>,
    #[serde(rename = "COD")]
    pub cod: Option<i32>,
    #[serde(rename = "NA2")]
    pub na2: Option<String>,
    #[serde(rename = "NA3")]
    pub na3: Option<String>,
    #[serde(rename = "NAM")]
    pub nam: String,
    #[serde(rename = "AREA_KM")]
    pub area_km: Option<f64>,
    #[serde(rename = "PERIMETRO")]
    pub perimetro: Option<f64>,
    #[serde(rename = "SHAPE_LENG")]
    pub shape_leng: Option<f64>,
    #[serde(rename = "SHAPE_AREA")]
    pub shape_area: Option<f64>,
}
pub type DepartmentFeatureCollection = GeoJsonFeatureCollection<DepartmentProperties>;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusStopProperties {
    #[serde(rename = "FID_L0Coor")]
    pub fid_l0coor: Option<i32>,
    #[serde(rename = "Ruta")]
    pub ruta: Option<String>,
    #[serde(rename = "Cod")]
    pub cod: Option<String>,
    #[serde(rename = "Coordenada")]
    pub coordenada: Option<String>,
    #[serde(rename = "Latitud")]
    pub latitud: Option<f64>,
    #[serde(rename = "Longitud")]
    pub longitud: Option<f64>,
    #[serde(rename = "FCODE")]
    pub fcode: Option<String>,
    #[serde(rename = "NA2")]
    pub na2: Option<String>,
    #[serde(rename = "NA3")]
    pub na3: Option<String>,
    #[serde(rename = "NAM")]
    pub nam: Option<String>,
}

pub type BusStopFeatureCollection = GeoJsonFeatureCollection<BusStopProperties>;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteProperties {
    #[serde(rename = "Código_de")]
    pub codigo_de: Option<String>,
    #[serde(rename = "Nombre_de_")]
    pub nombre_de: Option<String>,
    #[serde(rename = "SENTIDO")]
    pub sentido: Option<String>,
    #[serde(rename = "TIPO")]
    pub tipo: Option<String>,
    #[serde(rename = "SUBTIPO")]
    pub subtipo: Option<String>,
    #[serde(rename = "DEPARTAMEN")]
    pub departamento: Option<String>,
    #[serde(rename = "Kilómetro")]
    pub kilometro: Option<String>,
    #[serde(rename = "CANTIDAD_D")]
    pub cantidad_d: Option<i32>,
    #[serde(rename = "Shape_Leng")]
    pub shape_leng: Option<f64>,
}

pub type RouteFeatureCollection = GeoJsonFeatureCollection<RouteProperties>;

// Tipos para la planificación de rutas
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransferType {
    Direct,    // Misma parada
    Near,      // <= 500m
    Proximate, // <= 1km
}


mod point_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use geo_types::Point;

    pub fn serialize<S>(point: &Point<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (point.x(), point.y()).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Point<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = <(f64, f64)>::deserialize(deserializer)?;
        Ok(Point::new(x, y))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferPoint {
    #[serde(with = "point_serde")]
    pub location: Point<f64>,
    pub bus_stop: Option<BusStopProperties>,
    pub distance_to_route: f64,
    pub transfer_type: TransferType,
    pub from_route: String,
    pub to_route: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteSegment {
    pub route: RouteProperties,
    pub transfer_point: TransferPoint,
    pub transfer_type: TransferType,
    pub segment_distance: f64,
}

#[derive(Debug, Clone)]
pub struct RoutePlan {
    pub routes: Vec<RouteSegment>,
    pub total_distance: f64,
    pub transfers_count: i32,
    pub is_interdepartmental: bool,
}

// Errores que pueden ocurrir durante la planificación
#[derive(Debug, thiserror::Error)]
pub enum PlanningError {
    #[error("Point outside El Salvador")]
    OutsideCountry,
    #[error("No routes found within maximum distance")]
    NoRoutesFound,
    #[error("No valid path found between points")]
    NoValidPath,
    #[error("Maximum transfers exceeded")]
    MaxTransfersExceeded,
    #[error("Invalid coordinates")]
    InvalidCoordinates,
    #[error("Data error: {0}")]
    DataError(String),
    #[error("Geospatial operation error: {0}")]
    GeoError(String),
}

// Input para la función de planificación
#[derive(Debug)]
pub struct RouteRequest {
    pub origin: Point<f64>,
    pub destination: Point<f64>,
    pub max_route_distance: f64,
    pub max_transfer_distance: f64,
    pub max_transfers: i32,
}

// Implementaciones de métodos útiles
impl RoutePlan {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            total_distance: 0.0,
            transfers_count: 0,
            is_interdepartmental: false,
        }
    }

    pub fn add_segment(&mut self, segment: RouteSegment) {
        self.total_distance += segment.segment_distance;
        self.routes.push(segment);
        self.transfers_count = self.routes.len() as i32 - 1;
    }
}