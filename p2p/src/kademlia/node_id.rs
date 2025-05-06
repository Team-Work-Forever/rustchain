use rand::{rngs::OsRng, TryRngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::blockchain::{DoubleHasher, HashFunc};

use super::{distance::Distance, NODE_ID_LENGTH};

#[derive(Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct NodeId(pub [u8; NODE_ID_LENGTH]);

impl NodeId {
    pub fn new(pub_key: &[u8]) -> Self {
        let hasher = Sha256::digest(pub_key);
        let mut id = [0u8; NODE_ID_LENGTH];
        id.copy_from_slice(&hasher[..NODE_ID_LENGTH]);

        NodeId(id)
    }

    pub fn get_namespace_key(namespace: &str, node_id: NodeId) -> NodeId {
        let nx = format!("{}:{}", namespace, hex::encode(node_id.0));
        let hasher = DoubleHasher::default();
        let key = hasher.hash(nx);

        NodeId::new(&key)
    }

    pub fn create_chain_head(node_id: NodeId) -> NodeId {
        Self::get_namespace_key("chain_head", node_id)
    }

    pub fn create_ticket(node_id: NodeId) -> NodeId {
        Self::get_namespace_key("ticket", node_id)
    }

    pub fn random() -> Option<Self> {
        let mut secret_bytes = [0u8; 32];
        if let Err(_) = OsRng.try_fill_bytes(&mut secret_bytes) {
            return None;
        }

        Some(NodeId(secret_bytes))
    }

    pub(crate) fn distance(&self, node: &NodeId) -> Distance {
        let mut distance = [0; NODE_ID_LENGTH];
        for i in 0..NODE_ID_LENGTH {
            distance[i] = self.0[i] ^ node.0[i];
        }

        Distance(distance)
    }
}

impl PartialOrd for NodeId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for NodeId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NodeId")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl PartialEq<Vec<u8>> for NodeId {
    fn eq(&self, other: &Vec<u8>) -> bool {
        &self.0[..] == &other[..]
    }
}

impl TryFrom<Vec<u8>> for NodeId {
    type Error = &'static str;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != NODE_ID_LENGTH {
            return Err("Invalid length for NodeId");
        }

        let mut id = [0u8; NODE_ID_LENGTH];
        id.copy_from_slice(&value[..]);
        Ok(NodeId(id))
    }
}

impl From<NodeId> for Vec<u8> {
    fn from(value: NodeId) -> Self {
        value.0.to_vec()
    }
}
