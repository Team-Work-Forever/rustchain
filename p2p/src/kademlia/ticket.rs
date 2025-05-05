use ed25519_dalek::SIGNATURE_LENGTH;
use serde::{Deserialize, Serialize};

use crate::{
    blockchain::{DoubleHasher, HashFunc},
    kademlia::dht::KademliaError,
    network::grpc::proto::{ChallangeRequest, SubmitRequest},
    utils,
};

use super::{
    network::GrpcNetwork,
    signature::{HandleSignature, Signature},
    Node, NODE_ID_LENGTH,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeTicket {
    pub pow: [u8; 32],
    pub challange: u32,
    pub nonce: u32,
    pub signature: Option<Signature>,
}

impl std::fmt::Debug for NodeTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeTicket")
            .field("pow", &hex::encode(&self.pow))
            .field("challange", &self.challange)
            .field("nonce", &self.nonce)
            .field("signature", &self.signature)
            .finish()
    }
}

impl NodeTicket {
    fn new(pow: [u8; 32], challange: u32, nonce: u32) -> Self {
        Self {
            pow,
            challange,
            nonce,
            signature: None,
        }
    }

    pub fn set_signature(
        &mut self,
        pub_key: [u8; NODE_ID_LENGTH],
        signature: [u8; SIGNATURE_LENGTH],
    ) {
        self.signature = Some(Signature::from(pub_key, signature))
    }

    pub fn validate_signature(&self, pub_key: Option<[u8; 32]>) -> bool {
        let Some(signature) = self.signature.as_ref() else {
            return false;
        };

        if let Some(pub_key) = pub_key {
            return signature.validate_signature(pub_key, self.pow);
        }

        signature.validate_signature(signature.pub_key, self.pow)
    }

    pub fn calculate_pow(
        pub_key: [u8; 32],
        challange: u32,
        nonce: u32,
        hasher: impl HashFunc,
    ) -> [u8; 32] {
        let input = format!("{}{}{}", hex::encode(pub_key), challange, nonce);
        hasher.hash(input)
    }

    pub fn validate_pow(hash: &[u8; 32], difficulty: u32) -> bool {
        let nibbles = difficulty as usize;
        let full_bytes = nibbles / 2;
        let has_half_nibble = nibbles % 2 == 1;

        for i in 0..full_bytes {
            if hash[i] != 0 {
                return false;
            }
        }

        if has_half_nibble {
            if (hash[full_bytes] >> 4) != 0 {
                return false;
            }
        }

        true
    }

    fn brute_force_pow(
        pub_key: [u8; 32],
        challange: u32,
        dificulty: u32,
        hasher: impl HashFunc,
    ) -> (u32, [u8; 32]) {
        let mut prof_of_work: [u8; 32];
        let mut nonce: u32 = 0;

        loop {
            prof_of_work = Self::calculate_pow(pub_key, challange, nonce, hasher.clone());

            if Self::validate_pow(&prof_of_work, dificulty) {
                return (nonce, prof_of_work);
            }

            nonce = nonce.wrapping_add(1);
        }
    }

    pub async fn request_challange(host: &Node, bootstrap: &Node) -> Option<NodeTicket> {
        let Ok(mut client) = GrpcNetwork::handshake(bootstrap.clone())
            .await
            .map_err(|_| {
                return KademliaError::PingFailedError;
            })
        else {
            return None;
        };

        let response = match client
            .request_challange(ChallangeRequest {
                pub_key: host.keys.public_key.into(),
            })
            .await
        {
            Ok(request) => request.into_inner(),
            Err(_) => return None,
        };

        let (nonce, pow) = Self::brute_force_pow(
            host.keys.public_key,
            response.challange,
            response.difficulty,
            DoubleHasher::default(),
        );

        Some(NodeTicket::new(pow, response.challange, nonce))
    }

    pub async fn submit_challange(&mut self, host: &mut Node, bootstrap: &Node) -> Option<()> {
        let Ok(mut client) = GrpcNetwork::handshake(bootstrap.clone())
            .await
            .map_err(|_| {
                return KademliaError::PingFailedError;
            })
        else {
            return None;
        };

        let Ok(response) = client
            .submit_challange(SubmitRequest {
                pub_key: host.keys.public_key.into(),
                challenge: self.pow.into(),
                nonce: self.nonce as u32,
            })
            .await
        else {
            return None;
        };

        let response = response.into_inner();

        self.set_signature(
            utils::to_32bytes(response.pubkey)?,
            utils::to_64bytes(response.signature)?,
        );

        if !self.validate_signature(Some(bootstrap.keys.public_key)) {
            return None;
        }

        host.set_ticket(self);
        Some(())
    }
}
