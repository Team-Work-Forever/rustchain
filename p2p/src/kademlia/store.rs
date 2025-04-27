use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::DHTNode;

use super::{data::KademliaData, Node, NodeId, RoutingTable};

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistDHTNode {
    pub core: Node,
    pub distributed_hash_tb: HashMap<NodeId, Box<dyn KademliaData>>,
}

impl PersistDHTNode {
    pub fn new() -> Option<Self> {
        let Some(node) = Node::new("".into(), 0) else {
            return None;
        };

        Some(Self {
            core: node,
            distributed_hash_tb: HashMap::new(),
        })
    }
}

impl DHTNode {
    pub async fn from(address: String, port: usize, persist_dht: PersistDHTNode) -> Option<Self> {
        let node = Node::from_node(address, port, &persist_dht.core);

        let dth = Self {
            core: node.clone(),
            routing_table: Arc::new(Mutex::new(RoutingTable::new(node).await)),
            distributed_hash_tb: Arc::new(Mutex::new(persist_dht.distributed_hash_tb)),
        };

        Some(dth)
    }

    pub async fn into_persist(&self) -> PersistDHTNode {
        let dht = self.distributed_hash_tb.lock().await;

        PersistDHTNode {
            core: self.core.clone(),
            distributed_hash_tb: dht.clone(),
        }
    }
}
