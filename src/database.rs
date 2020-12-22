use std::env;
use mongodb::{ Client, options::ClientOptions };

pub async fn db_connect() -> Client {
    let db_user = env::var("DB_USER").expect("DB_USER is required to be defined in the .env file");
    let db_password = env::var("DB_PASSWORD").expect("DB_PASSWORD is required to be defined in the .env file");
    let db_host = env::var("DB_HOST").expect("DB_HOST is required to be defined in the .env file");
    let db_port = env::var("DB_PORT").expect("DB_PORT is required to be defined in the .env file");

    let connection_string = format!("mongodb://{}:{}@{}:{}", db_user, db_password, db_host, db_port);
    let client_options = ClientOptions::parse(&connection_string).await.unwrap_or_else(|_| panic!("Could not connect to database"));
    let client = Client::with_options(client_options).unwrap_or_else(|_| panic!("Malformed client options"));
    
    println!("🔗 Connected to database");

    return client;
}
