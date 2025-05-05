use std::net::SocketAddr;

use bincode::config;
use serde::{Deserialize, Serialize};
use tonic::{metadata::MetadataValue, Request, Status};

use crate::{
    blockchain::DoubleHasher,
    kademlia::{network::GrpcNetwork, ticket::NodeTicket, NodeId, NODE_ID_LENGTH},
    Node,
};

use super::grpc::proto::NodeInfo;

pub(crate) const PUB_KEY_METADATA: &str = "x-pubkey";
pub(crate) const ADDR_KEY_METADATA: &str = "x-addr";
pub(crate) const TICKET_KEY_METADATA: &str = "x-auth-ticket";

impl GrpcNetwork {
    pub(crate) fn get_peer<TRequest>(
        &self,
        request: &tonic::Request<TRequest>,
    ) -> Result<Node, Status> {
        let pub_key = Self::get_from_metada::<[u8; NODE_ID_LENGTH], _>(
            &request,
            PUB_KEY_METADATA,
            "cannot parse public key".into(),
        )
        .map_err(|_| Status::aborted("cannot parse public key"))?;

        let peer_addr = Self::get_addr(&request)?;

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
        request: &tonic::Request<TRequest>,
    ) -> Result<SocketAddr, Status> {
        let address = Self::get_from_metada::<String, _>(
            &request,
            ADDR_KEY_METADATA,
            "cannot parse address".into(),
        )
        .map_err(|_| Status::aborted("cannot parse address"))?;

        let socket_addr = address
            .parse::<SocketAddr>()
            .map_err(|_| Status::unauthenticated("Invalid address format"))?;

        Ok(socket_addr)
    }

    fn get_from_metada<TData, TRequest>(
        request: &tonic::Request<TRequest>,
        key: &'static str,
        error_msg: String,
    ) -> Result<TData, Status>
    where
        TData: Serialize + for<'de> Deserialize<'de>,
    {
        let metadata = request.metadata();

        let config = config::standard();
        let hex_value = metadata
            .get(key)
            .ok_or(Status::unauthenticated("Missing client pubkey"))?;

        let encoded_value =
            hex::decode(hex_value).map_err(|_| Status::aborted("Failed to decoded from hex"))?;

        match bincode::serde::decode_from_slice::<TData, _>(&encoded_value, config) {
            Ok((block_chain, _)) => Ok(block_chain),
            Err(_) => Err(Status::aborted(error_msg)),
        }
    }

    fn add_to_metadata<TData>(
        request: &mut Request<()>,
        key: &'static str,
        value: TData,
        error_msg: String,
    ) -> Result<(), Status>
    where
        TData: Serialize + for<'de> Deserialize<'de>,
    {
        let config = config::standard();
        let encoded_value = match bincode::serde::encode_to_vec(&value, config) {
            Ok(e) => e,
            Err(_) => panic!("Failed to append data to metadata"),
        };

        let meta_value = MetadataValue::try_from(hex::encode(encoded_value))
            .map_err(|_| Status::internal(error_msg))?;

        request.metadata_mut().insert(key, meta_value);

        Ok(())
    }

    pub(crate) fn add_pubkey_interceptor(
        public_key: [u8; 32],
        ticket: NodeTicket,
        addr: SocketAddr,
    ) -> impl FnMut(Request<()>) -> Result<Request<()>, Status> {
        move |mut req: Request<()>| {
            GrpcNetwork::add_to_metadata(
                &mut req,
                PUB_KEY_METADATA,
                public_key,
                "Invalid public key".into(),
            )?;

            GrpcNetwork::add_to_metadata(
                &mut req,
                ADDR_KEY_METADATA,
                addr.to_string(),
                "Invalid address key".into(),
            )?;

            GrpcNetwork::add_to_metadata(
                &mut req,
                TICKET_KEY_METADATA,
                ticket.clone(),
                "Invalid ticket".into(),
            )?;

            Ok(req)
        }
    }

    pub fn verify_sybil_attack(request: Request<()>) -> Result<Request<()>, Status> {
        let pub_key = Self::get_from_metada::<[u8; NODE_ID_LENGTH], _>(
            &request,
            PUB_KEY_METADATA,
            "cannot parse public key".into(),
        )
        .map_err(|_| Status::aborted("cannot parse public key"))?;

        let ticket = Self::get_from_metada::<NodeTicket, _>(
            &request,
            TICKET_KEY_METADATA,
            "cannot parse ticket".into(),
        )
        .map_err(|_| Status::aborted("cannot parse ticket"))?;

        if !ticket.validate_signature(None) {
            return Err(Status::aborted("ticket not valid!"));
        }

        let calculate_pow = NodeTicket::calculate_pow(
            pub_key,
            ticket.challange,
            ticket.nonce,
            DoubleHasher::default(),
        );

        if calculate_pow != ticket.pow {
            return Err(Status::unauthenticated("Failed the Prof of Work"));
        }

        Ok(request)
    }
}
