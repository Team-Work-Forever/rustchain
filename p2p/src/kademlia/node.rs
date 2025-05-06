use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::network::grpc::proto::NodeInfo;

use super::{secret_key::SecretPair, ticket::NodeTicket, NodeId, NODE_ID_LENGTH};

#[derive(Clone)]
pub struct Contract {
    pub host: String,
    pub port: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub keys: SecretPair,
    pub ticket: Option<NodeTicket>,

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
            ticket: None,
        })
    }

    pub fn from_pub_key(pub_key: &[u8; NODE_ID_LENGTH], address: String, port: usize) -> Self {
        Self {
            id: NodeId::new(pub_key),
            keys: SecretPair::default(*pub_key),
            address,
            port,
            ticket: None,
        }
    }

    pub fn from_contract(contract: &Contract) -> Self {
        Self {
            id: NodeId::new(&[0u8; 32]),
            keys: SecretPair::default([0u8; 32]),
            address: contract.host.clone(),
            port: contract.port,
            ticket: None,
        }
    }

    pub fn from_node(address: String, port: usize, node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            keys: node.keys.clone(),
            address,
            port,
            ticket: None,
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
            ticket: None,
        })
    }

    pub fn set_ticket(&mut self, ticket: &NodeTicket) {
        self.ticket = Some(ticket.clone());
    }

    pub fn get_addr(&self) -> Result<SocketAddr, Box<dyn std::error::Error>> {
        let addr: SocketAddr = format!("{}:{}", self.address, self.port).parse()?;
        Ok(addr)
    }
}
