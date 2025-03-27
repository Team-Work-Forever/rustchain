use std::time::SystemTime;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use log::info;
pub type Result<T> = std::result::Result<T, failure::Error>;

const TARGET_HEXT: usize = 4;

//block structure
#[derive(Degub, Clone)]
pub struct Block {
    timestamp: u128,
    transactions: String,
    previous_hash: String,
    hash: String,
    height: usize,
    nonce: i32,
}


#[derive(Degub)]
pub struct Blockchain {
    block: Vec<Block>
}


//block creation
impl Block {

    pub fn get_hash(&self) -> String {
        self.hash.clone()
    }

    //new genesis block
    pub fn new_genesis_block() -> Block {
        Block::new_block(String::from("Gensis Block"), String::new(), 0).unwrap()
    }

    pub fn new_block(data: String, previous_hash: String, height: usize) -> Result<Block> {
        let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_millis();
        let mut block = Block {
            timestamp: timestamp,
            transactions: data,
            previous_hash,
            hash: String::new(),
            height,
            nonce: 0,
        };
        block.run_proof_if_work()?;
        Ok(block)
    }

    fn run_proof_if_work(&mut self) -> Result<()> {
        info!("Mining the block");
        // before complete PoW its necessary do validate the hash
        while !self.validate()?{
            self.nonce += 1;
        }
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        self.hash = hasher.result_str();
        Ok(())
    }
    
    fn prepare_hash_data(&self) -> Result<Vec<u8>> {
        let content = (
            self.prev_block_hash.clone(), //clone data from the block
            self.hash_transactions()?,
            self.timestamp,
            TARGET_HEXS,
            self.nonce,
        );
        // serialize the content as a byte and return it
        let bytes = serialize(&content)?;
        Ok(bytes)
    }

    fn validate(&self) -> Result<bool> {
        let data = self.prepare_hash_data()?;
        let mut hasher = Sha256::new();
        hasher.input(&data[..]);
        let mut vec1: Vec<u8> = vec![];
        vec1.resize(TARGET_HEXT, '0' as u8);
        Ok(&hasher.result_str()[0..TARGET_HEXT] == String::from_utf8(vec1)?)
    }
}

//create empty blockchain with a genesis block
impl Blockchain{
    pub fn new() -> Blockchain {
        Blockchain {
            block: vec![Block::new_genesis_block()]
        }
    }
    pub fn add_block(&mut self, data: String) -> Result<()> {
        let prev = self.blocks.last().unwrap();
        let new_block = Block::new_block(data, prev.get_hash(), TARGET_HEXT)?;
        self.blocks.push(new_block);
        Ok(())
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain() {
        let mut blockchain = Blockchain::new();
        b.add_block("data".to_string());
        b.add_block("data1".to_string());
        b.add_block("data2".to_string());
        dbg!(b);
    }
}
