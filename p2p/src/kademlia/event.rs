use std::fmt::Debug;

use super::data::KademliaData;

#[derive(Debug)]
pub enum DHTEvent {
    Store(Box<dyn KademliaData>),
}

pub trait DHTEventHandler: Debug + Send + Sync {
    fn on_event(&self, event: DHTEvent);
}
