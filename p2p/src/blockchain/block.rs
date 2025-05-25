use serde::{Deserialize, Serialize};

use crate::merkle::MerkleTree;

use super::{BlockHeader, DoubleHasher, HashFunc, Transaction};

type MerkleRoot = [u8; 32];
type Hash = [u8; 32];

pub const MAX_TRANSACTION: usize = 200;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub(crate) fn new(
        index: u64,
        difficulty: u32,
        merkle_root: MerkleRoot,
        prev_hash: Hash,
        hash: Hash,
        timestamp: u128,
        nonce: u32,
        transactions: Vec<Transaction>,
    ) -> Block {
        Block {
            header: BlockHeader {
                index,
                difficulty,
                merkle_root,
                prev_hash,
                hash,
                timestamp,
                nonce,
                signature: None,
            },
            transactions,
        }
    }

    pub(crate) fn new_genesis() -> Block {
        let mut block = Block {
            header: BlockHeader {
                index: 0,
                difficulty: 0,
                timestamp: 0,
                merkle_root: [0u8; 32],
                nonce: 0,
                prev_hash: [0u8; 32],
                hash: [0u8; 32],
                signature: None,
            },
            transactions: vec![],
        };

        block.header.hash = block.compute_hash(DoubleHasher {});
        block
    }

    fn compute_hash<THasher: HashFunc>(&self, hasher: THasher) -> [u8; 32] {
        hasher.hash(format!(
            "{}{}{}{}",
            hex::encode(self.header.prev_hash),
            hex::encode(self.header.merkle_root),
            self.header.timestamp,
            self.header.nonce
        ))
    }

    pub fn validate<THasher>(&self, hasher: THasher, merkle_root: MerkleRoot) -> bool
    where
        THasher: HashFunc,
    {
        let merkle_tree = MerkleTree::from_transactions(self.transactions.clone());

        if merkle_tree.root != merkle_root {
            return false;
        }

        let compute_hash = self.compute_hash(hasher);
        compute_hash == self.header.hash
    }

    pub fn get_transaction<TData: 'static>(&self) -> impl Iterator<Item = (&Transaction, &TData)> {
        self.transactions
            .iter()
            .filter_map(move |tx| tx.get_data::<TData>().map(|data| (tx, data)))
    }
}
