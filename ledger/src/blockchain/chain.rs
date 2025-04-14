use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, JoinHandle},
    time,
};

use log::{error, info};
use serde::Serialize;

use super::{block::Block, block_builder::BlockBuilder, hash_func::DoubleHasher, Transaction};

#[derive(Debug, Clone, Serialize)]
pub struct BlockChain<TData>
where
    TData: Clone + Serialize,
{
    dificulty: u32,
    pub blocks: Vec<Block<TData>>,

    #[serde(skip_serializing)]
    pub(crate) transaction_poll: Arc<Mutex<Vec<Transaction<TData>>>>,
}

impl<TData> BlockChain<TData>
where
    TData: Clone + Serialize + Send + 'static,
{
    pub fn new() -> BlockChain<TData> {
        BlockChain {
            dificulty: 5,
            blocks: vec![Block::new_genesis()],
            transaction_poll: Arc::new(Mutex::new(vec![])),
        }
    }

    fn get_lock_pool(&self) -> Result<MutexGuard<Vec<Transaction<TData>>>, ()> {
        self.transaction_poll
            .lock()
            .map_err(|e| error!("Failed to lock transaction pool {}", e))
    }

    fn fetch_batch_transactions(&self, batch_size: usize) -> Result<Vec<Transaction<TData>>, ()> {
        let mut pool = self.get_lock_pool()?;

        if pool.is_empty() {
            return Ok(vec![]);
        }

        let end = batch_size.min(pool.len());
        Ok(pool.drain(0..end).collect::<Vec<_>>())
    }

    pub fn add_transaction(&self, transaction: Transaction<TData>) -> Result<(), ()> {
        let mut pool = self.get_lock_pool()?;
        pool.push(transaction);

        Ok(())
    }

    pub fn start_miner(&self, batch_size: usize, batch_pulling: time::Duration) -> JoinHandle<()> {
        let mut self_clone = self.clone();
        info!("[⛏️] Miner thread started!");

        thread::spawn(move || loop {
            thread::sleep(batch_pulling);

            match self_clone.fetch_batch_transactions(batch_size) {
                Ok(transactions) if !transactions.is_empty() => {
                    self_clone.add_block(|mut builder| {
                        builder.add_transactions(transactions);
                        builder
                    });
                }
                Ok(_) => {
                    info!("No transactions to be added");
                }
                Err(e) => {
                    error!("Failed to fetch transactions: {:?}", e);
                }
            }
        })
    }

    fn add_block<F>(&mut self, block_builder_fn: F) -> Block<TData>
    where
        TData: Serialize,
        F: FnOnce(BlockBuilder<TData>) -> BlockBuilder<TData>,
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
