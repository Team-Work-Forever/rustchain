use core::time;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use bincode::{Decode, Encode};
use ledger::{
    blockchain::{BlockChain, DoubleHasher, Transaction, TransactionData},
    logger,
    store::{BlockChainStorage, InFileStorage},
};

use log::{error, info};
use rand::Rng;
use serde::Serialize;

// Transaction Poll
const BATCH_PULLING_SIZE: usize = 15;
const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(2 * 60); // 2 mins

#[derive(Clone, Serialize, Debug, Encode, Decode)]
struct Data {
    value: u32,
}

impl TransactionData for Data {}

fn main() {
    logger::init_logger("info", "logs", "ledger");

    // local node
    // load properly block chain
    let storage = Arc::new(InFileStorage::new());
    let block_chain = {
        let block_chain = match storage.load() {
            Ok(chain) if chain.validate(DoubleHasher {}) => chain,
            _ => BlockChain::<Data>::new(),
        };

        Arc::new(Mutex::new(block_chain))
    };

    let miner_thread = BlockChain::start_miner(
        Arc::clone(&block_chain),
        storage,
        BATCH_PULLING_SIZE,
        BATCH_PULLING_TIME_FRAME,
    );

    // client node
    let tx_chain = Arc::clone(&block_chain);
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(15));
        let tx_chain = tx_chain.lock().unwrap();

        let ran = rand::rng().random();
        match tx_chain.transaction_poll.add_transaction(Transaction::new(
            "Diogo".to_string(),
            "OnlyCavas".to_string(),
            Data { value: ran },
        )) {
            Ok(_) => info!("[💰] Added Transaction: {:?}", ran),
            Err(_) => error!("Error while submitting transaction"),
        }
    });

    // interface
    let print_chain = Arc::clone(&block_chain);

    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(20));

        print!("\x1B[2J\x1B[1;1H");

        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let chain = print_chain.lock().unwrap();
        println!("{:#?}", *chain);
    });

    miner_thread.join().expect("Error running the minor");
}
