use serde::{Deserialize, Serialize};

use crate::kademlia::NODE_ID_LENGTH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub signature: Vec<u8>,
    pub pub_key: [u8; NODE_ID_LENGTH],
}

impl Signature {
    pub fn new(pub_key: [u8; NODE_ID_LENGTH], signature: [u8; 64]) -> Self {
        Self {
            pub_key,
            signature: signature.into(),
        }
    }

    pub fn get_signature(&self) -> [u8; 64] {
        if self.signature.len() != 64 {
            return [0u8; 64];
        }

        match self.signature.clone().try_into() {
            Ok(sign) => sign,
            Err(_) => [0u8; 64],
        }
    }
}

impl Default for Signature {
    fn default() -> Self {
        Self {
            pub_key: [0u8; 32],
            signature: vec![],
        }
    }
}
