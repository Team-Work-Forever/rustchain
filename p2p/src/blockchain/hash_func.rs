use sha2::{Digest, Sha256};

pub trait HashFunc: Clone {
    fn hash(&self, value: String) -> [u8; 32];
}

#[derive(Clone)]
pub struct DoubleHasher;

#[derive(Clone)]
pub struct DefaultHasher;

impl HashFunc for DefaultHasher {
    fn hash(&self, value: String) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(value);

        return hasher
            .finalize()
            .try_into()
            .expect("Try to convert to slice of 32 bytes");
    }
}

impl HashFunc for DoubleHasher {
    fn hash(&self, value: String) -> [u8; 32] {
        let first = Sha256::digest(value);
        Sha256::digest(first).try_into().expect("Cannot Hash value")
    }
}
