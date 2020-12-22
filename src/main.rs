use actix;
use clap::Clap;

use tokio::sync::mpsc;

use std::sync::{Arc, Mutex};
use actix_web::{web, App, HttpServer, Responder };
use mongodb::{ Client, options::ClientOptions };
use configs::{ init_logging, Opts, SubCommand };
use dotenv::dotenv;
use std::env;

mod configs;
mod capacitor;

use capacitor::Capacitor;

use near_indexer;

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

struct AppState {
    capacitor_ins: Arc<Mutex<Capacitor>>,
}

async fn handle_blocks_message(capacitor_ins: Arc<Mutex<Capacitor>>, mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    
    while let Some(block) = stream.recv().await {
        println!("⛏ Block height {:?}", block.block.header.height);
        let capacitor_unwrapped = capacitor_ins.lock().unwrap();

        for tx_res in block.receipt_execution_outcomes {
            let (_, outcome) = tx_res;

            if !capacitor_unwrapped.is_valid_receipt(&outcome.execution_outcome) {
                continue;
            }

            capacitor_unwrapped.process_outcome(outcome.execution_outcome.outcome).await;
        }
    }
}

async fn handle_post_add_account(data: web::Data<AppState>) -> impl Responder {
    let mut capacitor_ins = data.capacitor_ins.lock().unwrap();

    capacitor_ins.add_account_id("TestTest".to_string()).await;

    return "Hello World";
}


async fn start_http_server(capacitor_ins: Arc<Mutex<Capacitor>>) {
    let state = web::Data::new(AppState {
        capacitor_ins,
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/config/add_account", web::post().to(handle_post_add_account))
    })
    .bind("127.0.0.1:3000").expect("Could not run http server on that port")
    .run()
    .await.expect("Failed to start http server");
}

async fn start_process(stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    let database_client = db_connect().await;
    let mut capacitor_ins = Capacitor::new(database_client, vec![]);
    capacitor_ins.load().await;

    let mutex_capacitor: Mutex<Capacitor> = Mutex::new(capacitor_ins);
    let wrapped_capacitor = Arc::new(mutex_capacitor);

    actix::spawn(handle_blocks_message(wrapped_capacitor.clone(), stream));
    actix::spawn(start_http_server(wrapped_capacitor.clone()));
}

fn main() {
    // We use it to automatically search the for root certificates to perform HTTPS calls
    // (sending telemetry and downloading genesis)
    println!("🚀 Starting flux capacitor");
    openssl_probe::init_ssl_cert_env_vars();
    init_logging();
    dotenv().ok();
    
    let opts: Opts = Opts::parse();
    let home_dir = opts.home_dir.unwrap_or(std::path::PathBuf::from(near_indexer::get_default_home()));
    
    match opts.subcmd {
        SubCommand::Run => {
            let indexer_config = near_indexer::IndexerConfig {
                home_dir,
                sync_mode: near_indexer::SyncModeEnum::FromInterruption,
            };
            let indexer = near_indexer::Indexer::new(indexer_config);
            let stream = indexer.streamer();

            actix::spawn(start_process(stream));

            // actix::spawn(handle_blocks_message(stream));
            indexer.start();
        }
        SubCommand::Init(config) => near_indexer::init_configs(
            &home_dir,
            config.chain_id.as_ref().map(AsRef::as_ref),
            config.account_id.as_ref().map(AsRef::as_ref),
            config.test_seed.as_ref().map(AsRef::as_ref),
            config.num_shards,
            config.fast,
            config.genesis.as_ref().map(AsRef::as_ref),
            config.download,
            config.download_genesis_url.as_ref().map(AsRef::as_ref),
        ),
    }
}