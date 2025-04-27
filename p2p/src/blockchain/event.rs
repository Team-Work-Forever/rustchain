use std::fmt::Debug;

use super::Block;

#[derive(Debug)]
pub enum BlockChainEvent {
    AddBlock(Block),
}

pub trait BlockChainEventHandler: Debug + Send + Sync {
    fn on_event(&self, event: BlockChainEvent);
}
