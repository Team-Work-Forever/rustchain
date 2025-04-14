use core::time;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use ledger::{
    blockchain::{BlockChain, Transaction},
    logger,
};
use log::{error, info};
use rand::Rng;

// Transaction Pool
const BATCH_PULLING_SIZE: usize = 5;
const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(2 * 60); // 2 mins

fn main() {
    logger::init_logger("info", "logs", "ledger");

    let block_chain: Arc<Mutex<BlockChain<u32>>> = Arc::new(Mutex::new(BlockChain::<u32>::new()));

    let miner_thread = BlockChain::start_miner(
        Arc::clone(&block_chain),
        BATCH_PULLING_SIZE,
        BATCH_PULLING_TIME_FRAME,
    );

    // client node
    let tx_chain = Arc::clone(&block_chain);
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(15));
        let mut tx_chain = tx_chain.lock().unwrap();

        let data = rand::rng().random();
        match tx_chain.add_transaction(Transaction::new(
            "Diogo".to_string(),
            "OnlyCavas".to_string(),
            data,
        )) {
            Ok(_) => info!("[💰] Added Transaction: {}", data),
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
