use std::sync::{ Arc, Mutex };
use tokio::sync::mpsc;
use crate::Capacitor;

pub async fn handle_blocks_message(capacitor_ins: Arc<Mutex<Capacitor>>, mut stream: mpsc::Receiver<near_indexer::StreamerMessage>) {    
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