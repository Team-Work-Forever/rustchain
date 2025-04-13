use serde::Serialize;

use super::{block::Block, block_builder::BlockBuilder, hash_func::DoubleHasher};

#[derive(Debug, Clone, Serialize)]
pub struct BlockChain<TData: Clone + Serialize> {
    dificulty: u32,
    pub blocks: Vec<Block<TData>>,
}

impl<TData: Clone + Serialize> BlockChain<TData> {
    pub fn new() -> BlockChain<TData> {
        BlockChain {
            dificulty: 5,
            blocks: vec![Block::new_genesis()],
        }
    }

    fn next_index(&self) -> u64 {
        self.len().try_into().expect("Block count exceeds u64::MAX")
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn add_block<F>(&mut self, block_builder_fn: F) -> Block<TData>
    where
        TData: Serialize,
        F: FnOnce(BlockBuilder<TData>) -> BlockBuilder<TData>,
    {
        let prev_block = self
            .blocks
            .get(self.len() - 1)
            .expect("Wasn't possible to fetch the prev block");

        let block_builder = block_builder_fn(BlockBuilder::new(
            self.next_index(),
            self.dificulty,
            prev_block.hash,
        ));

        let block = block_builder.mine(DoubleHasher {});
        self.blocks.push(block.clone());
        block
    }
}
