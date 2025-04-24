use std::sync::{Arc, Mutex};

use crate::{blockchain::BlockChain, DHTNode};

pub struct NetworkNode {
    pub block_chain: Arc<Mutex<BlockChain>>,
    pub kademlia_net: Arc<Mutex<DHTNode>>,
}
