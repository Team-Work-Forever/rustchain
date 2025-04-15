use log::error;
use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time,
};

use bincode::{Decode, Encode};
use log::info;
use serde::Serialize;

use crate::store::BlockChainStorage;

use super::{
    block_builder::BlockBuilder, hash_func::DoubleHasher, transaction_pool::TransactionPool, Block,
    HashFunc, TransactionData,
};

#[derive(Clone, Debug)]
pub struct BlockChain<TData>
where
    TData: TransactionData,
{
    dificulty: u32,
    pub(crate) blocks: Vec<Block<TData>>,

    pub transaction_poll: TransactionPool<TData>,
}

impl<TData> BlockChain<TData>
where
    TData: TransactionData,
{
    pub fn new() -> BlockChain<TData> {
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

    pub fn start_miner<TStorage>(
        block_chain: Arc<Mutex<Self>>,
        storage: Arc<TStorage>,
        batch_size: usize,
        batch_pulling: time::Duration,
    ) -> JoinHandle<()>
    where
        TData: Clone + Serialize + Send + 'static,
        TStorage: BlockChainStorage<TData> + Send + 'static + Sync,
    {
        let block_chain = Arc::clone(&block_chain);
        let storage = Arc::clone(&storage);
        info!("[⛏️] Miner thread started!");

        thread::spawn(move || loop {
            thread::sleep(batch_pulling);
            let mut block_chain = block_chain.lock().expect("Error locking block chain");

            match block_chain
                .transaction_poll
                .fetch_batch_transactions(batch_size)
            {
                Ok(transactions) if !transactions.is_empty() => {
                    block_chain.add_block(|mut builder| {
                        builder.add_transactions(transactions);
                        builder
                    });
                }
                Ok(_) => info!("No transactions to be added"),
                Err(e) => error!("Failed to fetch transactions: {:?}", e),
            }

            if let Err(_) = storage.store(&block_chain) {
                error!("Failed to save block chain state!");
            }

            info!("State saved!");
        })
    }

    pub(crate) fn add_block<F>(&mut self, block_builder_fn: F) -> Block<TData>
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
            dificulty,
            blocks,
            transaction_poll: TransactionPool::new(),
        })
    }
}
