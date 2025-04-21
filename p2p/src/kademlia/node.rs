use std::net::SocketAddr;

use crate::network::grpc::proto::NodeInfo;

use super::{secret_key::SecretPair, NodeId, NODE_ID_LENGTH};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub keys: SecretPair,

    address: String,
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
