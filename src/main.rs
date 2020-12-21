use actix;
use clap::Clap;

use tokio::sync::mpsc;

use mongodb::{Client, options::ClientOptions };
use configs::{init_logging, Opts, SubCommand};


mod configs;
// mod db;
mod capacitor;
use capacitor::Capacitor;

use near_indexer;

pub async fn db_connect() -> Client {
    let connection_string = "mongodb://root:root@localhost:27017";
    let client_options = ClientOptions::parse(connection_string).await.unwrap_or_else(|_| panic!("Could not connect to database"));
    let client = Client::with_options(client_options).unwrap_or_else(|_| panic!("Malformed client options"));
    
    println!("🔗 Connected to database");

    return client;
}

async fn handle_blocks_message(mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {
    let database_client = db_connect().await;
    let mut capacitor_ins = Capacitor::new(database_client, vec!["protocol.test.near".to_string()]);

    capacitor_ins.load().await;

    while let Some(block) = stream.recv().await {
        println!("⛏ Block height {:?}", block.block.header.height);

        for tx_res in block.receipt_execution_outcomes {
            let (_, outcome) = tx_res;

            println!("is valid? {:?}", &outcome.execution_outcome.outcome.executor_id);

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