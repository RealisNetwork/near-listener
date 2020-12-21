use near_indexer::near_primitives::{
    views::{
        ExecutionOutcomeWithIdView, 
        ExecutionOutcomeView
    },
};
use mongodb::{ Client, Database, Cursor };
use bson::{ Bson, doc, Document };
use serde_json::{ Result, Value };
use std::vec::Vec;
use std::convert::TryInto;

pub struct Capacitor {
    capacitor_db: Database,
    database_client: Client,
    allowed_ids: Vec<String>,
}

impl Capacitor {
    pub fn new(database_client: Client, temp_allowed_ids: Vec<String>) -> Self {
        Self {
            capacitor_db: database_client.database("capacitor"),
            allowed_ids: temp_allowed_ids,
            database_client,
        }
    }

    pub async fn load(&mut self) {
        let allowed_collection = self.capacitor_db.collection("allowed_account_ids");
        let cursor = allowed_collection.find(None, None).await.unwrap();

        println!("cursor {:?}", cursor);
    }

    pub async fn add_account_id(&mut self, account_id: &String) {
        let allowed_collection = self.capacitor_db.collection("allowed_account_ids");
        let doc = doc! {
            "account_id": account_id.to_string(),
        };

        // TODO: check if the account id already exists

        allowed_collection.insert_one(doc, None).await.unwrap();
        self.allowed_ids.push(account_id.to_string());
    }

    pub fn is_valid_receipt(&self, execution_outcome: &ExecutionOutcomeWithIdView) -> bool {
        self.allowed_ids.contains(&execution_outcome.outcome.executor_id)
    }

    pub async fn process_outcome(&self, outcome: ExecutionOutcomeView) {
        let normalized_database_name = outcome.executor_id.replace(".", "_");
        let database = self.database_client.database(&normalized_database_name);

        for log in outcome.logs {
            let parsed_logs: Value = serde_json::from_str(log.as_str()).unwrap();
            let log_type = &parsed_logs["type"].as_str().unwrap().to_string();
            let collection = database.collection(log_type);

            let stringified_params = serde_json::to_string(&parsed_logs["params"]).unwrap();
            let parsed_params: Value = serde_json::from_str(&stringified_params).unwrap();

            let data: Bson = parsed_params.try_into().unwrap();
            let doc = bson::to_document(&data).unwrap();

            collection.insert_one(doc, None).await.unwrap_or_else(|_| panic!("🛑 Database could not insert document"));
        }
    }
}
