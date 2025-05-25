use std::sync::Arc;

use log::info;
use tonic::async_trait;

use crate::{
    blockchain::{Block, BlockChainError, BlockChainEvent, BlockChainEventHandler, BlockHeader},
    kademlia::{
        event::{DHTEvent, DHTEventHandler},
        NodeId,
    },
    models::network_node::MAX_TTL,
};

use super::network_node::NetworkNode;

#[async_trait]
impl DHTEventHandler for NetworkNode {
    async fn on_event(&self, event: DHTEvent) {
        match event {
            DHTEvent::Store(kademlia_data) => {
                let mut block_chain = self.block_chain.lock().await;

                if let Some(block) = kademlia_data.as_any().downcast_ref::<Block>() {
                    info!("Block Recived!");
                    match block_chain.append_block(block) {
                        Ok(_) => {
                            let block_key = NodeId::new(&block.header.hash);
                            let block = block.clone();

                            let kademlia = Arc::clone(&self.kademlia_net);
                            tokio::spawn(async move {
                                let kademlia = kademlia.lock().await;
                                let _ = kademlia.store(&block_key, Box::new(block)).await;
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
    pub async fn update_global_bc_head(&self, block: &BlockHeader) {
        let last_key = {
            let kademlia = self.kademlia_net.lock().await;
            NodeId::create_chain_head(kademlia.core.id.clone())
        };

        let store_stuff = {
            let kademlia = self.kademlia_net.lock().await;
            kademlia.store(&last_key, Box::new(block.clone())).await
        };

        if let Err(_) = store_stuff {
            // make a list to retry at least 3 times, with a 3 second span
            // implement like a try 3 times over thingy
            info!("Failed to store block: chain thread")
        }
    }

    async fn fix_block_chain(&self, last_block: &BlockHeader) {
        let last_key = NodeId::new(&last_block.hash);
        let mut block_chain = self.block_chain.lock().await;

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
}

#[async_trait]
impl BlockChainEventHandler for NetworkNode {
    async fn on_event(&self, event: BlockChainEvent) {
        match event {
            BlockChainEvent::AddBlock(block) => {
                let block_key = NodeId::new(&block.header.hash);

                let Some(last_block) = self.fetch_last_block_header(block.clone()).await else {
                    return;
                };

                if last_block.hash != block.header.hash {
                    self.fix_block_chain(&last_block).await;
                } else {
                    self.update_global_bc_head(&block.header).await;
                }

                let propagate_block = {
                    let kademlia = self.kademlia_net.lock().await;
                    kademlia.store(&block_key, Box::new(block.clone())).await
                };

                if let Err(_) = propagate_block {
                    // make a list to retry at least 3 times, with a 3 second span
                    // implement like a try 3 times over thingy
                    info!("Failed to propagate block: chain thread")
                }

                info!("Propagating block...")
            }
        }
    }
}
