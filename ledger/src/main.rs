use core::time;
use std::{sync::Arc, thread};

use ledger::{
    blockchain::{BlockChain, Transaction},
    logger,
};
use log::{error, info};
use rand::Rng;

fn main() {
    logger::init_logger("info", "logs", "ledger");

    let block_chain = BlockChain::<u32>::new();
    let miner_thread = block_chain.start_miner(5, time::Duration::from_secs(60 * 2));

    let block_chain_clone = Arc::new(block_chain);

    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(15));

        let data = rand::rng().random();
        match block_chain_clone.add_transaction(Transaction::new(
            "Diogo".to_string(),
            "OnlyCavas".to_string(),
            data,
        )) {
            Ok(_) => info!("[💰] Added Transaction: {}", data),
            Err(_) => error!("Error while submitting transaction"),
        }
    });

    miner_thread.join().expect("Error running the minor");
}
