use sha2::{Digest, Sha256};

use super::{NodeId, NODE_ID_LENGTH};

#[derive(Clone, Debug)]
pub struct Node {
    pub id: NodeId,

    pub address: String,
    pub port: u16,
}

impl Node {
    pub fn new(address: String, port: u16) -> Self {
        Self {
            id: NodeId::new(&[0u8; NODE_ID_LENGTH]),
            address,
            port,
        }
    }
    pub fn yet_another_new(address: String, port: u16) -> Self {
        Self {
            id: NodeId::new(&[1u8; NODE_ID_LENGTH]),
            address,
            port,
        }
    }
    pub fn test(address: String, port: u16) -> Self {
        let first = Sha256::digest("another way, that does not matter right now");
        let oi: [u8; 32] = Sha256::digest(first).try_into().expect("Cannot Hash value");

        Self {
            id: NodeId::new(&oi),
            address,
            port,
        }
    }
    pub fn ed(address: String, port: u16) -> Self {
        let first = Sha256::digest("perfect");
        let oi: [u8; 32] = Sha256::digest(first).try_into().expect("Cannot Hash value");

        Self {
            id: NodeId::new(&oi),
            address,
            port,
        }
    }
}
