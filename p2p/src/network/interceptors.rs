use std::net::SocketAddr;

use tonic::{metadata::MetadataValue, Request, Status};

use crate::{
    kademlia::{dht::KademliaData, network::GrpcNetwork, NodeId},
    Node,
};

use super::grpc::proto::NodeInfo;

pub(crate) const PUB_KEY_METADATA: &str = "x-pubkey";
pub(crate) const ADDR_KEY_METADATA: &str = "x-addr";

impl<TData: KademliaData> GrpcNetwork<TData> {
    pub(crate) fn get_peer<TRequest>(
        &self,
        request: &tonic::Request<TRequest>,
    ) -> Result<Node, Status> {
        let pub_key = self
            .get_public_key(&request)
            .map_err(|_| Status::aborted("Failed to get public key"))?;

        let peer_addr = self.get_addr(&request)?;

        let node = Node::from(NodeInfo {
            id: NodeId::new(&pub_key).into(),
            addr: peer_addr.ip().to_string(),
            port: peer_addr.port() as u64,
            pub_key: pub_key.into(),
        })
        .ok_or_else(|| Status::cancelled("Failed to get peer information"))?;

        Ok(node)
    }

    pub(crate) fn get_addr<TRequest>(
        &self,
        request: &tonic::Request<TRequest>,
    ) -> Result<SocketAddr, Status> {
        let metadata = request.metadata();

        let addr_val = metadata
            .get(ADDR_KEY_METADATA)
            .ok_or(Status::unauthenticated("Missing node address"))?;

        let addr_str = addr_val
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid addr metadata"))?;

        let socket_addr = addr_str
            .parse::<SocketAddr>()
            .map_err(|_| Status::unauthenticated("Invalid address format"))?;

        Ok(socket_addr)
    }

    pub(crate) fn get_public_key<TRequest>(
        &self,
        request: &tonic::Request<TRequest>,
    ) -> Result<[u8; 32], Status> {
        let metadata = request.metadata();

        let pubkey_val = metadata
            .get(PUB_KEY_METADATA)
            .ok_or(Status::unauthenticated("Missing client pubkey"))?;

        let pubkey_str = pubkey_val
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid metadata format"))?;

        let decoded =
            hex::decode(pubkey_str).map_err(|_| Status::unauthenticated("Hex decode failed"))?;

        if decoded.len() != 32 {
            return Err(Status::unauthenticated("Bad pubkey length"));
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&decoded);

        Ok(pubkey)
    }

    pub(crate) fn add_pubkey_interceptor(
        public_key: [u8; 32],
        addr: SocketAddr,
    ) -> impl FnMut(Request<()>) -> Result<Request<()>, Status> {
        let encoded_pubkey = hex::encode(public_key);
        let encoded_addr = addr.to_string();

        move |mut req: Request<()>| {
            let pub_meta_value = MetadataValue::try_from(encoded_pubkey.clone())
                .map_err(|_| Status::internal("Invalid public key"))?;

            req.metadata_mut().insert(PUB_KEY_METADATA, pub_meta_value);

            let addr_meta_value = MetadataValue::try_from(encoded_addr.clone())
                .map_err(|_| Status::internal("Invalid address key"))?;

            req.metadata_mut()
                .insert(ADDR_KEY_METADATA, addr_meta_value);

            Ok(req)
        }
    }
}
