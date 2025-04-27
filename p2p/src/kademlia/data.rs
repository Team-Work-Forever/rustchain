use std::{any::Any, fmt::Debug};

use serde::{Deserialize, Serialize};

#[typetag::serde]
pub trait KademliaData: erased_serde::Serialize + Debug + Send + Sync + 'static {
    fn clone_dyn(&self) -> Box<dyn KademliaData>;
    fn as_any(&self) -> &dyn Any;
}

impl Clone for Box<dyn KademliaData> {
    fn clone(&self) -> Self {
        self.clone_dyn()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticket {
    pub nonce: u32,
    pub difficulty: u32,
}

impl Ticket {
    pub fn new(nonce: u32, difficulty: u32) -> Box<Self> {
        Box::new(Self { nonce, difficulty })
    }
}

#[typetag::serde]
impl KademliaData for Ticket {
    fn clone_dyn(&self) -> Box<dyn KademliaData> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
