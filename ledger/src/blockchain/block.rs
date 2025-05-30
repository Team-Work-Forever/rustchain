use std::fmt;

use bincode::{Decode, Encode};
use serde::Serialize;

use crate::merkle::MerkleTree;

use super::{HashFunc, Transaction, TransactionData};

type MerkleRoot = [u8; 32];
type Hash = [u8; 32];

pub const MAX_TRANSACTION: usize = 200;

#[derive(Clone, Serialize, Encode, Decode)]
pub struct Block<TData>
where
    TData: TransactionData,
{
    pub index: u64,
    pub timestamp: u128,
    pub merkle_root: MerkleRoot,
    pub nonce: u32,
    pub prev_hash: Hash,
    pub hash: Hash,
    transactions: Vec<Transaction<TData>>,
}

impl<TData> Block<TData>
where
    TData: TransactionData,
{
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

    pub fn validate<THasher>(&self, hasher: THasher, merkle_root: MerkleRoot) -> bool
    where
        THasher: HashFunc,
    {
        let merkle_tree = MerkleTree::from_transactions(self.transactions.clone());

        if merkle_tree.root != merkle_root {
            return false;
        }

        let compute_hash = hasher.hash(format!(
            "{}{}{}{}",
            hex::encode(self.prev_hash),
            hex::encode(self.merkle_root),
            self.timestamp,
            self.nonce
        ));

        compute_hash == self.hash
    }
}

impl<TData> fmt::Debug for Block<TData>
where
    TData: TransactionData + fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("index", &self.index)
            .field("timestamp", &self.timestamp)
            .field("merkle_root", &hex::encode(self.merkle_root))
            .field("nonce", &self.nonce)
            .field("prev_hash", &hex::encode(&self.prev_hash))
            .field("hash", &hex::encode(&self.hash))
            .field("transactions", &self.transactions)
            .finish()
    }
}
