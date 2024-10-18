use geo_reader::{find_places_by_name, find_nearby_places, find_by_coordinates};
use tokio;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    loop {
        println!("Geo Reader Console App");
        println!("1. Find places by name");
        println!("2. Find nearby places");
        println!("3. Find by coordinates");
        println!("4. Exit");
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
                let (lat, lon, radius) = read_coordinates_and_radius();
                match find_nearby_places(lat, lon, radius).await {
                    Ok(places) => {
                        for place in places {
                            println!("Nearby: {}", place);
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "3" => {
                let (lat, lon, _) = read_coordinates_and_radius();
                match find_by_coordinates(lat, lon).await {
                    Ok(places) => {
                        for place in places {
                            println!("Exact match: {}", place);
                        }
                    }
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            "4" => break,
            _ => println!("Invalid choice, please try again."),
        }
    }
}

fn read_coordinates_and_radius() -> (f64, f64, f64) {
    print!("Enter latitude: ");
    io::stdout().flush().unwrap();
    let mut lat = String::new();
    io::stdin().read_line(&mut lat).unwrap();
    let lat: f64 = lat.trim().parse().unwrap();

    print!("Enter longitude: ");
    io::stdout().flush().unwrap();
    let mut lon = String::new();
    io::stdin().read_line(&mut lon).unwrap();
    let lon: f64 = lon.trim().parse().unwrap();

    print!("Enter radius (in meters): ");
    io::stdout().flush().unwrap();
    let mut radius = String::new();
    io::stdin().read_line(&mut radius).unwrap();
    let radius: f64 = radius.trim().parse().unwrap();

    (lat, lon, radius)
}
