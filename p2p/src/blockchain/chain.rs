use std::{sync::Arc, time};

use log::{error, info};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{blockchain::event::BlockChainEvent, kademlia::secret_key::SecretPair};

use super::{
    block_builder::BlockBuilder, event::BlockChainEventHandler, hash_func::DoubleHasher,
    transaction_pool::TransactionPool, Block, HashFunc, Transaction,
};

#[derive(Debug, Error)]
pub enum BlockChainError {
    #[error("Block already appended")]
    BlockAlreadyPersisted,

    #[error("Block is invalid")]
    InvalidBlock,

    #[error("Failed to fetch block")]
    BlockNotFound,

    #[error("Chain is broken")]
    ChainBroken,
}

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
        for (current, next) in self.blocks.iter().zip(self.blocks.iter().skip(1)) {
            if !current.validate(hasher.clone(), current.header.merkle_root) {
                return false;
            }

            if next.header.prev_hash != current.header.hash {
                return false;
            }
        }

        if let Some(last) = self.blocks.last() {
            if !last.validate(hasher.clone(), last.header.merkle_root) {
                return false;
            }
        }

        true
    }

    pub fn start_miner(
        pair: SecretPair,
        block_chain: Arc<Mutex<Self>>,
        event_handler: Arc<dyn BlockChainEventHandler>,
        batch_size: usize,
        batch_pulling: time::Duration,
    ) {
        let block_chain = Arc::clone(&block_chain);
        let event_handler = Arc::clone(&event_handler);

        info!("[⛏️] Miner async task started!");

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(batch_pulling).await;

                let block_chain = Arc::clone(&block_chain);
                let transactions = {
                    let Ok(mut block_chain) = block_chain.try_lock() else {
                        continue;
                    };

                    match block_chain
                        .transaction_poll
                        .fetch_batch_transactions(batch_size)
                    {
                        Ok(transactions) if !transactions.is_empty() => Some(transactions),
                        Ok(_) => {
                            info!("No transactions to be added");
                            None
                        }
                        Err(e) => {
                            error!("Failed to fetch transactions: {:?}", e);
                            None
                        }
                    }
                };

                let Some(transactions) = transactions else {
                    continue;
                };

                let block_chain_tx = Arc::clone(&block_chain);
                let block = {
                    let Ok(mut block_chain) = block_chain_tx.try_lock() else {
                        continue;
                    };

                    block_chain.add_block(|mut builder| {
                        builder
                            .add_transactions(transactions)
                            .sign_with(pair.clone());

                        builder
                    })
                };

                {
                    let event_handler = Arc::clone(&event_handler);
                    event_handler
                        .on_event(BlockChainEvent::AddBlock(block))
                        .await;
                }
            }
        });
    }

    fn get_block_by_hash(&self, hash: [u8; 32]) -> Option<&Block> {
        if let Some(position) = self
            .blocks
            .iter()
            .position(|block| block.header.hash == hash)
        {
            return self.blocks.get(position);
        }

        None
    }

    pub fn get_blockchain_head(&self) -> Option<&Block> {
        self.blocks.iter().max_by_key(|block| block.header.index)
    }

    pub fn search_blocks_on<PredicateFn>(
        &self,
        predicate: PredicateFn,
    ) -> impl Iterator<Item = &Block>
    where
        PredicateFn: Fn(&Block) -> bool,
    {
        let mut chain = Vec::new();
        let tip = self
            .get_blockchain_head()
            .expect("Failed to find the tip of the chain");

        let mut current = Some(tip);

        while let Some(block) = current {
            chain.push(block);
            current = self.get_block_by_hash(block.header.prev_hash);
        }

        chain.reverse();
        chain.into_iter().filter(move |block| predicate(*block))
    }

    pub fn search_transactions_on<PerdicateFn>(
        &self,
        perdicate: PerdicateFn,
    ) -> impl Iterator<Item = &Transaction>
    where
        PerdicateFn: Fn(&Transaction) -> bool,
    {
        self.blocks
            .iter()
            .flat_map(|block| block.transactions.iter())
            .filter(move |transaction| perdicate(*transaction))
    }

    pub fn remove_last(&mut self) -> Option<Block> {
        self.blocks.pop()
    }

    pub fn append_block(&mut self, block: &Block) -> Result<(), BlockChainError> {
        // TODO: verify the signature of the block

        if self
            .search_blocks_on(|b| b.header.hash == block.header.hash)
            .next()
            .is_some()
        {
            return Err(BlockChainError::BlockAlreadyPersisted);
        }

        let Some(prev_block) = self.get_blockchain_head() else {
            return Err(BlockChainError::BlockNotFound);
        };

        if !block.validate(DoubleHasher::default(), block.header.merkle_root) {
            return Err(BlockChainError::InvalidBlock);
        }

        if prev_block.header.hash != block.header.prev_hash {
            return Err(BlockChainError::ChainBroken);
        }

        self.blocks.push(block.clone());
        Ok(())
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
            prev_block.header.hash,
        ));

        info!("[⛏️] Mining block!");
        let block = block_builder.mine(DoubleHasher {});
        info!("[⛏️] Finish block!: {}", hex::encode(block.header.hash));

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
