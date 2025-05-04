use std::fmt::Debug;

use tonic::async_trait;

use super::Block;

#[derive(Debug)]
pub enum BlockChainEvent {
    AddBlock(Block),
}

#[async_trait]
pub trait BlockChainEventHandler: Debug + Send + Sync {
    async fn on_event(&self, event: BlockChainEvent);
}
