use serde::{Deserialize, Serialize};

use crate::kademlia::NODE_ID_LENGTH;

use super::secret_key::SecretPair;

pub trait HandleSignature {
    fn sign(pair: SecretPair, value: [u8; NODE_ID_LENGTH]) -> Signature;
    fn validate_signature(
        &self,
        pub_key: [u8; NODE_ID_LENGTH],
        value: [u8; NODE_ID_LENGTH],
    ) -> bool;
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub signature: Vec<u8>,
    pub pub_key: [u8; NODE_ID_LENGTH],
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature")
            .field("signature", &hex::encode(&self.signature))
            .field("pub_key", &hex::encode(&self.pub_key))
            .finish()
    }
}

impl HandleSignature for Signature {
    fn sign(pair: SecretPair, value: [u8; NODE_ID_LENGTH]) -> Signature {
        let signature = pair.sign(value);
        Signature::from(pair.public_key, signature)
    }

    fn validate_signature(
        &self,
        pub_key: [u8; NODE_ID_LENGTH],
        value: [u8; NODE_ID_LENGTH],
    ) -> bool {
        if pub_key != self.pub_key {
            return false;
        }

        let pair = SecretPair::default(pub_key);
        pair.verify(value, self.get_signature())
    }
}

impl Signature {
    pub fn from(pub_key: [u8; NODE_ID_LENGTH], signature: [u8; 64]) -> Self {
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
