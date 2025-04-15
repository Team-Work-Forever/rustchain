use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
};

use crate::blockchain::{BlockChain, TransactionData};
use bincode::{config, Decode, Encode};
use thiserror::Error;

const BLOCK_CHAIN_BIN: &str = "block_chain.bin";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to create blockchain file")]
    FileCreate(#[from] io::Error),

    #[error("failed to encode blockchain")]
    Encode(#[from] bincode::error::EncodeError),

    #[error("failed to decode blockchain")]
    Decode(#[from] bincode::error::DecodeError),

    #[error("blockchain file not found")]
    NotFound,
}

pub fn store_block_chain<TData>(block_chain: &BlockChain<TData>) -> Result<(), StoreError>
where
    TData: TransactionData + Encode,
{
    let config = config::standard();
    let block_chain_file = File::create(BLOCK_CHAIN_BIN)?;
    let mut buff_writer = BufWriter::new(block_chain_file);

    let bin_data = bincode::encode_to_vec(&block_chain, config)?;
    buff_writer.write_all(&bin_data)?;

    Ok(())
}

pub fn load_block_chain<TData>() -> Result<BlockChain<TData>, StoreError>
where
    TData: TransactionData + Decode<()>,
{
    if !Path::new(BLOCK_CHAIN_BIN).exists() {
        return Err(StoreError::NotFound);
    }

    let block_chain_file = File::open(BLOCK_CHAIN_BIN)?;
    let mut buf_reader = BufReader::new(block_chain_file);
    let mut bin_data = vec![];

    buf_reader.read_to_end(&mut bin_data)?;
    let config = config::standard();

    match bincode::decode_from_slice::<BlockChain<TData>, _>(&bin_data, config) {
        Ok((block_chain, _)) => Ok(block_chain),
        Err(e) => Err(StoreError::Decode(e)),
    }
}
