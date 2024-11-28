use serde::{Deserialize, Serialize};
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::BufReader;
use std::path::Path;
use tantivy::{schema::*, Document, Index};
use tracing::{info, error, instrument};
use serde_json::Value;
use rmp_serde::Deserializer;

#[derive(Serialize, Deserialize, Debug)]
pub struct LimDepartamentales {
    pub fcode: Option<String>,
    pub cod: Option<u32>,
    pub na2: Option<String>,
    pub nam: Option<String>,
    pub area_km: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParadaTransporte {
    pub ruta: Option<String>,
    pub parada_pgo: Option<String>,
    pub latitud: Option<f64>,
    pub longitud: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ruta {
    pub codigo_de: Option<String>,
    pub nombre_de: Option<String>,
    pub sentido: Option<String>,
    pub tipo: Option<String>,
    pub kilometro: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Geometry {
    pub geometry_type: String,
    pub coordinates: Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct Feature<T> {
    properties: T,
    geometry: Geometry,
}

#[derive(thiserror::Error, Debug)]
enum IndexError {
    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de Tantivy: {0}")]
    Tantivy(#[from] tantivy::error::TantivyError),
    #[error("Error de MessagePack: {0}")]
    MessagePack(String),
    #[error("Error en la geometría: {0}")]
    Geometry(String),
}

struct IndexFields {
    name: Field,
    tipo: Field,
    latitude: Field,
    longitude: Field,
    route_code: Field,
    description: Field,
    geometry: Field,
    route_id: Field,
    bus_id: Field,
    number_route: Field,
    fees: Field,
    first_trip: Field,
    last_trip: Field,
    frequency: Field,
    distance: Field,
}

impl IndexFields {
    fn new(schema_builder: &mut SchemaBuilder) -> Self {
        IndexFields {
            name: schema_builder.add_text_field("name", TEXT | STORED),
            tipo: schema_builder.add_text_field("tipo", TEXT | STORED),
            latitude: schema_builder.add_f64_field("latitude", FAST | STORED),
            longitude: schema_builder.add_f64_field("longitude", FAST | STORED),
            route_code: schema_builder.add_text_field("route_code", TEXT | STORED),
            description: schema_builder.add_text_field("description", TEXT | STORED),
            geometry: schema_builder.add_text_field("geometry", STORED),
            route_id: schema_builder.add_i64_field("route_id", STORED),
            bus_id: schema_builder.add_i64_field("bus_id", STORED),
            number_route: schema_builder.add_text_field("number_route", TEXT | STORED),
            fees: schema_builder.add_f64_field("fees", STORED),
            first_trip: schema_builder.add_text_field("first_trip", STORED),
            last_trip: schema_builder.add_text_field("last_trip", STORED),
            frequency: schema_builder.add_text_field("frequency", STORED),
            distance: schema_builder.add_f64_field("distance", STORED),
        }
    }
}

trait ToDocument {
    fn to_document(&self, fields: &IndexFields, geometry: &Geometry) -> Document;
}

impl<T> Feature<T> 
where
    T: ToDocument + std::fmt::Debug,
{
    fn to_document(&self, fields: &IndexFields) -> Document {
        self.properties.to_document(fields, &self.geometry)
    }
}

impl ToDocument for LimDepartamentales {
    fn to_document(&self, fields: &IndexFields, geometry: &Geometry) -> Document {
        let mut doc = Document::default();
        if let Some(name) = &self.nam {
            doc.add_text(fields.name, name);
            info!("Indexando departamento: {}", name);
        }
        if let Some(na2) = &self.na2 {
            doc.add_text(fields.description, na2);
        }
        
        // Guardar geometría completa
        let geometry_json = serde_json::json!({
            "type": geometry.geometry_type,
            "coordinates": geometry.coordinates
        });
        doc.add_text(fields.geometry, geometry_json.to_string());
        
        // Extraer primer punto para indexación espacial
        if let Some(coords) = geometry.coordinates.as_array() {
            if !coords.is_empty() {
                if let Some(first_point) = coords[0].as_array() {
                    if let (Some(lon), Some(lat)) = (first_point[0].as_f64(), first_point[1].as_f64()) {
                        doc.add_f64(fields.longitude, lon);
                        doc.add_f64(fields.latitude, lat);
                        info!("Coordenadas centrales: lon={}, lat={}", lon, lat);
                    }
                }
            }
        }
        
        doc.add_text(fields.tipo, "departamento");
        doc
    }
}

impl ToDocument for ParadaTransporte {
    fn to_document(&self, fields: &IndexFields, geometry: &Geometry) -> Document {
        let mut doc = Document::default();
        if let Some(parada) = &self.parada_pgo {
            doc.add_text(fields.name, parada);
            doc.add_text(fields.number_route, parada);
        }
        if let Some(ruta) = &self.ruta {
            doc.add_text(fields.route_code, ruta);
        }
        if let Some(lat) = self.latitud {
            doc.add_f64(fields.latitude, lat);
        }
        if let Some(lon) = self.longitud {
            doc.add_f64(fields.longitude, lon);
        }
        
        let geometry_json = serde_json::json!({
            "type": geometry.geometry_type,
            "coordinates": geometry.coordinates
        });
        doc.add_text(fields.geometry, geometry_json.to_string());
        
        doc.add_text(fields.tipo, "parada");
        doc
    }
}

impl ToDocument for Ruta {
    fn to_document(&self, fields: &IndexFields, geometry: &Geometry) -> Document {
        let mut doc = Document::default();
        if let Some(nombre) = &self.nombre_de {
            doc.add_text(fields.name, nombre);
        }
        if let Some(codigo) = &self.codigo_de {
            doc.add_text(fields.route_code, codigo);
            doc.add_text(fields.number_route, codigo);
        }
        if let Some(tipo) = &self.tipo {
            doc.add_text(fields.tipo, tipo);
        }
        if let Some(km) = self.kilometro {
            doc.add_f64(fields.distance, km);
        }
        
        let geometry_json = serde_json::json!({
            "type": geometry.geometry_type,
            "coordinates": geometry.coordinates
        });
        doc.add_text(fields.geometry, geometry_json.to_string());
        
        // Extraer primer punto para indexación espacial
        if let Some(coords) = geometry.coordinates.as_array() {
            if !coords.is_empty() {
                if let Some(first_point) = coords[0].as_array() {
                    if let (Some(lon), Some(lat)) = (first_point[0].as_f64(), first_point[1].as_f64()) {
                        doc.add_f64(fields.longitude, lon);
                        doc.add_f64(fields.latitude, lat);
                    }
                }
            }
        }
        
        doc
    }
}

#[instrument(skip(reader))]
fn read_features<T>(reader: BufReader<File>) -> Result<Vec<Feature<T>>, IndexError>
where
    T: for<'de> Deserialize<'de> + std::fmt::Debug,
{
    let mut deserializer = Deserializer::new(reader);
    
    let count: u32 = Deserialize::deserialize(&mut deserializer)
        .map_err(|e| IndexError::MessagePack(format!("Error leyendo contador: {}", e)))?;
    
    info!("Esperando leer {} features", count);
    let mut features = Vec::with_capacity(count as usize);
    
    for i in 0..count {
        let feature: Feature<T> = Deserialize::deserialize(&mut deserializer)
            .map_err(|e| IndexError::MessagePack(format!("Error leyendo feature {}: {}", i, e)))?;
        features.push(feature);
    }
    
    Ok(features)
}

fn create_or_open_index(index_path: &Path, schema: Schema) -> Result<Index, IndexError> {
    if index_path.exists() {
        info!("Eliminando índice existente en {:?}", index_path);
        remove_dir_all(index_path)?;
    }
    
    info!("Creando directorio para el índice");
    create_dir_all(index_path)?;
    
    info!("Creando nuevo índice");
    Ok(Index::create_in_dir(index_path, schema)?)
}

#[instrument(skip(index_writer, fields))]
fn index_feature<T>(
    feature: Feature<T>,
    index_writer: &mut tantivy::IndexWriter,
    fields: &IndexFields,
) -> Result<(), IndexError>
where
    T: ToDocument + std::fmt::Debug,
{
    let doc = feature.to_document(fields);
    index_writer.add_document(doc)?;
    Ok(())
}

#[instrument]
fn main() -> Result<(), IndexError> {
    tracing_subscriber::fmt::init();
    
    info!("Iniciando creación del índice");
    
    let mut schema_builder = Schema::builder();
    let fields = IndexFields::new(&mut schema_builder);
    let schema = schema_builder.build();
    
    let index_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("index");

    info!("Creando índice en: {:?}", index_path);
    
    let index = create_or_open_index(&index_path, schema)?;
    let mut index_writer = index.writer(200_000_000)?;
    
    let files = vec![
        ("../../data/LIM DEPARTAMENTALES.bin", "LimDepartamentales"),
        ("../../data/Paradas Transporte Colectivo AMSS.bin", "ParadaTransporte"),
        ("../../data/Rutas Interdepartamentales.bin", "Ruta"),
        ("../../data/Rutas Interurbanas.bin", "Ruta"),
        ("../../data/Rutas Urbanas.bin", "Ruta"),
    ];

    for (file_path, tipo) in files {
        info!("Procesando archivo: {}", file_path);
        
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        
        match tipo {
            "LimDepartamentales" => {
                let features = read_features::<LimDepartamentales>(reader)?;
                info!("Leídos {} features", features.len());
                for feature in features {
                    index_feature(feature, &mut index_writer, &fields)?;
                }
            },
            "ParadaTransporte" => {
                let features = read_features::<ParadaTransporte>(reader)?;
                info!("Leídos {} features", features.len());
                for feature in features {
                    index_feature(feature, &mut index_writer, &fields)?;
                }
            },
            "Ruta" => {
                let features = read_features::<Ruta>(reader)?;
                info!("Leídos {} features", features.len());
                for feature in features {
                    index_feature(feature, &mut index_writer, &fields)?;
                }
            },
            _ => {
                error!("Tipo desconocido: {}", tipo);
                continue;
            }
        }
        
        info!("Completado: {}", file_path);
    }
    
    index_writer.commit()?;
    info!("Índice creado exitosamente");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_conversion() {
        let mut schema_builder = Schema::builder();
        let fields = IndexFields::new(&mut schema_builder);
        
        let lim = LimDepartamentales {
            fcode: Some("TEST".to_string()),
            cod: Some(1),
            na2: Some("TEST".to_string()),
            nam: Some("Test Name".to_string()),
            area_km: Some(100.0),
        };
        
        let geometry = Geometry {
            geometry_type: "Point".to_string(),
            coordinates: serde_json::json!([[-89.2, 13.7]]),
        };
        
        let doc = lim.to_document(&fields, &geometry);
        assert!(doc.get_first(fields.name).is_some());
    }
}