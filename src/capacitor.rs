use near_indexer::near_primitives::{
    views::{
        ExecutionOutcomeWithIdView, 
        ExecutionOutcomeView,
        ExecutionStatusView
    },
};
use tokio::stream::StreamExt;
use mongodb::{ Client, Database, options::{ UpdateOptions } };
use bson::{ Bson, doc };
use serde_json::{ Value };
use std::vec::Vec;
use std::convert::TryInto;
use chrono::{ Utc };

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
        let mut cursor = allowed_collection.find(None, None).await.unwrap();

        while let Some(doc) = cursor.next().await {
            let allowed_doc = doc.unwrap();
            let account_id = allowed_doc.get("account_id").and_then(Bson::as_str).unwrap();

            if !self.allowed_ids.contains(&account_id.to_string()) {
                self.allowed_ids.push(account_id.to_string());
            }
        }

        println!("📝 Listening for the following contracts: {:?}", self.allowed_ids);
    }

    pub async fn add_account_id(&mut self, account_id: String) {
        let allowed_collection = self.capacitor_db.collection("allowed_account_ids");
        let doc = doc! {
            "account_id": account_id.to_string(),
        };

        let mut cursor = allowed_collection.find(doc.clone(), None).await.unwrap();

        while let Some(doc) = cursor.next().await {
            let allowed_doc = doc.unwrap();
            let doc_account_id = allowed_doc.get("account_id").and_then(Bson::as_str).unwrap();

            if doc_account_id == account_id {
                return ();
            }
        }

        allowed_collection.insert_one(doc.clone(), None).await.unwrap();
        self.allowed_ids.push(account_id.to_string());
    }

    pub fn is_valid_receipt(&self, execution_outcome: &ExecutionOutcomeWithIdView) -> bool {
        match &execution_outcome.outcome.status {
            ExecutionStatusView::SuccessValue(_) => (),
            ExecutionStatusView::SuccessReceiptId(_) => (),
            _ => return false
        }

        self.allowed_ids.contains(&execution_outcome.outcome.executor_id)
    }

    pub async fn process_outcome(&self, outcome: ExecutionOutcomeView) {
        println!("🤖 Processing logs for {}", &outcome.executor_id);
        let normalized_database_name = outcome.executor_id.replace(".", "_");
        let database = self.database_client.database(&normalized_database_name);

        for log in outcome.logs {
            let parsed_logs: Value = serde_json::from_str(log.as_str()).unwrap();
            let log_type = &parsed_logs["type"].as_str().unwrap().to_string();
            let cap_id = &parsed_logs["cap_id"].as_str().unwrap_or("None").to_string();
            let action_type = &parsed_logs["action"].as_str().unwrap_or("write").to_string();
            let collection = database.collection(log_type);

            let stringified_params = serde_json::to_string(&parsed_logs["params"]).unwrap();
            let parsed_params: Value = serde_json::from_str(&stringified_params).unwrap();

            let data: Bson = parsed_params.try_into().unwrap();
            let mut doc = bson::to_document(&data).unwrap();
            let creation_date_time = Utc::now();
            
            doc.insert("cap_creation_date", creation_date_time);
            doc.insert("cap_id", cap_id);

            if action_type == "update" {
                let options = UpdateOptions::builder().upsert(true).build();
                let update_doc = doc! {
                    "$set": data,
                };

                collection.update_one(doc!{ "cap_id": cap_id }, update_doc, options)
                    .await
                    .unwrap_or_else(|_| panic!("🛑 Database could not insert document"));
            } else {
                collection.insert_one(doc, None)
                    .await
                    .unwrap_or_else(|_| panic!("🛑 Database could not insert document"));
            }
        }
    }
}
