use std::{any::Any, fmt::Debug};

use chrono::Utc;
use ed25519_dalek::PUBLIC_KEY_LENGTH;
use rand::{rng, RngCore};
use serde::{Deserialize, Serialize};

use crate::kademlia::{
    secret_key::SecretPair,
    signature::{HandleSignature, Signature},
};

use super::{DoubleHasher, HashFunc};

type PublicKey = [u8; PUBLIC_KEY_LENGTH];

#[typetag::serde]
pub trait TransactionData: erased_serde::Serialize + Debug + Send + Sync + 'static {
    fn get_hash(&self) -> Option<PublicKey>;
    fn clone_dyn(&self) -> Box<dyn TransactionData>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn TransactionData> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: PublicKey,
    pub data: Box<dyn TransactionData>,
    pub signature: Signature,
    pub nonce: u32,
    pub timestamp: i64,
}

impl Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("from", &hex::encode(&self.from))
            .field("data", &self.data)
            .field("signature", &self.signature)
            .field("nonce", &self.nonce)
            .field("timestamp", &self.timestamp)
            .finish()
    }
}

impl Transaction {
    pub fn new<TData: TransactionData>(pair: SecretPair, data: TData) -> Option<Transaction> {
        let hash_func = DoubleHasher::default();

        let from = pair.public_key;
        let timestamp = Utc::now().timestamp();
        let nonce: u32 = rng().next_u32();

        let Some(data_finger_print) = data.get_hash() else {
            return None;
        };

        let input = format!("{}{}{}", hex::encode(data_finger_print), timestamp, nonce);
        let finger_print = hash_func.hash(input);

        let signature = Signature::sign(pair, finger_print);

        Some(Transaction {
            from,
            data: Box::new(data),
            signature,
            timestamp,
            nonce,
        })
    }

    pub fn get_data<TData: 'static>(&self) -> Option<&TData> {
        self.data.as_any().downcast_ref::<TData>()
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize to JSON")
    }
}
