use std::{
    fmt::Debug,
    time::{SystemTime, UNIX_EPOCH},
};

use bincode::{Decode, Encode};
use serde::Serialize;

pub trait TransactionData: Clone + Serialize + Debug + Encode {}

#[derive(Clone, Serialize, Debug, Encode, Decode)]
pub struct Transaction<TData>
where
    TData: TransactionData,
{
    pub from: String,
    pub to: String,
    pub data: TData,
    pub timestamp: u128,
}

impl<TData> Transaction<TData>
where
    TData: TransactionData,
{
    pub fn new(from: String, to: String, data: TData) -> Transaction<TData> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to calculate the timestamp")
            .as_nanos();

        Transaction {
            from,
            to,
            data,
            timestamp,
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize to JSON")
    }
}
