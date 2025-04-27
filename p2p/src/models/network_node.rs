use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::{self, Duration},
};

use rand::{rng, seq::IndexedRandom};

use crate::{blockchain::BlockChain, DHTNode, Node};

pub const BATCH_PULLING_SIZE: usize = 15;
pub const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(2 * 60);

pub enum NetworkMode {
    Bootstrap {
        host: String,
        port: usize,
    },
    Join {
        bootstraps: Vec<Node>,
        host: String,
        port: usize,
    },
    Client {
        bootstraps: Vec<Node>,
        host: String,
        port: usize,
    },
}

#[derive(Debug)]
pub struct NetworkNode {
    pub block_chain: Arc<Mutex<BlockChain>>,
    pub kademlia_net: Arc<Mutex<DHTNode>>,
}

impl NetworkNode {
    pub(crate) async fn load_from(
        mode: NetworkMode,
        block_chain: BlockChain,
        dht: DHTNode,
    ) -> Option<Arc<Self>> {
        let block_chain = Arc::new(Mutex::new(block_chain));

        let dht = Arc::new(Mutex::new(dht));
        let network_node = Self {
            block_chain,
            kademlia_net: dht.clone(),
        };

        let network_node = Arc::new(network_node);
        NetworkNode::connect(network_node.clone());

        match mode {
            NetworkMode::Join { bootstraps, .. } | NetworkMode::Client { bootstraps, .. } => {
                let Some(bootstrap) = bootstraps.choose(&mut rng()).cloned() else {
                    return None;
                };

                if let None = dht
                    .lock()
                    .expect("Tried to unlock dht")
                    .join_network(bootstrap)
                    .await
                {
                    return None;
                }
            }
            _ => {}
        };

        Some(network_node)
    }

    pub async fn new(mode: NetworkMode) -> Option<Arc<Self>> {
        let (host, port) = match &mode {
            NetworkMode::Bootstrap { host, port } => (host.clone(), *port),
            NetworkMode::Join { host, port, .. } => (host.clone(), *port),
            NetworkMode::Client { host, port, .. } => (host.clone(), *port),
        };

        let Some(dht) = DHTNode::new(host, port).await else {
            return None;
        };

        Self::load_from(mode, BlockChain::new(), dht).await
    }

    pub(crate) fn connect(self: Arc<Self>) {
        self.kademlia_net
            .lock()
            .expect("Tried to unlock dht")
            .init_grpc_connection(self.clone());

        BlockChain::start_miner(
            self.block_chain.clone(),
            self,
            BATCH_PULLING_SIZE,
            BATCH_PULLING_TIME_FRAME,
        );
    }

    pub fn get_connection(&self) -> Result<Node, Box<dyn Error + '_>> {
        let kademlia = self.kademlia_net.lock()?;

        let public_key = kademlia.core.keys.public_key;
        let addr = kademlia.core.get_addr()?;

        Ok(Node::from_pub_key(
            &public_key,
            addr.ip().to_string(),
            addr.port() as usize,
        ))
    }
}
