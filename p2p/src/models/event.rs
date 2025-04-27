use crate::{
    blockchain::{BlockChainEvent, BlockChainEventHandler},
    kademlia::event::{DHTEvent, DHTEventHandler},
};

use super::network_node::NetworkNode;

impl DHTEventHandler for NetworkNode {
    fn on_event(&self, event: crate::kademlia::event::DHTEvent) {
        match event {
            DHTEvent::Store(kademlia_data) => {
                println!("Roger, Roger: {:#?}", kademlia_data)
            }
        }
    }
}

impl BlockChainEventHandler for NetworkNode {
    fn on_event(&self, event: crate::blockchain::BlockChainEvent) {
        match event {
            BlockChainEvent::AddBlock(_block) => {
                println!("Block mined!");
            }
        }
    }
}
