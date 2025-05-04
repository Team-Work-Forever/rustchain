use std::fmt;

use serde::{Deserialize, Serialize};

use crate::kademlia::{secret_key::SecretPair, signature::Signature};

type MerkleRoot = [u8; 32];
type Hash = [u8; 32];

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub index: u64,
    pub difficulty: u32,
    pub timestamp: u128,
    pub merkle_root: MerkleRoot,
    pub nonce: u32,
    pub prev_hash: Hash,
    pub hash: Hash,
    pub signature: Option<Signature>,
}

impl BlockHeader {
    pub fn sign(&mut self, pair: SecretPair) {
        let signature = pair.sign(self.hash);
        self.signature = Some(Signature::new(pair.public_key, signature));
    }

    pub fn validate_signature(&self, pub_key: [u8; 32]) -> bool {
        let Some(signature) = self.signature.clone() else {
            return false;
        };

        if pub_key != signature.pub_key {
            return false;
        }

        let pair = SecretPair::default(pub_key);
        pair.verify(self.hash, signature.get_signature())
    }
}

impl fmt::Debug for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Header");

        debug
            .field("index", &self.index)
            .field("difficulty", &self.difficulty)
            .field("timestamp", &self.timestamp)
            .field("merkle_root", &hex::encode(&self.merkle_root))
            .field("nonce", &self.nonce)
            .field("prev_hash", &hex::encode(&self.prev_hash))
            .field("hash", &hex::encode(self.hash));

        if let Some(signature) = &self.signature {
            debug.field("signature", &hex::encode(&signature.get_signature()));
        }

        debug.finish()
    }
}
