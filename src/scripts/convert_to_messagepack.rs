use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use tracing::{error, info, instrument, warn};

// Estructuras de datos principales
#[derive(Debug, Serialize, Deserialize)]
pub struct LimDepartamentales {
    pub fcode: Option<String>,
    pub cod: Option<u32>,
    pub na2: Option<String>,
    pub nam: Option<String>,
    pub area_km: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParadaTransporte {
    pub ruta: Option<String>,
    pub parada_pgo: Option<String>,
    pub latitud: Option<f64>,
    pub longitud: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ruta {
    pub codigo_de: Option<String>,
    pub nombre_de: Option<String>,
    pub sentido: Option<String>,
    pub tipo: Option<String>,
    pub kilometro: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Geometry {
    geometry_type: String,
    coordinates: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct Feature<T> {
    properties: T,
    geometry: Geometry,
}

// Configuración
#[derive(Debug, Deserialize)]
struct FileConfig {
    input_path: String,
    type_name: String,
    output_path: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    files: Vec<FileConfig>,
}

// Manejo de errores personalizado
#[derive(Debug, thiserror::Error)]
enum ConversionError {
    #[error("Error de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Error de JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Error de MessagePack: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),
    #[error("No hay features en el archivo")]
    NoFeatures,
    #[error("Error de validación: {0}")]
    Validation(String),
}

// Trait para validación
trait Validate {
    fn validate(&self) -> Result<(), ConversionError>;
}

// Implementaciones de validación
impl Validate for LimDepartamentales {
    fn validate(&self) -> Result<(), ConversionError> {
        if let Some(area) = self.area_km {
            if area <= 0.0 {
                return Err(ConversionError::Validation("Área debe ser positiva".into()));
            }
        }
        Ok(())
    }
}

impl Validate for ParadaTransporte {
    fn validate(&self) -> Result<(), ConversionError> {
        if let (Some(lat), Some(lon)) = (self.latitud, self.longitud) {
            if lat < -90.0 || lat > 90.0 || lon < -180.0 || lon > 180.0 {
                return Err(ConversionError::Validation(
                    "Coordenadas geográficas inválidas".into(),
                ));
            }
        }
        Ok(())
    }
}

impl Validate for Ruta {
    fn validate(&self) -> Result<(), ConversionError> {
        if let Some(km) = self.kilometro {
            if km < 0.0 {
                return Err(ConversionError::Validation(
                    "Kilómetros deben ser positivos".into(),
                ));
            }
        }
        Ok(())
    }
}

#[instrument(skip(input_path, output_path))]
fn convert_geojson_to_bin<T>(input_path: &str, output_path: &str) -> Result<(), ConversionError>
where
    T: for<'de> Deserialize<'de> + Serialize + std::fmt::Debug + Validate,
{
    info!("Iniciando conversión de {}", input_path);

    // Crear directorio de salida si no existe
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = BufReader::new(File::open(input_path)?);
    let mut deserializer = serde_json::Deserializer::from_reader(file);
    let json_data: Value = Value::deserialize(&mut deserializer)?;

    let features = json_data["features"]
        .as_array()
        .ok_or(ConversionError::NoFeatures)?;

    let writer = BufWriter::new(File::create(output_path)?);
    let mut encoder = rmp_serde::encode::Serializer::new(writer);

    // Escribir el número de features como encabezado
    features.len().serialize(&mut encoder)?;

    for (index, feature) in features.iter().enumerate() {
        let geometry = Geometry {
            geometry_type: feature["geometry"]["type"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
            coordinates: feature["geometry"]["coordinates"].clone(),
        };

        let properties: T = serde_json::from_value(feature["properties"].clone())?;
        
        // Validar los datos antes de escribir
        properties.validate()?;

        let feature_obj = Feature {
            properties,
            geometry,
        };

        feature_obj.serialize(&mut encoder)?;

        if (index + 1) % 1000 == 0 {
            info!("Procesados {} features", index + 1);
        }
    }

    info!("Conversión completada exitosamente: {}", output_path);
    Ok(())
}

fn process_file(config: &FileConfig) -> Result<(), ConversionError> {
    info!("Procesando archivo: {}", config.input_path);
    
    match config.type_name.as_str() {
        "LimDepartamentales" => convert_geojson_to_bin::<LimDepartamentales>(&config.input_path, &config.output_path),
        "ParadaTransporte" => convert_geojson_to_bin::<ParadaTransporte>(&config.input_path, &config.output_path),
        "Ruta" => convert_geojson_to_bin::<Ruta>(&config.input_path, &config.output_path),
        _ => {
            error!("Tipo desconocido: {}", config.type_name);
            Err(ConversionError::Validation(format!(
                "Tipo desconocido: {}",
                config.type_name
            )))
        }
    }
}

fn main() {
    // Inicializar el sistema de logging
    tracing_subscriber::fmt::init();

    // Cargar configuración
    let config: Config = match toml::from_str(include_str!("config.toml")) {
        Ok(config) => config,
        Err(e) => {
            error!("Error al cargar la configuración: {}", e);
            return;
        }
    };

    // Procesar archivos en paralelo
    let results: Vec<Result<(), ConversionError>> = config
        .files
        .par_iter()
        .map(|file_config| process_file(file_config))
        .collect();

    // Reportar resultados
    let mut success_count = 0;
    let mut error_count = 0;

    for (index, result) in results.iter().enumerate() {
        match result {
            Ok(_) => {
                success_count += 1;
                info!("Archivo {} procesado exitosamente", config.files[index].input_path);
            }
            Err(e) => {
                error_count += 1;
                error!(
                    "Error al procesar {}: {}",
                    config.files[index].input_path,
                    e
                );
            }
        }
    }

    info!(
        "Proceso completado. Éxitos: {}, Errores: {}",
        success_count, error_count
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_lim_departamentales_validation() {
        let lim = LimDepartamentales {
            fcode: Some("TEST".into()),
            cod: Some(1),
            na2: Some("TEST".into()),
            nam: Some("TEST".into()),
            area_km: Some(-1.0),
        };
        assert!(lim.validate().is_err());
    }

    #[test]
    fn test_parada_transporte_validation() {
        let parada = ParadaTransporte {
            ruta: Some("TEST".into()),
            parada_pgo: Some("TEST".into()),
            latitud: Some(91.0),
            longitud: Some(0.0),
        };
        assert!(parada.validate().is_err());
    }

    #[test]
    fn test_conversion() -> Result<(), ConversionError> {
        let input_json = r#"{
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {
                    "fcode": "TEST",
                    "cod": 1,
                    "na2": "TEST",
                    "nam": "TEST",
                    "area_km": 100.0
                },
                "geometry": {
                    "type": "Point",
                    "coordinates": [0.0, 0.0]
                }
            }]
        }"#;

        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap();

        // Escribir JSON temporal
        std::fs::write("temp.json", input_json)?;

        // Probar conversión
        convert_geojson_to_bin::<LimDepartamentales>("temp.json", temp_path)?;

        // Limpiar
        std::fs::remove_file("temp.json")?;
        Ok(())
    }
}