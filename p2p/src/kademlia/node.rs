use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::network::grpc::proto::NodeInfo;

use super::{secret_key::SecretPair, NodeId, NODE_ID_LENGTH};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub keys: SecretPair,

    #[serde(skip)]
    address: String,

    #[serde(skip)]
    port: usize,
}

impl Into<NodeInfo> for Node {
    fn into(self) -> NodeInfo {
        NodeInfo {
            id: self.id.into(),
            addr: self.address,
            port: self.port as u64,
            pub_key: self.keys.public_key.into(),
        }
    }
}

impl Node {
    pub fn from(node: NodeInfo) -> Option<Self> {
        let public_key = node
            .pub_key
            .try_into()
            .expect("Failed to convert to slice bytes");

        Some(Self {
            id: node.id.try_into().unwrap(),
            keys: SecretPair::default(public_key),
            address: node.addr,
            port: node.port as usize,
        })
    }

    pub fn from_pub_key(pub_key: &[u8; NODE_ID_LENGTH], address: String, port: usize) -> Self {
        Self {
            id: NodeId::new(pub_key),
            keys: SecretPair::default(*pub_key),
            address,
            port,
        }
    }

    pub fn from_node(address: String, port: usize, node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            keys: node.keys.clone(),
            address,
            port,
        }
    }

    pub fn new(address: String, port: usize) -> Option<Self> {
        let Ok(keys) = SecretPair::generate_keys() else {
            return None;
        };

        Some(Self {
            id: NodeId::new(&keys.public_key[..NODE_ID_LENGTH]),
            keys,
            address,
            port,
        })
    }

    pub fn get_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        let addr: SocketAddr = format!("{}:{}", self.address, self.port).parse()?;
        Ok(addr)
    }
}
