use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[typetag::serde]
pub trait TransactionData: erased_serde::Serialize + Debug + Send + Sync + 'static {
    fn clone_dyn(&self) -> Box<dyn TransactionData>;
}

impl Clone for Box<dyn TransactionData> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub data: Box<dyn TransactionData>,
    pub timestamp: u128,
}

impl Transaction {
    pub fn new<TData: TransactionData>(from: String, to: String, data: TData) -> Transaction {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to calculate the timestamp")
            .as_nanos();

        Transaction {
            from,
            to,
            data: Box::new(data),
            timestamp,
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize to JSON")
    }
}
