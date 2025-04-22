use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use bincode::{Decode, Encode};
use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

use crate::network::grpc::proto::{
    find_value_response::Resp, FindNodeRequest, FindValueRequest, PingRequest, StoreRequest,
};

use super::{
    distance::NodeDistance, network::GrpcNetwork, Node, NodeId, RoutingTable, KBUCKET_MAX,
};

pub trait KademliaData: Encode + Send + Decode<()> + 'static {}

#[derive(Debug, Error)]
pub enum KademliaError {
    #[error("Error while accessing private resources")]
    FailedAccessError,

    #[error("Failed to ping node")]
    PingFailedError,

    #[error("Failed to store command")]
    StoreFailedError,

    #[error("Failed to find node command")]
    FindNodeFailedError,

    #[error("Failed to find value command")]
    FindValueFailedError,
}

#[derive(Debug, Clone)]
pub struct DHTNode<TData>
where
    TData: KademliaData,
{
    pub core: Node,

    pub routing_table: Arc<Mutex<RoutingTable<TData>>>,
    pub distributed_hash_tb: Arc<Mutex<HashMap<NodeId, TData>>>,
}

impl<TData: KademliaData> DHTNode<TData> {
    pub async fn bootstrap(address: String, port: usize) -> Option<Self> {
        let Some(node) = Node::new(address, port) else {
            return None;
        };

        let dth = Self {
            core: node.clone(),
            routing_table: Arc::new(Mutex::new(RoutingTable::new(node).await)),
            distributed_hash_tb: Arc::new(Mutex::new(HashMap::new())),
        };

        dth.init();

        Some(dth)
    }

    pub async fn new(bootstrap: Self, address: String, port: usize) -> Option<Self> {
        let boot_node = Self::bootstrap(address, port).await?;

        {
            let mut routing_table = boot_node.get_routing_table().await;
            routing_table.insert_node(&bootstrap.core).await;
        }

        let Ok(update_nodes) = boot_node.node_lookup(&boot_node.core.id).await else {
            return None;
        };

        {
            let mut routing_table = boot_node.get_routing_table().await;
            for node in update_nodes {
                routing_table.insert_node(&node).await;
            }
        }

        Some(boot_node)
    }

    async fn get_routing_table(&self) -> MutexGuard<RoutingTable<TData>> {
        self.routing_table.lock().await
    }

    async fn get_dth_table(&self) -> MutexGuard<HashMap<NodeId, TData>> {
        self.distributed_hash_tb.lock().await
    }

    fn init(&self) {
        let core = self.core.clone();
        let routing_table = self.routing_table.clone();
        let distributed_hash_tb = self.distributed_hash_tb.clone();

        // start the GRPC
        tokio::spawn(async move {
            if let Err(e) =
                GrpcNetwork::start_network(core, routing_table, distributed_hash_tb).await
            {
                eprintln!("Network error: {}", e);
            }
        });

        // start the block chain miner
    }

    pub async fn ping(host: &Node, target: &Node) -> Result<(), KademliaError> {
        let mut client = GrpcNetwork::<TData>::connect_over(host.clone(), target.clone())
            .await
            .map_err(|e| {
                println!("{}", e);
                return KademliaError::PingFailedError;
            })?;

        let response = client
            .ping(PingRequest {
                node_id: host.clone().id.into(),
            })
            .await
            .map_err(|_| KademliaError::PingFailedError)?
            .into_inner();

        let target_id =
            NodeId::try_from(response.node_id).map_err(|_| KademliaError::PingFailedError)?;

        if target.id != target_id {
            return Err(KademliaError::PingFailedError);
        }

        Ok(())
    }

    pub async fn store(&self, key: &NodeId, value: TData) -> Result<(), KademliaError> {
        let mut has_stored = false;

        let closest_nodes = self.node_lookup(key).await?;

        let config = bincode::config::standard();
        let Ok(encoded_data) = bincode::encode_to_vec(&value, config) else {
            return Err(KademliaError::StoreFailedError);
        };

        for node in closest_nodes.clone() {
            let Ok(mut client) =
                GrpcNetwork::<TData>::connect_over(self.core.clone(), node.clone()).await
            else {
                continue;
            };

            let Ok(response) = client
                .store(StoreRequest {
                    key: key.clone().into(),
                    value: encoded_data.clone().into(),
                })
                .await
            else {
                continue;
            };

            let response = response.into_inner();
            let Ok(lookup_id) = NodeId::try_from(response.key) else {
                continue;
            };

            if *key == lookup_id {
                has_stored = true;
            }
        }

        if closest_nodes.iter().any(|n| n.id == self.core.id) {
            self.get_dth_table().await.insert(key.clone(), value);
            has_stored = true;
        }

        return if has_stored {
            Ok(())
        } else {
            Err(KademliaError::StoreFailedError)
        };
    }

    pub async fn node_lookup(&self, target_id: &NodeId) -> Result<Vec<Node>, KademliaError> {
        let routing_table = self.get_routing_table().await;

        let mut visited_nodes = HashSet::<NodeId>::new();
        let mut closest_nodes = Vec::new();

        let mut check_nodes =
            VecDeque::from(routing_table.get_closest_nodes(&target_id, KBUCKET_MAX));

        while let Some(node_distance) = check_nodes.pop_front() {
            let current_node = node_distance.1;

            if !visited_nodes.insert(current_node.id.clone()) {
                continue;
            }

            let Ok(target_closest_nodes) =
                DHTNode::<TData>::find_node(&self.core, &current_node, target_id).await
            else {
                continue;
            };

            closest_nodes.push(current_node);

            let mut distances = target_closest_nodes
                .into_iter()
                .filter(|node| node.id != self.core.id && !visited_nodes.contains(&node.id))
                .map(|node| NodeDistance(target_id.distance(&node.id), node))
                .collect::<Vec<_>>();

            distances.sort();
            check_nodes.extend(distances);
        }

        closest_nodes.sort_by_key(|node| target_id.distance(&node.id));
        closest_nodes.dedup_by(|a, b| a.id == b.id);

        Ok(closest_nodes)
    }

    pub async fn find_node(
        host: &Node,
        target: &Node,
        lookup_id: &NodeId,
    ) -> Result<Vec<Node>, Box<KademliaError>> {
        let mut client = GrpcNetwork::<TData>::connect_over(host.clone(), target.clone())
            .await
            .map_err(|_| KademliaError::FindNodeFailedError)?;

        let response = client
            .find_node(FindNodeRequest {
                key: lookup_id.clone().into(),
                count: KBUCKET_MAX.to_le_bytes().into(),
            })
            .await
            .map_err(|_| KademliaError::FindNodeFailedError)?;

        let response = response.into_inner();
        let nodes = response
            .nodes
            .iter()
            .flat_map(|node_info| Node::from(node_info.clone()))
            .collect::<Vec<_>>();

        Ok(nodes)
    }

    pub async fn find_value(&self, key: &NodeId) -> Result<Option<TData>, KademliaError> {
        let routing_table = self.get_routing_table().await;

        // self, search for the value first!

        let mut visited_nodes = HashSet::<NodeId>::new();
        let mut check_nodes = VecDeque::from(routing_table.get_closest_nodes(&key, KBUCKET_MAX));

        while let Some(NodeDistance(_, current_node)) = check_nodes.pop_front() {
            if !visited_nodes.insert(current_node.id.clone()) {
                continue;
            }

            let Ok(mut client) =
                GrpcNetwork::<TData>::connect_over(self.core.clone(), current_node.clone()).await
            else {
                continue;
            };

            let Ok(response) = client
                .find_value(FindValueRequest {
                    key: key.clone().into(),
                })
                .await
            else {
                continue;
            };

            let response = response.into_inner();

            match response.resp {
                Some(Resp::Nodes(target_closest_nodes)) => {
                    let mut distances = target_closest_nodes
                        .nodes
                        .into_iter()
                        .filter_map(|node_info| Node::from(node_info))
                        .filter(|node| node.id != self.core.id && !visited_nodes.contains(&node.id))
                        .map(|node| NodeDistance(key.distance(&node.id), node))
                        .collect::<Vec<_>>();

                    distances.sort();
                    check_nodes.extend(distances);
                }

                Some(Resp::Value(value)) => {
                    let config = bincode::config::standard();

                    let Ok((decoded_value, _)) = bincode::decode_from_slice(&value, config) else {
                        return Err(KademliaError::FindValueFailedError);
                    };

                    return Ok(Some(decoded_value));
                }

                None => continue,
            }
        }

        Ok(None)
    }
}
