use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use thiserror::Error;
use tokio::sync::{Mutex, MutexGuard};

use crate::network::grpc::proto::{FindNodeRequest, PingRequest};

use super::{
    distance::NodeDistance, network::GrpcNetwork, Node, NodeId, RoutingTable, KBUCKET_MAX,
};

pub trait KademliaData {
    fn clone_box(&self) -> Box<dyn KademliaData>;
}
impl<T> KademliaData for T
where
    T: Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn KademliaData> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn KademliaData> {
    fn clone(&self) -> Box<dyn KademliaData> {
        self.clone_box()
    }
}

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
}

#[derive(Debug, Clone)]
pub struct DHTNode<TData: KademliaData> {
    pub core: Node,

    pub routing_table: Arc<Mutex<RoutingTable>>,
    _distributed_hash_tb: HashMap<NodeId, TData>,
}

impl<TData: KademliaData> DHTNode<TData> {
    pub async fn bootstrap(address: String, port: usize) -> Option<Self> {
        let Some(node) = Node::new(address, port) else {
            return None;
        };

        let dth = Self {
            core: node.clone(),
            routing_table: Arc::new(Mutex::new(RoutingTable::new(node).await)),
            _distributed_hash_tb: HashMap::new(),
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

    async fn get_routing_table(&self) -> MutexGuard<RoutingTable> {
        self.routing_table.lock().await
    }

    fn init(&self) {
        let core = self.core.clone();
        let routing_table = self.routing_table.clone();

        // start the GRPC
        tokio::spawn(async move {
            if let Err(e) = GrpcNetwork::start_network(core, routing_table.clone()).await {
                eprintln!("Network error: {}", e);
            }
        });

        // start the block chain miner
    }

    pub async fn ping(host: &Node, target: &Node) -> Result<(), KademliaError> {
        let mut client = GrpcNetwork::connect_over(host.clone(), target.clone())
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

    // pub async fn store(&self, key: NodeId, value: TData) -> Result<(), KademliaError> {
    //     let key_clone = key.clone();
    //     let routing_table = self.get_routing_table()?;

    //     let closest_nodes = routing_table.get_closest_nodes(&key, KBUCKET_MAX);
    //     let mut sucess = false;

    //     for node in closest_nodes.iter() {
    //         let node = node.1.clone();

    //         let Ok(mut client) = GrpcNetwork::connect_over(self.core.clone(), node.clone()).await
    //         else {
    //             continue;
    //         };

    //         let config = config::standard();
    //         let Ok(bin_data) = bincode::encode_to_vec(&value, config) else {
    //             continue;
    //         };

    //         let Ok(response) = client
    //             .store(StoreRequest {
    //                 key: key_clone.clone().into(),
    //                 value: bin_data,
    //             })
    //             .await
    //         else {
    //             continue;
    //         };

    //         let response = response.into_inner();
    //         let target_id =
    //             NodeId::try_from(response.key).map_err(|_| KademliaError::PingFailedError)?;

    //         sucess = target_id == node.id;
    //     }

    //     if !sucess {
    //         return Err(KademliaError::StoreFailedError);
    //     }

    //     Ok(())
    // }

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

        // arrange and sort
        closest_nodes.sort_by_key(|node| target_id.distance(&node.id));
        closest_nodes.dedup_by(|a, b| a.id == b.id);

        Ok(closest_nodes)
    }

    pub async fn find_node(
        host: &Node,
        target: &Node,
        lookup_id: &NodeId,
    ) -> Result<Vec<Node>, Box<KademliaError>> {
        let mut client = GrpcNetwork::connect_over(host.clone(), target.clone())
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

    pub async fn find_value(&self, target: Self) -> Result<(), Box<dyn std::error::Error>> {
        let _host_node = target.core.clone();
        let mut _client = GrpcNetwork::connect_over(self.core.clone(), target.core).await?;

        Ok(())
    }
}
