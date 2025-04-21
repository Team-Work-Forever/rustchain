use std::sync::Arc;

use tokio::sync::{Mutex, MutexGuard};
use tonic::Response;

use crate::{
    kademlia::NodeId,
    network::grpc::proto::{
        kademlia_service_server::KademliaService, FindNodeRequest, FindNodeResponse, NodeInfo,
        PingRequest, PongResponse, StoreRequest, StoreResponse,
    },
};

use super::{Node, RoutingTable, KBUCKET_MAX};

#[derive(Debug)]
pub struct GrpcNetwork {
    pub(crate) node: Node,
    pub(crate) routing_table: Arc<Mutex<RoutingTable>>,
}

impl GrpcNetwork {
    pub fn new(node: Node, routing_table: Arc<Mutex<RoutingTable>>) -> Self {
        Self {
            node,
            routing_table,
        }
    }

    async fn get_routing_table(&self) -> MutexGuard<RoutingTable> {
        self.routing_table.lock().await
    }
}

#[tonic::async_trait]
impl KademliaService for GrpcNetwork {
    async fn ping(
        &self,
        request: tonic::Request<PingRequest>,
    ) -> Result<tonic::Response<PongResponse>, tonic::Status> {
        let host_node = self.node.clone();
        let _peer = self.get_peer(&request)?;

        Ok(Response::new(PongResponse {
            node_id: host_node.id.into(),
        }))
    }

    async fn store(
        &self,
        request: tonic::Request<StoreRequest>,
    ) -> Result<tonic::Response<StoreResponse>, tonic::Status> {
        let request = request.into_inner();

        Ok(Response::new(StoreResponse {
            key: request.key.into(),
        }))
    }

    async fn find_node(
        &self,
        request: tonic::Request<FindNodeRequest>,
    ) -> Result<tonic::Response<FindNodeResponse>, tonic::Status> {
        // add the incoming node to routing table
        let mut routing_table = self.get_routing_table().await;
        let incoming_node = self.get_peer(&request)?;

        routing_table.insert_node(&incoming_node).await;

        let request = request.into_inner();

        let count = {
            let bytes: &[u8] = &request.count;
            let number = u64::from_le_bytes(bytes.try_into().unwrap());

            number as usize
        }
        .min(KBUCKET_MAX);

        let lookup_id = NodeId::try_from(request.key)
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let closest_nodes = routing_table.get_closest_nodes(&lookup_id, count);

        let response = closest_nodes
            .iter()
            .map(|distance| distance.clone().1.into())
            .collect::<Vec<NodeInfo>>();

        Ok(Response::new(FindNodeResponse { nodes: response }))
    }
}
