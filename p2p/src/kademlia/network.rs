use std::{collections::HashMap, sync::Arc};

use log::info;
use rand::Rng;
use tokio::sync::Mutex;
use tonic::{Response, Status};

use crate::{
    blockchain::DoubleHasher,
    kademlia::NodeId,
    network::grpc::proto::{
        find_value_response::Resp, join_service_server::JoinService,
        kademlia_service_server::KademliaService, ChallangeRequest, ChallangeResponse,
        FindNodeRequest, FindNodeResponse, FindValueRequest, FindValueResponse, NodeInfo,
        PingRequest, PongResponse, RepetedNode, StoreRequest, StoreResponse, SubmitRequest,
        SubmitResponse,
    },
};

use super::{
    data::{KademliaData, Ticket},
    event::{DHTEvent, DHTEventHandler},
    routing_table::RoutingTable,
    signature::{HandleSignature, Signature},
    ticket::NodeTicket,
    Node, KBUCKET_MAX,
};

#[derive(Debug, Clone)]
pub(crate) struct GrpcNetwork {
    pub(crate) node: Node,
    pub(crate) routing_table: Arc<Mutex<RoutingTable>>,
    pub(crate) distributed_hashing_table: Arc<Mutex<HashMap<NodeId, Box<dyn KademliaData>>>>,

    pub(crate) event_handler: Arc<dyn DHTEventHandler>,
}

impl GrpcNetwork {
    pub fn new(
        node: Node,
        routing_table: Arc<Mutex<RoutingTable>>,
        distributed_hashing_table: Arc<Mutex<HashMap<NodeId, Box<dyn KademliaData>>>>,
        event_handler: Arc<dyn DHTEventHandler>,
    ) -> Self {
        Self {
            node,
            routing_table,
            distributed_hashing_table,
            event_handler,
        }
    }
}

#[tonic::async_trait]
impl JoinService for GrpcNetwork {
    async fn request_challange(
        &self,
        request: tonic::Request<ChallangeRequest>,
    ) -> Result<tonic::Response<ChallangeResponse>, tonic::Status> {
        let request = request.into_inner();

        let pub_key = NodeId::try_from(request.pub_key)
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let ticket_id = NodeId::create_ticket(pub_key);

        let difficulty: u32 = 5;
        let nonce: u32 = rand::rng().random();

        let dht_clone = Arc::clone(&self.distributed_hashing_table);
        {
            if let Ok(mut dht) = dht_clone.try_lock() {
                if dht.contains_key(&ticket_id) {
                    return Err(tonic::Status::already_exists("Ticket already exists"));
                }

                dht.insert(ticket_id, Ticket::new(nonce, difficulty));
            } else {
                return Err(tonic::Status::aborted(
                    "Failed to lock DHT for inserting ticket",
                ));
            }
        }

        Ok(Response::new(ChallangeResponse {
            challange: nonce,
            difficulty,
        }))
    }

    async fn submit_challange(
        &self,
        request: tonic::Request<SubmitRequest>,
    ) -> Result<tonic::Response<SubmitResponse>, tonic::Status> {
        let request = request.into_inner();

        let pub_key = NodeId::try_from(request.pub_key)
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let ticket_id = NodeId::create_ticket(pub_key.clone());

        let dht_clone = Arc::clone(&self.distributed_hashing_table);
        let mut dht = {
            let Ok(dht) = dht_clone.try_lock() else {
                return Err(tonic::Status::aborted(
                    "Failed to lock DHT for fetching ticket",
                ));
            };

            dht
        };

        let Some(ticket) = dht.get(&ticket_id) else {
            return Err(tonic::Status::not_found("Ticket not found"));
        };

        let ticket = ticket
            .as_any()
            .downcast_ref::<Ticket>()
            .ok_or_else(|| tonic::Status::internal("Failed to fetch the Ticket"))?;

        let prof_of_work = NodeTicket::calculate_pow(
            pub_key.0,
            ticket.nonce,
            request.nonce,
            DoubleHasher::default(),
        );

        if !NodeTicket::validate_pow(&prof_of_work, ticket.difficulty) {
            return Err(tonic::Status::unauthenticated("Prof of work invalid"));
        }

        if let None = dht.remove(&ticket_id) {
            return Err(tonic::Status::aborted("Failed to update tables"));
        }

        let signature = Signature::sign(self.node.keys.clone(), prof_of_work);

        Ok(Response::new(SubmitResponse {
            pubkey: signature.pub_key.into(),
            signature: signature.get_signature().into(),
        }))
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
        let incoming_node = self.get_peer(&request)?;
        let request = request.into_inner();

        let key = NodeId::try_from(request.key.clone())
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let config = bincode::config::standard();
        let Ok((decoded_value, _)) =
            bincode::serde::decode_from_slice::<Box<dyn KademliaData>, _>(&request.value, config)
        else {
            return Err(tonic::Status::aborted("Failed to decode value"));
        };

        {
            let dht_clone = Arc::clone(&self.distributed_hashing_table);
            let Ok(mut dht) = dht_clone.try_lock() else {
                return Err(tonic::Status::aborted(
                    "Failed to lock DHT for storing value",
                ));
            };

            dht.insert(key.clone(), decoded_value.clone());
        }

        let routing_table = Arc::clone(&self.routing_table);

        {
            if let Ok(mut routing_table) = routing_table.try_lock() {
                routing_table.insert_node(&incoming_node).await;
                info!("Persisted incoming node: {:#?}", incoming_node);
            } else {
                info!(
                    "Failed to lock routing table for persisting incoming node: {:#?} On Store",
                    incoming_node
                );
            }
        }

        // persist
        {
            let event_handler = Arc::clone(&self.event_handler);

            event_handler
                .on_event(DHTEvent::Store(decoded_value.clone()))
                .await;
        }

        Ok(Response::new(StoreResponse { key: key.into() }))
    }

    async fn find_node(
        &self,
        request: tonic::Request<FindNodeRequest>,
    ) -> Result<tonic::Response<FindNodeResponse>, tonic::Status> {
        let incoming_node = self.get_peer(&request)?;
        let request = request.into_inner();

        let count = {
            let bytes: &[u8] = &request.count;
            let number = u64::from_le_bytes(bytes.try_into().unwrap());

            number as usize
        }
        .min(KBUCKET_MAX);

        let lookup_id = NodeId::try_from(request.key)
            .map_err(|e| tonic::Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let routing_table = Arc::clone(&self.routing_table);
        let closest_nodes = {
            let Ok(mut routing_table) = routing_table.try_lock() else {
                return Err(tonic::Status::aborted(
                    "Failed to lock routing table for finding nodes",
                ));
            };

            // keep record of the incoming node for future requests
            routing_table.insert_node(&incoming_node).await;
            routing_table.get_closest_nodes(&lookup_id, count).await
        };

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
        let incoming_node = self.get_peer(&request)?;
        let request = request.into_inner();

        let key = NodeId::try_from(request.key)
            .map_err(|e| Status::invalid_argument(format!("Invalid node ID: {}", e)))?;

        let dht_clone = Arc::clone(&self.distributed_hashing_table);
        let dht = {
            let Ok(dht) = dht_clone.try_lock() else {
                return Err(Status::aborted("Failed to lock DHT for finding value"));
            };

            dht
        };

        if let Some(value) = dht.get(&key) {
            let config = bincode::config::standard();

            let Ok(encoded_data) = bincode::serde::encode_to_vec(&value, config) else {
                return Err(Status::aborted("Failed to return value"));
            };

            return Ok(Response::new(FindValueResponse {
                resp: Some(Resp::Value(encoded_data)),
            }));
        };

        let routing_table = Arc::clone(&self.routing_table);
        let closest_nodes = {
            let Ok(mut routing_table) = routing_table.try_lock() else {
                return Err(Status::aborted(
                    "Failed to lock routing table for finding nodes",
                ));
            };

            // keep record of the incoming node for future requests
            routing_table.insert_node(&incoming_node).await;
            routing_table.get_closest_nodes(&key, KBUCKET_MAX).await
        };

        let response = closest_nodes
            .iter()
            .map(|distance| distance.clone().1.into())
            .collect::<Vec<NodeInfo>>();

        Ok(Response::new(FindValueResponse {
            resp: Some(Resp::Nodes(RepetedNode { nodes: response })),
        }))
    }
}
