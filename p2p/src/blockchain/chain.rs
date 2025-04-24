use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time,
};

use bincode::{Decode, Encode};
use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::store::NetworkNodeStorage;

use super::{
    block_builder::BlockBuilder, hash_func::DoubleHasher, transaction_pool::TransactionPool, Block,
    HashFunc,
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
            info!("{:#?}", block);
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
        TStorage: NetworkNodeStorage + Send + Sync + 'static,
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

            if let Err(_) = storage.store(&block_chain.clone()) {
                error!("Failed to save block chain state!");
            }

            info!("State saved!");
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

impl Encode for BlockChain {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.dificulty.encode(encoder)?;
        // self.blocks.encode(encoder)?;
        Ok(())
    }
}

impl<Ctx> Decode<Ctx> for BlockChain {
    fn decode<D: bincode::de::Decoder<Context = Ctx>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let dificulty = u32::decode(decoder)?;
        // let blocks = Vec::<Block<TData>>::decode(decoder)?;

        Ok(Self {
            dificulty,
            // blocks,
            blocks: vec![],
            transaction_poll: TransactionPool::new(),
        })
    }
}
