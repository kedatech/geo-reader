use geo_reader::{find_places_by_name, find_nearby_places, find_by_coordinates, find_route_as_geojson};
use tokio;
use dotenv::dotenv;
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    // Cargar las variables desde el archivo .env
    dotenv().ok();

    loop {
        println!("Geo Reader Console App");
        println!("1. Find places by name");
        println!("2. Find nearby places");
        println!("3. Find by coordinates");
        println!("4. Find route as GeoJSON");
        println!("5. Exit");
        print!("Choose an option: ");
        io::stdout().flush().unwrap();

        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        match choice {
            "1" => {
                print!("Enter place name: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).unwrap();
                let name = name.trim();

                match find_places_by_name(name).await {
                    Ok(places) => {
                        for (name, longitude, latitude) in places {
                            println!(
                                "Found: {} (Longitude: {}, Latitude: {})",
                                name, longitude, latitude
                            );
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "2" => {
                if let Some((lat, lon, radius)) = read_coordinates_and_radius() {
                    match find_nearby_places(lat, lon, radius).await {
                        Ok(places) => {
                            for place in places {
                                println!("Nearby: {}", place);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
            "3" => {
                if let Some((lat, lon, _)) = read_coordinates_and_radius() {
                    match find_by_coordinates(lat, lon).await {
                        Ok(places) => {
                            for place in places {
                                println!("Exact match: {}", place);
                            }
                        }
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
            "4" => {
                match read_coordinates_from_env() {
                    Some((start_lat, start_lon, end_lat, end_lon)) => {
                        match find_route_as_geojson(start_lat, start_lon, end_lat, end_lon).await {
                            Ok(geojson) => println!("GeoJSON Route: {}", geojson),
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                    None => eprintln!("Error: Could not read coordinates from environment."),
                }
            }
            "5" => break,
            _ => println!("Invalid choice, please try again."),
        }
    }
}

fn read_coordinates_and_radius() -> Option<(f64, f64, f64)> {
    let lat = prompt_for_float("Enter latitude: ")?;
    let lon = prompt_for_float("Enter longitude: ")?;
    let radius = prompt_for_float("Enter radius (in meters): ")?;
    Some((lat, lon, radius))
}

fn prompt_for_float(prompt: &str) -> Option<f64> {
    print!("{}", prompt);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    match input.trim().parse::<f64>() {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("Invalid input. Please enter a valid number.");
            None
        }
    }
}

fn read_coordinates_from_env() -> Option<(f64, f64, f64, f64)> {
    let start_lat = env::var("START_LAT").ok()?.parse::<f64>().ok()?;
    let start_lon = env::var("START_LON").ok()?.parse::<f64>().ok()?;
    let end_lat = env::var("END_LAT").ok()?.parse::<f64>().ok()?;
    let end_lon = env::var("END_LON").ok()?.parse::<f64>().ok()?;

    Some((start_lat, start_lon, end_lat, end_lon))
}
