use std::net::SocketAddr;

use tonic::{metadata::MetadataValue, Code, Request, Status};

use crate::{
    blockchain::DoubleHasher,
    kademlia::{network::GrpcNetwork, ticket::NodeTicket, NodeId},
    Node,
};

use super::grpc::proto::NodeInfo;

pub(crate) const PUB_KEY_METADATA: &str = "x-pubkey";
pub(crate) const ADDR_KEY_METADATA: &str = "x-addr";

pub(crate) const TICKET_POW_METADATA: &str = "x-pow";
pub(crate) const TICKET_NONCE_METADATA: &str = "x-nonce";
pub(crate) const TICKET_CHALLANGE_METADATA: &str = "x-challange";

impl GrpcNetwork {
    pub(crate) fn get_peer<TRequest>(
        &self,
        request: &tonic::Request<TRequest>,
    ) -> Result<Node, Status> {
        let pub_key = Self::get_public_key(&request)
            .map_err(|_| Status::aborted("Failed to get public key"))?;

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

    pub(crate) fn get_number<TRequest>(
        request: &tonic::Request<TRequest>,
        key: &str,
    ) -> Result<u32, Status> {
        let metadata = request.metadata();

        let number_val = metadata
            .get(key)
            .ok_or(Status::unauthenticated("Missing metadata key"))?;

        let number_str = number_val
            .to_str()
            .map_err(|_| Status::invalid_argument("Metadata value is not a valid UTF-8 string"))?;

        let number = number_str
            .parse::<u32>()
            .map_err(|_| Status::invalid_argument("Failed to parse metadata value as u32"))?;

        Ok(number)
    }

    pub(crate) fn get_addr<TRequest>(
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

    pub(crate) fn get_pow<TRequest>(
        request: &tonic::Request<TRequest>,
    ) -> Result<[u8; 32], Status> {
        let metadata = request.metadata();

        let pubkey_val = metadata
            .get(TICKET_POW_METADATA)
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
        ticket: NodeTicket,
        addr: SocketAddr,
    ) -> impl FnMut(Request<()>) -> Result<Request<()>, Status> {
        let encoded_pubkey = hex::encode(public_key);
        let encoded_addr = addr.to_string();
        let encoded_pow = hex::encode(ticket.pow);
        let encoded_nonce = ticket.nonce;
        let encoded_challange = ticket.challange;

        move |mut req: Request<()>| {
            let pub_meta_value = MetadataValue::try_from(encoded_pubkey.clone())
                .map_err(|_| Status::internal("Invalid public key"))?;

            req.metadata_mut().insert(PUB_KEY_METADATA, pub_meta_value);

            let addr_meta_value = MetadataValue::try_from(encoded_addr.clone())
                .map_err(|_| Status::internal("Invalid address key"))?;

            req.metadata_mut()
                .insert(ADDR_KEY_METADATA, addr_meta_value);

            let pow_value = MetadataValue::try_from(encoded_pow.clone())
                .map_err(|_| Status::internal("Invalid pow key"))?;

            req.metadata_mut().insert(TICKET_POW_METADATA, pow_value);

            let nonce_value = MetadataValue::try_from(encoded_nonce.clone())
                .map_err(|_| Status::internal("Invalid nonce key"))?;

            req.metadata_mut()
                .insert(TICKET_NONCE_METADATA, nonce_value);

            let nonce_value = MetadataValue::try_from(encoded_challange.clone())
                .map_err(|_| Status::internal("Invalid challange key"))?;

            req.metadata_mut()
                .insert(TICKET_CHALLANGE_METADATA, nonce_value);

            Ok(req)
        }
    }

    pub fn verify_sybil_attack(request: Request<()>) -> Result<Request<()>, Status> {
        let pub_key = Self::get_public_key(&request)?;
        let challange = Self::get_number(&request, TICKET_CHALLANGE_METADATA)?;
        let nonce = Self::get_number(&request, TICKET_NONCE_METADATA)?;

        let prof_of_work = Self::get_pow(&request)?;

        let calculate_pow =
            NodeTicket::calculate_pow(pub_key, challange, nonce, DoubleHasher::default());

        if calculate_pow != prof_of_work {
            return Err(Status::new(
                Code::Unauthenticated,
                "Failed the Prof of Work",
            ));
        }

        println!("Pois: {}", hex::encode(calculate_pow));

        Ok(request)
    }
}
