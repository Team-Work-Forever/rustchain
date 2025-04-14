use std::time::{SystemTime, UNIX_EPOCH};

use super::{hash_func::HashFunc, Block, Transaction};
use crate::merkle::MerkleTree;
use serde::Serialize;

#[derive(Debug)]
pub struct BlockBuilder<TData: Serialize> {
    index: u64,
    difficulty: u32,
    prev_hash: [u8; 32],
    transactions: Vec<Transaction<TData>>,
}

impl<TData: Clone + Serialize> BlockBuilder<TData> {
    pub fn new(index: u64, dificult: u32, prev_hash: [u8; 32]) -> BlockBuilder<TData> {
        BlockBuilder {
            index,
            difficulty: dificult,
            prev_hash,
            transactions: vec![],
        }
    }

    pub fn add_transactions<Iterator>(&mut self, transactions: Iterator) -> &mut Self
    where
        Iterator: IntoIterator<Item = Transaction<TData>>,
    {
        self.transactions.extend(transactions);
        self
    }

    pub fn mine<THasher: HashFunc>(&self, hasher: THasher) -> Block<TData> {
        let mut hash: [u8; 32];
        let mut nonce = 0;

        // compute the merkle tree
        let merkle_tree = MerkleTree::from_transactions(self.transactions.clone());
        let merkle_root = merkle_tree.root;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos();

        loop {
            let input = format!(
                "{}{}{}{}",
                hex::encode(self.prev_hash),
                hex::encode(merkle_root),
                timestamp,
                nonce
            );

            hash = hasher.hash(input);

            if self.validate_hash(&hash, self.difficulty) {
                return Block::new(
                    self.index,
                    merkle_root,
                    self.prev_hash,
                    hash,
                    timestamp,
                    nonce,
                    self.transactions.clone(),
                );
            }

            nonce = nonce.wrapping_add(1);
        }
    }

    fn validate_hash(&self, hash: &[u8; 32], difficulty: u32) -> bool {
        let nibbles = difficulty as usize;
        let full_bytes = nibbles / 2;
        let has_half_nibble = nibbles % 2 == 1;

        for i in 0..full_bytes {
            if hash[i] != 0 {
                return false;
            }
        }

        if has_half_nibble {
            if (hash[full_bytes] >> 4) != 0 {
                return false;
            }
        }

        true
    }
}
