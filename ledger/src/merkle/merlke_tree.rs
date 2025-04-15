use hex;
use log::info;
use sha2::{Digest, Sha256};

use crate::blockchain::{Transaction, TransactionData};

pub struct MerkleTree {
    pub root: [u8; 32],
    pub levels: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn from_transactions<TData: TransactionData>(txs: Vec<Transaction<TData>>) -> Self {
        let leaves = txs
            .iter()
            .map(|transaction| Self::hash(transaction.to_json_string()))
            .collect::<Vec<_>>();

        Self::build_tree(leaves)
    }

    fn build_tree(mut current_level: Vec<[u8; 32]>) -> Self {
        let mut levels = vec![current_level.clone()];

        while current_level.len() > 1 {
            current_level = Self::next_level(&current_level);
            levels.push(current_level.clone());
        }

        MerkleTree {
            root: current_level[0].clone(),
            levels,
        }
    }

    fn next_level(prev_level: &[[u8; 32]]) -> Vec<[u8; 32]> {
        prev_level
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    Self::hash(format!(
                        "{}{}",
                        hex::encode(chunk[0]),
                        hex::encode(chunk[1])
                    ))
                } else {
                    Self::hash(format!(
                        "{}{}",
                        hex::encode(chunk[0]),
                        hex::encode(chunk[0])
                    )) // duplicate last
                }
            })
            .collect()
    }

    fn hash(data: String) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().try_into().expect("msg")
    }

    pub fn print_tree(&self) {
        for (i, level) in self.levels.iter().enumerate().rev() {
            info!("Level {}: {:?}", i, level);
        }
    }
}
