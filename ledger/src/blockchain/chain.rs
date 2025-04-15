use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread::{self, JoinHandle},
    time,
};

use bincode::{Decode, Encode};
use log::{error, info};
use serde::Serialize;

use crate::store;

use super::{
    block::Block, block_builder::BlockBuilder, hash_func::DoubleHasher, Transaction,
    TransactionData,
};

#[derive(Clone, Debug)]
pub struct BlockChain<TData>
where
    TData: TransactionData,
{
    dificulty: u32,
    pub(crate) blocks: Vec<Block<TData>>,

    pub(crate) transaction_poll: Arc<Mutex<Vec<Transaction<TData>>>>,
}

impl<TData> BlockChain<TData>
where
    TData: TransactionData,
{
    pub fn new() -> BlockChain<TData> {
        BlockChain {
            dificulty: 5,
            blocks: vec![Block::new_genesis()],
            transaction_poll: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn from_bin() -> BlockChain<TData>
    where
        TData: Decode<()>,
    {
        match store::load_block_chain() {
            Ok(block_chain) => block_chain,
            Err(_) => BlockChain::<TData>::new(),
        }
    }

    pub fn add_transaction(&mut self, transaction: Transaction<TData>) -> Result<(), ()> {
        let mut pool = self.get_lock_pool()?;
        pool.push(transaction);

        Ok(())
    }

    fn get_lock_pool(&self) -> Result<MutexGuard<Vec<Transaction<TData>>>, ()> {
        self.transaction_poll
            .lock()
            .map_err(|e| error!("Failed to lock transaction pool {}", e))
    }

    pub fn start_miner(
        block_chain: Arc<Mutex<BlockChain<TData>>>,
        batch_size: usize,
        batch_pulling: time::Duration,
    ) -> JoinHandle<()>
    where
        TData: Clone + Serialize + Send + 'static,
    {
        let block_chain = Arc::clone(&block_chain);
        info!("[⛏️] Miner thread started!");

        thread::spawn(move || loop {
            thread::sleep(batch_pulling);
            let mut block_chain = block_chain.lock().expect("Error locking block chain");

            match block_chain.fetch_batch_transactions(batch_size) {
                Ok(transactions) if !transactions.is_empty() => {
                    block_chain.add_block(|mut builder| {
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

    fn fetch_batch_transactions(
        &mut self,
        batch_size: usize,
    ) -> Result<Vec<Transaction<TData>>, ()> {
        let mut pool = self.get_lock_pool()?;

        if pool.is_empty() {
            return Ok(vec![]);
        }

        let end = batch_size.min(pool.len());
        Ok(pool.drain(0..end).collect::<Vec<_>>())
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

        if let Err(_) = store::store_block_chain(&self) {
            error!("Failed to save block chain state!");
            return block;
        }

        info!("State saved!");
        block
    }

    fn next_index(&self) -> u64 {
        self.blocks
            .len()
            .try_into()
            .expect("Block count exceeds u64::MAX")
    }
}

impl<TData: Encode> Encode for BlockChain<TData>
where
    TData: TransactionData,
{
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.dificulty.encode(encoder)?;
        self.blocks.encode(encoder)?;
        Ok(())
    }
}

impl<Ctx, TData> Decode<Ctx> for BlockChain<TData>
where
    TData: TransactionData + Decode<Ctx>,
{
    fn decode<D: bincode::de::Decoder<Context = Ctx>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let dificulty = u32::decode(decoder)?;
        let blocks = Vec::<Block<TData>>::decode(decoder)?;

        Ok(Self {
            dificulty: dificulty,
            blocks: blocks,
            transaction_poll: Arc::new(Mutex::new(vec![])),
        })
    }
}
