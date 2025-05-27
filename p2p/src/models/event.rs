use std::{env, sync::Arc};

use log::info;
use tonic::async_trait;

use crate::{
    blockchain::{Block, BlockChainError, BlockChainEvent, BlockChainEventHandler, BlockHeader},
    kademlia::{
        event::{DHTEvent, DHTEventHandler},
        NodeId,
    },
    models::network_node::MAX_TTL,
    store::InFileStorage,
    vars,
};

use super::network_node::NetworkNode;

#[async_trait]
impl DHTEventHandler for NetworkNode {
    async fn on_event(&self, event: DHTEvent) {
        match event {
            DHTEvent::Store(kademlia_data) => {
                if let Some(header) = kademlia_data.as_any().downcast_ref::<BlockHeader>() {
                    info!("Chain tip recived Recived! {:#?}", header);
                }

                if let Some(block) = kademlia_data.as_any().downcast_ref::<Block>() {
                    info!("Block Recived!");

                    let block_chain_tx = Arc::clone(&self.block_chain);
                    let Ok(mut block_chain) = block_chain_tx.try_lock() else {
                        return;
                    };

                    match block_chain.append_block(block) {
                        Ok(_) => {
                            let block_key = NodeId::new(&block.header.hash);
                            let block = block.clone();

                            let kademlia = Arc::clone(&self.kademlia_net);
                            tokio::spawn(async move {
                                if let Ok(kademlia) = kademlia.try_lock() {
                                    let _ = kademlia.store(&block_key, Box::new(block)).await;
                                    info!("Block repropagated to the network");
                                }
                            });
                        }
                        Err(BlockChainError::ChainBroken) => {
                            let last_key = NodeId::new(&block.header.hash);
                            for incoming in self.fetch_block_chain(&last_key, MAX_TTL).await {
                                if let None = block_chain.remove_last() {
                                    break;
                                }

                                match block_chain.append_block(&incoming) {
                                    Ok(ok) => ok,
                                    _ => break,
                                };
                            }
                        }
                        Err(BlockChainError::InvalidBlock) => info!("invalid block"), // PoR - decrease peer's score
                        Err(BlockChainError::BlockAlreadyPersisted) => {
                            info!("Block already persisted")
                        }
                        Err(BlockChainError::BlockNotFound) => {
                            info!("Failed to fetch block")
                        }
                    }
                }
            }
        }
    }
}

impl NetworkNode {
    async fn persist_state(&self) {
        info!("Persisting state....");
        let Ok(storage_path) = env::var(vars::STORAGE_PATH) else {
            info!("Failed to fetch env var with the storage path!");
            return;
        };

        let storage = InFileStorage::new(storage_path);
        if let Err(_) = self.persist_node(storage).await {
            info!("Failed to persist node state");
        }

        info!("State persisted!");
    }

    pub async fn update_global_bc_head(&self, block: &BlockHeader) {
        let kademlia_net = Arc::clone(&self.kademlia_net);
        let last_key = {
            let Ok(kademlia) = kademlia_net.try_lock() else {
                return;
            };

            // info!("Chain Tip Key: {}", hex::encode(kademlia.core.id.0));
            NodeId::create_chain_head(kademlia.core.id.clone())
        };

        let kademlia_net = Arc::clone(&self.kademlia_net);
        let store_stuff = {
            let Ok(kademlia) = kademlia_net.try_lock() else {
                return;
            };

            kademlia.store(&last_key, Box::new(block.clone())).await
        };

        if let Err(_) = store_stuff {
            // make a list to retry at least 3 times, with a 3 second span
            // implement like a try 3 times over thingy
            info!("Failed to store block: chain thread")
        }

        info!("Update Chain Head on network");
    }

    async fn fix_block_chain(&self, last_block: &BlockHeader) {
        let block_chain = Arc::clone(&self.block_chain);
        let last_key = NodeId::new(&last_block.hash);
        let Ok(mut block_chain) = block_chain.try_lock() else {
            return;
        };

        for incoming in self.fetch_block_chain(&last_key, MAX_TTL).await {
            if let None = block_chain.remove_last() {
                break;
            }

            match block_chain.append_block(&incoming) {
                Ok(ok) => ok,
                _ => break,
            };
        }

        info!("Fix chain");
    }
}

#[async_trait]
impl BlockChainEventHandler for NetworkNode {
    async fn on_event(&self, event: BlockChainEvent) {
        match event {
            BlockChainEvent::AddBlock(block) => {
                info!("recive block...");
                let block_key = NodeId::new(&block.header.hash);

                let last_block = {
                    let Some(last_block) = self.fetch_last_block_header(block.clone()).await else {
                        return;
                    };

                    last_block
                };

                if last_block.hash != block.header.hash {
                    self.fix_block_chain(&last_block).await;
                } else {
                    self.update_global_bc_head(&block.header).await;
                }

                let kademlia_net = Arc::clone(&self.kademlia_net);
                let propagate_block = {
                    let Ok(kademlia) = kademlia_net.try_lock() else {
                        return;
                    };

                    kademlia.store(&block_key, Box::new(block.clone())).await
                };

                if let Err(_) = propagate_block {
                    // make a list to retry at least 3 times, with a 3 second span
                    // implement like a try 3 times over thingy
                    info!("Failed to propagate block: chain thread")
                }

                {
                    self.persist_state().await;
                }

                info!("Propagating block...")
            }
        }
    }
}
