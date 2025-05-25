use std::{
    collections::HashSet,
    error::Error,
    sync::Arc,
    time::{self, Duration},
};

use log::info;
use rand::{
    rng,
    seq::{IndexedRandom, SliceRandom},
};
use tokio::sync::Mutex;

use crate::{
    blockchain::{Block, BlockChain, BlockChainError, BlockChainEventHandler, BlockHeader},
    kademlia::{event::DHTEventHandler, node::Contract, NodeId},
    DHTNode, Node,
};

pub const BATCH_PULLING_SIZE: usize = 15;
pub const MAX_TTL: u32 = 1024;
pub const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(10);

pub struct NetworkMode {
    pub bootstraps: Vec<Contract>,
    pub host: String,
    pub port: usize,
}

#[derive(Debug)]
pub struct NetworkNode {
    pub block_chain: Arc<Mutex<BlockChain>>,
    pub kademlia_net: Arc<Mutex<DHTNode>>,
    // ui events ->
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

        network_node.join_to_network(mode.bootstraps.clone()).await;

        let network_node_tx = Arc::clone(&network_node);
        Self::check_peers_health(network_node_tx);

        Some(network_node)
    }

    async fn join_to_network(&self, bootstraps: Vec<Contract>) {
        println!("Trying to establish connection...");

        loop {
            let Some(bootstrap) = bootstraps.choose(&mut rng()).cloned() else {
                return;
            };

            let mut kademlia = self.kademlia_net.lock().await;

            if kademlia.join_network(&bootstrap).await.is_some() {
                break;
            }

            println!("Failed to connect. Retrying in 10 seconds...");
            tokio::time::sleep(Duration::from_secs(10)).await;
        }

        println!("Syncing with network...");

        if let Err(_) = self.sync().await {
            return;
        }

        println!("Syncing process completed!");
    }

    pub async fn new(mode: NetworkMode) -> Option<Arc<Self>> {
        let Some(dht) = DHTNode::new(mode.host.clone(), mode.port).await else {
            return None;
        };

        Self::load_from(mode, BlockChain::new(), dht).await
    }

    pub(crate) async fn connect(self: Arc<Self>) {
        let handler: Arc<dyn DHTEventHandler> = Arc::clone(&self) as Arc<dyn DHTEventHandler>;

        let node_key_pair = {
            let kademlia = self.kademlia_net.lock().await;
            kademlia.init_grpc_connection(Arc::clone(&handler));

            kademlia.core.keys.clone()
        };

        let handler: Arc<dyn BlockChainEventHandler> =
            Arc::clone(&self) as Arc<dyn BlockChainEventHandler>;

        BlockChain::start_miner(
            node_key_pair,
            self.block_chain.clone(),
            handler,
            BATCH_PULLING_SIZE,
            BATCH_PULLING_TIME_FRAME,
        );
    }

    pub(crate) fn check_peers_health(network_node: Arc<NetworkNode>) {
        let averiguate_len = 5;
        let duration = Duration::from_secs(60);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(duration).await;

                let host_node = {
                    let kademlia = network_node.kademlia_net.lock().await;
                    kademlia.core.clone()
                };

                let mut closest_nodes = {
                    let kademlia = network_node.kademlia_net.lock().await;

                    match kademlia.node_lookup(&host_node.id).await {
                        Ok(nodes) => nodes,
                        _ => continue,
                    }
                };

                closest_nodes.truncate(averiguate_len);
                closest_nodes.shuffle(&mut rng());

                let kademlia = network_node.kademlia_net.lock().await;
                for try_node in closest_nodes {
                    match DHTNode::ping(&host_node, &try_node).await {
                        Ok(_) => {
                            continue;
                        }
                        _ => {
                            let mut routing_table = kademlia.routing_table.lock().await;
                            routing_table.remove(&try_node);
                        }
                    }
                }
            }
        });
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

        let Some(goal_block) = block_chain.get_blockchain_head() else {
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
            block_chain.get_blockchain_head().cloned()
        };

        let Some(chain_head) = fetch_chain_head else {
            return Err(());
        };

        info!("Fething block... {:#?}", chain_head);
        let Some(last_block) = self.fetch_last_block_header(chain_head).await else {
            return Err(());
        };

        info!("Fething block... {:#?}", last_block);
        let search_key = NodeId::new(&last_block.hash);
        for block in self.fetch_block_chain(&search_key, MAX_TTL).await {
            let mut block_chain = self.block_chain.lock().await;

            match block_chain.append_block(&block) {
                Ok(_) | Err(BlockChainError::BlockAlreadyPersisted) => continue,
                Err(_) => panic!("Failed to sync node"),
            }
        }

        // It anounces that this block header it's the chain's global head
        self.update_global_bc_head(&last_block).await;

        println!("blockchain: {:#?}", self.block_chain);
        Ok(())
    }

    pub async fn fetch_last_block_header(&self, tip: Block) -> Option<BlockHeader> {
        let mut candidate_blocks = vec![tip.header];

        let kademlia = self.kademlia_net.lock().await;
        let mut closest_nodes = match kademlia.node_lookup(&kademlia.core.id).await {
            Ok(nodes) => nodes,
            Err(_) => return None,
        };

        while let Some(search_node) = closest_nodes.pop() {
            let search_key = NodeId::create_chain_head(search_node.id.clone());
            info!("Search Key: {:#?}", search_key);

            let block_header = match kademlia.find_value(&search_key).await {
                Ok(result) => result,
                _ => {
                    continue;
                }
            };

            info!("Checking header:::... {:#?}", block_header);
            let Some(block) = block_header else {
                continue;
            };

            info!("Checking ref:::... {:#?}", block);
            let Some(block) = block.as_any().downcast_ref::<BlockHeader>() else {
                continue;
            };

            info!("Gain block: {:#?}", block);
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
