use std::fmt::Debug;

use tonic::async_trait;

use super::data::KademliaData;

#[derive(Debug)]
pub enum DHTEvent {
    Store(Box<dyn KademliaData>),
}

#[async_trait]
pub trait DHTEventHandler: Debug + Send + Sync {
    async fn on_event(&self, event: DHTEvent);
}
