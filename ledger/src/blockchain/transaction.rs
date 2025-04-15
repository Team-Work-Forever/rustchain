use std::fmt::Debug;

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
}

impl<TData> Transaction<TData>
where
    TData: TransactionData,
{
    pub fn new(from: String, to: String, data: TData) -> Transaction<TData> {
        Transaction { from, to, data }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize to JSON")
    }
}
