use serde::{Deserialize, Serialize};
use geo_types::{Point, LineString, MultiPolygon};

// Tipos para los archivos GeoJSON originales
#[derive(Debug, Clone, Serialize, Deserialize,PartialEq)]
pub struct Department {
    pub fcode: String,
    pub cod: i32,
    pub na2: String,     // nombre del departamento
    pub nam: String,
    pub area_km: f64,
    pub geometry: MultiPolygon<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusStop {
    pub route: String,           // código de ruta
    pub parada_pgo: String,     // nombre de parada
    pub latitud: f64,
    pub longitud: f64,
    pub nam: String,            // departamento
    pub geometry: Point<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Route {
    #[serde(rename = "Código_de")]
    pub codigo_de: String,      // código completo
    #[serde(rename = "Nombre_de_")]
    pub nombre_de: String,      // código corto amigable (ej: "202-A")
    pub sentido: RouteDirection,
    #[serde(rename = "TIPO")]
    pub tipo: RouteType,
    #[serde(rename = "SUBTIPO")]
    pub subtipo: RouteSubtype,
    #[serde(rename = "DEPARTAMEN")]
    pub departamento: String,
    #[serde(rename = "Kilómetro")]
    pub kilometro: String,
    pub geometry: LineString<f64>,
}

// Enums normalizados para los valores fijos
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum RouteDirection {
    #[serde(rename = "IDA")]
    #[serde(alias = "Ida")]
    Outbound,
    #[serde(rename = "REGRESO")]
    #[serde(alias = "Regreso")]
    Return,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteType {
    #[serde(rename = "POR AUTOBUS")]
    Bus,
    #[serde(rename = "POR MICROBUS")]
    #[serde(alias = "POR MICROBUSES")]
    Microbus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RouteSubtype {
    #[serde(rename = "INTERDEPARTAMENTAL")]
    Interdepartmental,
    #[serde(rename = "INTERURBANO")]
    #[serde(alias = "INTERURBANA")]
    Interurban,
    #[serde(rename = "URBANO")]
    #[serde(alias = "URBANA")]
    Urban,
}

// Estructura para mantener los mappings normalizados
pub struct NormalizedValues;

impl NormalizedValues {
    pub const TIPO_BUS: &'static [(&'static str, RouteType)] = &[
        ("POR AUTOBUS", RouteType::Bus),
        ("POR MICROBUS", RouteType::Microbus),
        ("POR MICROBUSES", RouteType::Microbus),
    ];
    
    pub const SENTIDO: &'static [(&'static str, RouteDirection)] = &[
        ("IDA", RouteDirection::Outbound),
        ("Ida", RouteDirection::Outbound),
        ("REGRESO", RouteDirection::Return),
        ("Regreso", RouteDirection::Return),
    ];

    pub const SUBTIPO: &'static [(&'static str, RouteSubtype)] = &[
        ("INTERDEPARTAMENTAL", RouteSubtype::Interdepartmental),
        ("URBANO", RouteSubtype::Urban),
        ("URBANA", RouteSubtype::Urban),
        ("INTERURBANO", RouteSubtype::Interurban),
        ("INTERURBANA", RouteSubtype::Interurban),
    ];
}

// Tipos para la planificación de rutas
#[derive(Debug, Clone, PartialEq)]
pub enum TransferType {
    Direct,    // Misma parada
    Near,      // <= 500m
    Proximate, // <= 1km
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferPoint {
    pub location: Point<f64>,
    pub bus_stop: Option<BusStop>,    // Puede no haber parada registrada
    pub distance_to_route: f64,
    pub transfer_type: TransferType,
    pub from_route: String,  // código de ruta origen
    pub to_route: String,    // código de ruta destino
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteSegment {
    pub route: Route,
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
    pub max_route_distance: f64,     // 5km para encontrar rutas cercanas
    pub max_transfer_distance: f64,  // 1km para transbordos próximos
    pub max_transfers: i32,          // 10 transbordos máximo
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