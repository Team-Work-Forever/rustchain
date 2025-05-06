use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    blockchain::{BlockChain, DoubleHasher},
    kademlia::store::PersistDHTNode,
    store::NetworkNodeStorage,
    DHTNode,
};

use super::network_node::{NetworkMode, NetworkNode};

#[derive(Debug, Error)]
pub enum StoreNodeError {
    #[error("Failed to persist node")]
    PersistError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistNodeNetwork {
    block_chain: BlockChain,
    dht: PersistDHTNode,
}

impl NetworkNode {
    pub async fn persist_node(
        &self,
        storage: impl NetworkNodeStorage,
    ) -> Result<(), StoreNodeError> {
        let persist = {
            let block_chain = self.block_chain.lock().await;

            let kademlia = self.kademlia_net.lock().await;

            let dht = kademlia.into_persist().await;

            Ok(PersistNodeNetwork {
                block_chain: block_chain.clone(),
                dht,
            })
        }?;

        if let Err(_) = storage.store(&persist) {
            return Err(StoreNodeError::PersistError);
        }

        Ok(())
    }

    pub async fn load_node(
        mode: NetworkMode,
        storage: impl NetworkNodeStorage,
    ) -> Option<Arc<Self>> {
        let persist_node = {
            match storage.load::<PersistNodeNetwork>() {
                Ok(persist) => persist,
                Err(_) => {
                    let Some(persist_dht) = PersistDHTNode::new() else {
                        panic!("Couldn't create a node")
                    };

                    PersistNodeNetwork {
                        block_chain: BlockChain::new(),
                        dht: persist_dht,
                    }
                }
            }
        };

        if !persist_node.block_chain.validate(DoubleHasher {}) {
            return None;
        }

        let Some(dht) = DHTNode::from(mode.host.clone(), mode.port, persist_node.dht).await else {
            return None;
        };

        Self::load_from(mode, persist_node.block_chain, dht).await
    }
}
