use std::any::Any;

use crate::{
    blockchain::{Block, BlockHeader},
    kademlia::data::KademliaData,
};

#[typetag::serde]
impl KademliaData for BlockHeader {
    fn clone_dyn(&self) -> Box<dyn KademliaData> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[typetag::serde]
impl KademliaData for Block {
    fn clone_dyn(&self) -> Box<dyn KademliaData> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
