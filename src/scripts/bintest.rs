use std::fs::File;
use rmp_serde::decode;
use serde_json::to_string_pretty;

fn main() {
    let file_path = "../../data/LIM DEPARTAMENTALES.bin";
    let bin_file = File::open(file_path).expect("No se pudo abrir el archivo binario");

    let data: serde_json::Value = decode::from_read(bin_file).expect("Error al leer MessagePack");
    let json_string: String = to_string_pretty(&data).expect("Error al convertir a JSON");
    println!("{}", json_string);
}
