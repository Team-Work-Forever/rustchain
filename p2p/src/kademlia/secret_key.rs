use ed25519_dalek::{
    ed25519::signature::SignerMut, Signature, SigningKey, VerifyingKey, SIGNATURE_LENGTH,
};
use rand::{rngs::OsRng, TryRngCore};
use serde::{Deserialize, Serialize};

use super::NODE_ID_LENGTH;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretPair {
    pub public_key: [u8; NODE_ID_LENGTH],  // Verifing key
    pub private_key: [u8; NODE_ID_LENGTH], // Signing key
}

impl SecretPair {
    pub fn default(public_key: [u8; NODE_ID_LENGTH]) -> Self {
        Self {
            public_key,
            private_key: [0u8; NODE_ID_LENGTH],
        }
    }

    pub fn sign(&self, value: [u8; NODE_ID_LENGTH]) -> [u8; SIGNATURE_LENGTH] {
        let mut private_key = SigningKey::from_bytes(&self.private_key);
        let signature = private_key.sign(&value);

        signature.to_bytes()
    }

    pub fn verify(&self, value: [u8; NODE_ID_LENGTH], signature: [u8; SIGNATURE_LENGTH]) -> bool {
        let public_key = match VerifyingKey::from_bytes(&self.public_key) {
            Ok(pub_key) => pub_key,
            _ => return false,
        };

        match public_key.verify_strict(&value, &Signature::from_bytes(&signature)) {
            Ok(_) => true,
            _ => false,
        }
    }

    pub fn generate_keys() -> Result<SecretPair, ()> {
        let mut secret_bytes = [0u8; 32];
        if let Err(_) = OsRng.try_fill_bytes(&mut secret_bytes) {
            return Err(());
        }

        let private_key = SigningKey::from_bytes(&secret_bytes);
        let public_key = VerifyingKey::from(&private_key);

        Ok(SecretPair {
            private_key: private_key.to_bytes(),
            public_key: *public_key.as_bytes(),
        })
    }
}

impl std::fmt::Debug for SecretPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SecretPair")
            .field(&hex::encode(&self.public_key))
            .field(&hex::encode(&self.private_key))
            .finish()
    }
}
