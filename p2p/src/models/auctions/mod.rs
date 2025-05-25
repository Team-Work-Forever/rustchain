use crate::kademlia::NODE_ID_LENGTH;

pub mod auction;
pub mod bid;
pub mod item;

type Timestamp = i64;
pub type Currency = u32;
pub type PublicKey = [u8; NODE_ID_LENGTH];
