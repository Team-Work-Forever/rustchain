use std::{collections::HashMap, sync::Arc};

use proto::{
    kademlia_service_client::KademliaServiceClient, kademlia_service_server::KademliaServiceServer,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tonic::{
    service::{interceptor::InterceptedService, Interceptor},
    transport::{Channel, Server},
};

use crate::{
    kademlia::{
        dht::KademliaData, event::DHTEventHandler, network::GrpcNetwork, NodeId, RoutingTable,
    },
    Node,
};

pub mod proto {
    tonic::include_proto!("kademlia");
}

#[derive(Debug, Error)]
pub enum NetWorkError {
    #[error("Failed to get the ip address")]
    FailedToFetchIp,

    #[error("Failed to establish connection with peer")]
    FailToEstablishConnection,
}

impl GrpcNetwork {
    pub async fn start_network(
        node: Node,
        routing_table: Arc<Mutex<RoutingTable>>,
        distributed_hash_table: Arc<Mutex<HashMap<NodeId, Box<dyn KademliaData>>>>,
        event_bus: Arc<dyn DHTEventHandler>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let grpc_kademlia = GrpcNetwork::new(
            node.clone(),
            routing_table.clone(),
            distributed_hash_table,
            event_bus,
        );

        let node_addr = node.get_addr()?;

        Server::builder()
            .add_service(KademliaServiceServer::new(grpc_kademlia))
            .serve(node_addr)
            .await?;

        Ok(())
    }

    pub async fn connect_over(
        host: Node,
        target: Node,
    ) -> Result<KademliaServiceClient<InterceptedService<Channel, impl Interceptor>>, NetWorkError>
    {
        let Ok(host_addr) = host.get_addr() else {
            return Err(NetWorkError::FailedToFetchIp);
        };

        let Ok(target_addr) = target.get_addr() else {
            return Err(NetWorkError::FailedToFetchIp);
        };

        let url = format!("http://{}:{}", target_addr.ip(), target_addr.port());

        let channel = Channel::from_shared(url)
            .map_err(|_| NetWorkError::FailToEstablishConnection)?
            .connect()
            .await
            .map_err(|_| NetWorkError::FailToEstablishConnection)?;

        let client = KademliaServiceClient::with_interceptor(
            channel,
            Self::add_pubkey_interceptor(host.keys.public_key, host_addr),
        );

        Ok(client)
    }
}
