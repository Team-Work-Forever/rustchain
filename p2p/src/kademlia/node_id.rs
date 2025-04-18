use sha2::{Digest, Sha256};

use super::NODE_ID_LENGTH;

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct NodeId(pub [u8; NODE_ID_LENGTH]);

impl NodeId {
    pub fn new(pub_key: &[u8]) -> Self {
        let hasher = Sha256::digest(pub_key);
        let mut id = [0u8; NODE_ID_LENGTH];
        id.copy_from_slice(&hasher[..NODE_ID_LENGTH]);

        NodeId(id)
    }

    pub fn distance(&self, node: &NodeId) -> NodeId {
        let mut distance = [0; NODE_ID_LENGTH];
        for i in 0..NODE_ID_LENGTH {
            distance[i] = self.0[i] ^ node.0[i];
        }

        NodeId(distance)
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
