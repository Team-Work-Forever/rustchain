use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
};

use bincode::config;
use log::error;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tonic::async_trait;

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

#[async_trait]
pub trait NetworkNodeStorage: Debug {
    fn load<TData>(&self) -> Result<TData, StoreError>
    where
        TData: Serialize + for<'de> Deserialize<'de>;

    fn store<TData>(&self, data: &TData) -> Result<(), StoreError>
    where
        TData: Serialize + for<'de> Deserialize<'de> + Debug;
}

#[derive(Debug, Clone)]
pub struct InFileStorage<'a> {
    storage_location: &'a str,
}

impl<'a> InFileStorage<'a> {
    pub fn new(path: &'a str) -> Self {
        Self {
            storage_location: path,
        }
    }
}

impl<'a> NetworkNodeStorage for InFileStorage<'a> {
    fn load<TData>(&self) -> Result<TData, StoreError>
    where
        TData: Serialize + for<'de> Deserialize<'de>,
    {
        if !Path::new(self.storage_location).exists() {
            return Err(StoreError::NotFound);
        }

        let bin_file_data = File::open(self.storage_location)?;
        let mut buf_reader = BufReader::new(bin_file_data);
        let mut bin_data = vec![];

        buf_reader.read_to_end(&mut bin_data)?;
        let config = config::standard();

        match bincode::serde::decode_from_slice::<TData, _>(&bin_data, config) {
            Ok((block_chain, _)) => Ok(block_chain),
            Err(e) => Err(StoreError::Decode(e)),
        }
    }

    fn store<TData>(&self, data: &TData) -> Result<(), StoreError>
    where
        TData: Serialize + for<'de> Deserialize<'de> + Debug,
    {
        let config = config::standard();
        let bin_file_data = File::create(self.storage_location)?;
        let mut buff_writer = BufWriter::new(bin_file_data);

        let bin_data = bincode::serde::encode_to_vec(&data, config)?;
        buff_writer.write_all(&bin_data)?;
        buff_writer.flush()?;

        Ok(())
    }
}
