use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time,
};

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::blockchain::event::BlockChainEvent;

use super::{
    block_builder::BlockBuilder, event::BlockChainEventHandler, hash_func::DoubleHasher,
    transaction_pool::TransactionPool, Block, HashFunc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockChain {
    dificulty: u32,
    pub(crate) blocks: Vec<Block>,

    #[serde(skip)]
    pub transaction_poll: TransactionPool,
}

impl BlockChain {
    pub fn new() -> BlockChain {
        BlockChain {
            dificulty: 5,
            blocks: vec![Block::new_genesis()],
            transaction_poll: TransactionPool::new(),
        }
    }

    pub fn validate<THasher>(&self, hasher: THasher) -> bool
    where
        THasher: HashFunc,
    {
        for block in self.blocks.iter() {
            if !block.validate(hasher.clone(), block.merkle_root) {
                return false;
            }
        }

        true
    }

    pub fn start_miner(
        block_chain: Arc<Mutex<Self>>,
        event_handler: Arc<dyn BlockChainEventHandler>,
        batch_size: usize,
        batch_pulling: time::Duration,
    ) -> JoinHandle<()> {
        let block_chain = Arc::clone(&block_chain);
        info!("[⛏️] Miner thread started!");

        thread::spawn(move || loop {
            thread::sleep(batch_pulling);
            let mut block_chain = block_chain.lock().expect("Error locking block chain");

            match block_chain
                .transaction_poll
                .fetch_batch_transactions(batch_size)
            {
                Ok(transactions) if !transactions.is_empty() => {
                    let block = block_chain.add_block(|mut builder| {
                        builder.add_transactions(transactions);
                        builder
                    });

                    event_handler.on_event(BlockChainEvent::AddBlock(block));
                }
                Ok(_) => info!("No transactions to be added"),
                Err(e) => error!("Failed to fetch transactions: {:?}", e),
            }
        })
    }

    pub(crate) fn add_block<F>(&mut self, block_builder_fn: F) -> Block
    where
        F: FnOnce(BlockBuilder) -> BlockBuilder,
    {
        let prev_block = self
            .blocks
            .get(self.blocks.len() - 1)
            .expect("Wasn't possible to fetch the prev block");

        let block_builder = block_builder_fn(BlockBuilder::new(
            self.next_index(),
            self.dificulty,
            prev_block.hash,
        ));

        info!("[⛏️] Mining block!");
        let block = block_builder.mine(DoubleHasher {});
        info!("[⛏️] Finish block!: {}", hex::encode(block.hash));

        self.blocks.push(block.clone());

        block
    }

    fn next_index(&self) -> u64 {
        self.blocks
            .len()
            .try_into()
            .expect("Block count exceeds u64::MAX")
    }
}
