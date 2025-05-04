use std::{
    collections::HashSet,
    error::Error,
    sync::Arc,
    time::{self, Duration},
};

use rand::{rng, seq::IndexedRandom};
use tokio::sync::Mutex;

use crate::{
    blockchain::{Block, BlockChain, BlockChainEventHandler, BlockHeader, DoubleHasher, HashFunc},
    kademlia::{event::DHTEventHandler, NodeId},
    DHTNode, Node,
};

pub const BATCH_PULLING_SIZE: usize = 15;
pub const MAX_TTL: u32 = 1024;
pub const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(1 * 60);

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
            kademlia_net: Arc::clone(&dht),
        };

        let network_node = Arc::new(network_node);
        NetworkNode::connect(Arc::clone(&network_node)).await;

        match mode {
            NetworkMode::Join { bootstraps, .. } | NetworkMode::Client { bootstraps, .. } => {
                let Some(bootstrap) = bootstraps.choose(&mut rng()).cloned() else {
                    return None;
                };

                if let None = dht.lock().await.join_network(bootstrap).await {
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

    pub(crate) async fn connect(self: Arc<Self>) {
        let handler: Arc<dyn DHTEventHandler> = Arc::clone(&self) as Arc<dyn DHTEventHandler>;
        let kademlia = self.kademlia_net.lock().await;

        kademlia.init_grpc_connection(Arc::clone(&handler));

        let handler: Arc<dyn BlockChainEventHandler> =
            Arc::clone(&self) as Arc<dyn BlockChainEventHandler>;

        BlockChain::start_miner(
            kademlia.core.keys.clone(),
            self.block_chain.clone(),
            handler,
            BATCH_PULLING_SIZE,
            BATCH_PULLING_TIME_FRAME,
        );
    }

    pub async fn search_for_block(&self, block_hash: &NodeId) -> Option<Block> {
        let kademlia = self.kademlia_net.lock().await;

        let fetch_block = match kademlia.find_value(block_hash).await {
            Ok(values) => values,
            Err(_) => return None,
        };

        let Some(block) = fetch_block else {
            return None;
        };

        if let Some(block) = block.as_any().downcast_ref::<Block>() {
            return Some(block.clone());
        }

        None
    }

    pub async fn fetch_block_chain(
        &self,
        search_block_hash: &NodeId,
        ttl: u32,
    ) -> impl Iterator<Item = Block> {
        let block_chain = self.block_chain.lock().await;

        let mut counter = 0;
        let mut founded_blocks = Vec::<Block>::new();
        let mut visited = HashSet::new();

        let Some(goal_block) = block_chain.get_last_block() else {
            return vec![].into_iter();
        };

        let Some(mut block) = self.search_for_block(search_block_hash).await else {
            return Vec::new().into_iter();
        };

        while counter < ttl {
            let current_hash = NodeId::new(&block.header.hash);

            if !visited.insert(current_hash.clone()) {
                break;
            }

            founded_blocks.push(block.clone());

            if goal_block.header.hash == block.header.prev_hash {
                break;
            }

            let prev_hash = NodeId::new(&block.header.prev_hash);

            let Some(prev_block) = self.search_for_block(&prev_hash).await else {
                break;
            };

            block = prev_block;
            counter += 1;
        }

        founded_blocks.reverse();
        founded_blocks.into_iter()
    }

    pub async fn sync(&self) -> Result<(), ()> {
        let fetch_chain_head = {
            let block_chain = self.block_chain.lock().await;
            block_chain.get_last_block().cloned()
        };

        let Some(chain_head) = fetch_chain_head else {
            return Err(());
        };

        let Some(last_block) = self.fetch_last_block_header(chain_head).await else {
            return Err(());
        };

        let search_key = NodeId::new(&last_block.hash);
        for block in self.fetch_block_chain(&search_key, MAX_TTL).await {
            let mut block_chain = self.block_chain.lock().await;

            if let Err(_) = block_chain.append_block(&block) {
                return Err(());
            }
        }

        Ok(())
    }

    pub(crate) fn get_last_key(node_id: NodeId) -> NodeId {
        let namespace = format!("chain_head:{}", hex::encode(node_id.0));
        let hasher = DoubleHasher::default();
        let key = hasher.hash(namespace);

        NodeId::new(&key)
    }

    pub async fn fetch_last_block_header(&self, tip: Block) -> Option<BlockHeader> {
        let mut candidate_blocks = vec![tip.header];

        let kademlia = self.kademlia_net.lock().await;
        let mut closest_nodes = match kademlia.node_lookup(&kademlia.core.id).await {
            Ok(nodes) => nodes,
            Err(_) => return None,
        };

        while let Some(search_node) = closest_nodes.pop() {
            let search_key = Self::get_last_key(search_node.id.clone());

            let block_header = match kademlia.find_value(&search_key).await {
                Ok(result) => result,
                _ => {
                    continue;
                }
            };

            let Some(block) = block_header else {
                continue;
            };

            let Some(block) = block.as_any().downcast_ref::<BlockHeader>() else {
                continue;
            };

            if !block.validate_signature(search_node.keys.public_key) {
                continue;
            }

            candidate_blocks.push(block.clone());
        }

        candidate_blocks.into_iter().max_by(|a, b| {
            a.difficulty
                .cmp(&b.difficulty)
                .then_with(|| a.index.cmp(&b.index))
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        })
    }

    pub async fn get_connection(&self) -> Result<Node, Box<dyn Error + '_>> {
        let kademlia = self.kademlia_net.lock().await;

        let public_key = kademlia.core.keys.public_key;
        let addr = kademlia.core.get_addr()?;

        Ok(Node::from_pub_key(
            &public_key,
            addr.ip().to_string(),
            addr.port() as usize,
        ))
    }
}
