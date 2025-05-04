use std::time::{SystemTime, UNIX_EPOCH};

use super::{hash_func::HashFunc, Block, Transaction};
use crate::{kademlia::secret_key::SecretPair, merkle::MerkleTree};

#[derive(Debug)]
pub(crate) struct BlockBuilder {
    index: u64,
    difficulty: u32,
    prev_hash: [u8; 32],
    transactions: Vec<Transaction>,
    pair: Option<SecretPair>,
}

impl BlockBuilder {
    pub fn new(index: u64, dificult: u32, prev_hash: [u8; 32]) -> BlockBuilder {
        BlockBuilder {
            index,
            difficulty: dificult,
            prev_hash,
            transactions: vec![],
            pair: None,
        }
    }

    pub fn add_transactions<Iterator>(&mut self, transactions: Iterator) -> &mut Self
    where
        Iterator: IntoIterator<Item = Transaction>,
    {
        self.transactions.extend(transactions);
        self
    }

    pub fn sign_with(&mut self, pair: SecretPair) -> &mut Self {
        self.pair = Some(pair);
        self
    }

    pub fn mine<THasher: HashFunc>(&self, hasher: THasher) -> Block {
        let mut hash: [u8; 32];
        let mut nonce = 0;

        // compute the merkle tree
        let merkle_tree = MerkleTree::from_transactions(self.transactions.clone());
        let merkle_root = merkle_tree.root;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to calculate the timestamp")
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
                let mut block = Block::new(
                    self.index,
                    self.difficulty,
                    merkle_root,
                    self.prev_hash,
                    hash,
                    timestamp,
                    nonce,
                    self.transactions.clone(),
                );

                let Some(pair) = self.pair.clone() else {
                    continue;
                };

                block.header.sign(pair);
                return block;
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
