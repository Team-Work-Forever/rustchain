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
        if let Err(_) = self.sync().await {
            info!("Failed to sync with the network");
            return;
        }

        match event {
            DHTEvent::Store(kademlia_data) => {
                let check_block_filter =
                    if let Some(header) = kademlia_data.as_any().downcast_ref::<BlockHeader>() {
                        info!("Chain tip recived Recived! {:#?}", header);

                        let block_id = NodeId::new(&header.hash);
                        let Some(block) = self.search_for_block(&block_id).await else {
                            info!("Failed to fetch block with id: {:#?}", block_id);
                            return;
                        };

                        Some(block)
                    } else if let Some(block) = kademlia_data.as_any().downcast_ref::<Block>() {
                        Some(block.clone())
                    } else {
                        info!("Received data is not a Block or BlockHeader");
                        None
                    };

                if let Some(block) = check_block_filter {
                    info!("Block Recived!");
                    let block_chain_tx = Arc::clone(&self.block_chain);

                    {
                        let Ok(mut block_chain) = block_chain_tx.try_lock() else {
                            return;
                        };

                        match block_chain.append_block(&block) {
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
                                info!("Chain broken mate, fixing it...");
                                self.fix_block_chain(&block.header).await;
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

                self.persist_state().await;
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
            info!("Failed to store block: chain thread")
        }

        info!("Update Chain Head on network");
    }

    pub async fn fix_block_chain(&self, last_block: &BlockHeader) {
        let block_chain = Arc::clone(&self.block_chain);
        let last_key = NodeId::new(&last_block.hash);

        for incoming in self.fetch_block_chain(&last_key, MAX_TTL).await {
            {
                let Ok(mut block_chain) = block_chain.try_lock() else {
                    info!("Failed to lock block chain");
                    continue;
                };

                if let None = block_chain.remove_last() {
                    info!("Failed to remove last block from the chain");
                    continue;
                }

                match block_chain.append_block(&incoming) {
                    Ok(ok) => ok,
                    _ => {
                        info!("Failed to remove last block from the chain");
                        continue;
                    }
                };
            };
        }
    }
}

#[async_trait]
impl BlockChainEventHandler for NetworkNode {
    async fn on_event(&self, event: BlockChainEvent) {
        if let Err(_) = self.sync().await {
            info!("Failed to sync with the network");
            return;
        }

        match event {
            BlockChainEvent::AddBlock(block) => {
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
