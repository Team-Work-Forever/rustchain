use std::fmt::{self};

use serde::Serialize;

use crate::merkle::MerkleTree;

use super::Transaction;

type MerkleRoot = [u8; 32];
type Hash = [u8; 32];

#[derive(Clone, Serialize)]
pub struct Block<TData: Clone + Serialize> {
    pub index: u64,
    pub timestamp: u128,
    pub merkle_root: MerkleRoot,
    pub nonce: u32,
    pub prev_hash: Hash,
    pub hash: Hash,
    transactions: Vec<Transaction<TData>>,
}

impl<TData: Clone + Serialize> Block<TData> {
    pub(crate) fn new(
        index: u64,
        merkle_root: MerkleRoot,
        prev_hash: Hash,
        hash: Hash,
        timestamp: u128,
        nonce: u32,
        transactions: Vec<Transaction<TData>>,
    ) -> Block<TData> {
        Block {
            index,
            merkle_root,
            prev_hash,
            hash,
            timestamp,
            nonce,
            transactions,
        }
    }

    pub(crate) fn new_genesis() -> Block<TData> {
        Block {
            index: 0,
            timestamp: 0,
            merkle_root: [0u8; 32],
            nonce: 0,
            prev_hash: [0u8; 32],
            hash: [0u8; 32],
            transactions: vec![],
        }
    }

    pub fn validate(&self, merkle_root: MerkleRoot) -> bool {
        MerkleTree::from_transactions(self.transactions.clone()).root == merkle_root
    }
}

impl<TData: Clone + fmt::Debug + Serialize> fmt::Debug for Block<TData> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Block")
            .field("index", &self.index)
            .field("merkle_root", &hex::encode(&self.merkle_root)) // You can .to_string() here if needed
            .field("hash", &hex::encode(&self.hash))
            .field("prev_hash", &hex::encode(&self.prev_hash))
            .field("nonce", &self.nonce)
            .field("timestamp", &self.timestamp)
            .field("transactions", &self.transactions)
            .finish()
    }
}
