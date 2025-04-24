use std::{
    sync::{Arc, Mutex},
    thread,
    time::{self, Duration},
};

use log::{error, info};
use p2p::{
    blockchain::{BlockChain, DoubleHasher, Transaction},
    logger,
    models::transactions::InitAuctionTransaction,
    store::{InFileStorage, NetworkNodeStorage},
};

const BATCH_PULLING_SIZE: usize = 15;
const BATCH_PULLING_TIME_FRAME: Duration = time::Duration::from_secs(2 * 60); // 2 mins

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger("info", "logs", "ledger");

    let storage = Arc::new(InFileStorage::new("./block_chain.bin"));

    let block_chain = {
        let block_chain = match storage.load::<BlockChain>() {
            Ok(chain) if chain.validate(DoubleHasher {}) => chain,
            _ => BlockChain::new(),
        };

        Arc::new(Mutex::new(block_chain))
    };

    let miner_thread = BlockChain::start_miner(
        Arc::clone(&block_chain),
        storage.clone(),
        BATCH_PULLING_SIZE,
        BATCH_PULLING_TIME_FRAME,
    );

    let tx_chain = Arc::clone(&block_chain);
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(15));
        let tx_chain = tx_chain.lock().unwrap();

        // let ran = rand::rng().random();
        match tx_chain.transaction_poll.add_transaction(Transaction::new(
            "Diogo".to_string(),
            "OnlyCavas".to_string(),
            InitAuctionTransaction {
                auction_id: "id".into(),
            }, // Data { name: ran },
        )) {
            // Ok(_) => info!("[💰] Added Transaction: {:?}", ran),
            Ok(_) => info!("[💰] Added Transaction: "),
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

    Ok(())

    // let bootstrap = DHTNode::<MyData>::bootstrap("127.0.0.1".into(), 5000)
    //     .await
    //     .expect("Error creating bootstrap node");

    // let node1 = DHTNode::<MyData>::new(bootstrap.clone(), "127.0.0.1".into(), 5006)
    //     .await
    //     .expect("Error creating node 1");

    // let node2 = DHTNode::<MyData>::new(bootstrap.clone(), "127.0.0.1".into(), 5007)
    //     .await
    //     .expect("Error creating node 2");

    // let Some(store_key) = NodeId::random() else {
    //     panic!("Failed to create key");
    // };

    // let value_store = MyData::new("Diogo Assunção".into());
    // if let Err(_) = node1.store(&store_key, value_store).await {
    //     panic!("Failed to propagate value thru network");
    // };

    // if let Ok(Some(value)) = node2.find_value(&store_key).await {
    //     println!("My data is something like: {}, ain't ya ;)", value.name);
    // }

    // println!();
    // println!("Node 1");
    // println!("{:?}", node1.distributed_hash_tb);
    // println!();
    // println!("Node 2");
    // println!("{:?}", node2.distributed_hash_tb);

    // Ok(())
}
