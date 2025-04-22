use std::{collections::HashMap, sync::Arc};

use tokio::sync::{Mutex, MutexGuard};
use tonic::{Request, Response, Status};

use crate::{
    kademlia::NodeId,
    network::grpc::proto::{
        find_value_response::Resp, kademlia_service_server::KademliaService, FindNodeRequest,
        FindNodeResponse, FindValueRequest, FindValueResponse, NodeInfo, PingRequest, PongResponse,
        RepetedNode, StoreRequest, StoreResponse,
    },
};

use super::{dht::KademliaData, Node, RoutingTable, KBUCKET_MAX};

#[derive(Debug)]
pub struct GrpcNetwork<TData: KademliaData> {
    pub(crate) node: Node,
    pub(crate) routing_table: Arc<Mutex<RoutingTable<TData>>>,
    pub(crate) distributed_hashing_table: Arc<Mutex<HashMap<NodeId, TData>>>,
}

impl<TData: KademliaData> GrpcNetwork<TData> {
    pub fn new(
        node: Node,
        routing_table: Arc<Mutex<RoutingTable<TData>>>,
        distributed_hashing_table: Arc<Mutex<HashMap<NodeId, TData>>>,
    ) -> Self {
        Self {
            node,
            routing_table,
            distributed_hashing_table,
        }
    }

    async fn get_routing_table(&self) -> MutexGuard<RoutingTable<TData>> {
        self.routing_table.lock().await
    }

    async fn get_distributed_table(&self) -> MutexGuard<HashMap<NodeId, TData>> {
        self.distributed_hashing_table.lock().await
    }

    async fn persist_incoming_node<TRequest>(
        &self,
        request: &Request<TRequest>,
    ) -> Result<MutexGuard<RoutingTable<TData>>, Status> {
        let mut routing_table = self.get_routing_table().await;
        let incoming_node = self.get_peer(&request)?;

        routing_table.insert_node(&incoming_node).await;
        Ok(routing_table)
    }
}

#[tonic::async_trait]
impl<TData: KademliaData> KademliaService for GrpcNetwork<TData> {
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
        let _ = self.persist_incoming_node(&request).await?;
        let request = request.into_inner();

        let key = NodeId::try_from(request.key.clone())
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let config = bincode::config::standard();
        let Ok((decoded_value, _)) = bincode::decode_from_slice(&request.value, config) else {
            return Err(tonic::Status::aborted("Failed to decode value"));
        };

        let mut dht = self.get_distributed_table().await;
        dht.insert(key.clone(), decoded_value);

        Ok(Response::new(StoreResponse { key: key.into() }))
    }

    async fn find_node(
        &self,
        request: tonic::Request<FindNodeRequest>,
    ) -> Result<tonic::Response<FindNodeResponse>, tonic::Status> {
        let routing_table = self.persist_incoming_node(&request).await?;
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

    async fn find_value(
        &self,
        request: tonic::Request<FindValueRequest>,
    ) -> Result<tonic::Response<FindValueResponse>, tonic::Status> {
        let routing_table = self.persist_incoming_node(&request).await?;
        let request = request.into_inner();

        let key = NodeId::try_from(request.key)
            .map_err(|e| Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let dht = self.get_distributed_table().await;

        if let Some(value) = dht.get(&key) {
            let config = bincode::config::standard();
            let Ok(encoded_data) = bincode::encode_to_vec(&value, config) else {
                return Err(Status::aborted("Failed to return value"));
            };

            return Ok(Response::new(FindValueResponse {
                resp: Some(Resp::Value(encoded_data)),
            }));
        };

        let closest_nodes = routing_table.get_closest_nodes(&key, KBUCKET_MAX);

        let response = closest_nodes
            .iter()
            .map(|distance| distance.clone().1.into())
            .collect::<Vec<NodeInfo>>();

        Ok(Response::new(FindValueResponse {
            resp: Some(Resp::Nodes(RepetedNode { nodes: response })),
        }))
    }
}
