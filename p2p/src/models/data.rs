use serde::{Deserialize, Serialize};

use crate::kademlia::dht::KademliaData;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KData {
    pub name: String,
}

impl KData {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[typetag::serde]
impl KademliaData for KData {
    fn clone_dyn(&self) -> Box<dyn KademliaData> {
        Box::new(self.clone())
    }
}
