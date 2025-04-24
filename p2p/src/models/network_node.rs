use std::sync::{Arc, Mutex};

use crate::blockchain::BlockChain;

pub struct NetworkNode {
    pub block_chain: Arc<Mutex<BlockChain>>,
}
