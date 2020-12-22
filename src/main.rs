use actix;
use clap::Clap;

use tokio::sync::mpsc;

use mongodb::{Client, options::ClientOptions };
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

async fn handle_blocks_message(mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    let database_client = db_connect().await;
    let mut capacitor_ins = Capacitor::new(database_client, vec![]);

    capacitor_ins.add_account_id("protocol.test.near".to_string()).await;
    capacitor_ins.load().await;

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
            actix::spawn(handle_blocks_message(stream));
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