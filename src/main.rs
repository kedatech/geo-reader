use geo_reader::find_places_by_name;
use tokio;

#[tokio::main]
async fn main() {
    match find_places_by_name("park").await {
        Ok(places) => {
            for place in places {
                println!("Lugar encontrado: {}", place);
            }
        }
        Err(e) => eprintln!("Error al buscar lugares: {}", e),
    }
}
