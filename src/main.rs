use actix;
use clap::Clap;

use tokio::sync::mpsc;

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

async fn handle_blocks_message(capacitor_ins: &mut Capacitor, mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    // let database_client = db_connect().await;
    // let mut capacitor_ins = Capacitor::new(database_client, vec![]);
    // capacitor_ins.load().await;
    
    while let Some(block) = stream.recv().await {
        println!("⛏ Block height {:?}", block.block.header.height);

        for tx_res in block.receipt_execution_outcomes {
            let (_, outcome) = tx_res;

            if !capacitor_ins.is_valid_receipt(&outcome.execution_outcome) {
                continue;
            }

            capacitor_ins.process_outcome(outcome.execution_outcome.outcome).await;
        }
    }
}

async fn handle_post_add_account() -> impl Responder {
    return "Hello World";
}

async fn start_http_server(capacitor_ins: &mut Capacitor) {
    capacitor_ins.add_account_id("blah".to_string());
    HttpServer::new(|| {
        App::new()
            .route("/config/add_account", web::post().to(handle_post_add_account))
    })
    .bind("127.0.0.1:3000").expect("Could not run http server on that port")
    .run()
    .await.expect("Failed to start http server");
}

async fn start_process(stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    let database_client = db_connect().await;
    let mut capacitor_ins = Capacitor::new(database_client, vec![]);
    let borrowed_capacitor = &mut capacitor_ins;
    capacitor_ins.load().await;

    capacitor_ins.add_account_id("tralala".to_string()).await;

    actix::spawn(handle_blocks_message(borrowed_capacitor, stream));
    start_http_server(borrowed_capacitor).await;
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