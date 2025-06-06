pub mod data;
pub mod dht;
pub mod distance;
pub mod event;
pub mod k_bucket;
pub mod network;
pub mod node;
pub mod node_id;
pub mod routing_table;
pub mod secret_key;
pub mod signature;
pub mod store;
pub mod ticket;

pub const NODE_ID_LENGTH: usize = 32;
const NODE_ID_BITS: usize = NODE_ID_LENGTH * 8;

const KBUCKET_MAX: usize = 2;

pub use node::Node;
pub use node_id::NodeId;
pub use routing_table::RoutingTable;
