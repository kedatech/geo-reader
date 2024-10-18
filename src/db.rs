use tokio_postgres::{Client, NoTls, Error};
use std::env;
use dotenv::dotenv;

/// Establece la conexión con la base de datos PostgreSQL.
pub async fn connect_to_db() -> Result<Client, Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL debe estar configurada en .env");

    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Error en la conexión: {}", e);
        }
    });

    Ok(client)
}
