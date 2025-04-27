use std::{sync::Arc, thread, time};

use log::{error, info};
use p2p::{
    blockchain::Transaction,
    logger,
    models::{
        data::KData,
        network_node::{NetworkMode, NetworkNode},
        transactions::InitAuctionTransaction,
    },
    store::InFileStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger("info", "logs", "ledger");
    let storage = InFileStorage::new("start_wars.bin");

    let Some(bootstrap) = NetworkNode::load_node(
        NetworkMode::Bootstrap {
            host: "127.0.0.1".into(),
            port: 3000,
        },
        storage.clone(),
    )
    .await
    else {
        panic!("Error creating bootstrap");
    };

    println!("Block chain: {:#?}", bootstrap.block_chain);
    println!("DHT: {:#?}", bootstrap.kademlia_net);

    let Ok(conn) = bootstrap.get_connection() else {
        panic!("Error fetching pub key");
    };

    let bootstrap_list = vec![conn];

    let Some(node1) = NetworkNode::new(NetworkMode::Join {
        bootstraps: bootstrap_list.clone(),
        host: "127.0.0.1".into(),
        port: 4000,
    })
    .await
    else {
        panic!("Error creating node1");
    };

    let Some(_node2) = NetworkNode::new(NetworkMode::Join {
        bootstraps: bootstrap_list,
        host: "127.0.0.1".into(),
        port: 4001,
    })
    .await
    else {
        panic!("Error creating node2");
    };

    let Some(store_id) = p2p::kademlia::NodeId::random() else {
        panic!("Ardeu!")
    };

    let _ = node1
        .kademlia_net
        .store(&store_id, Box::new(KData::new("Diogo Assunção".into())))
        .await;

    let tx_chain = Arc::clone(&bootstrap.block_chain);
    thread::spawn(move || loop {
        thread::sleep(time::Duration::from_secs(15));
        let tx_chain = tx_chain.lock().unwrap();

        match tx_chain.transaction_poll.add_transaction(Transaction::new(
            "Diogo".to_string(),
            "OnlyCavas".to_string(),
            InitAuctionTransaction {
                auction_id: "cebolas".into(),
            },
        )) {
            Ok(_) => info!("[💰] Added Transaction"),
            Err(_) => error!("Error while submitting transaction"),
        }
    });

    println!("Node running. Press Ctrl+C to stop.");

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");

    if let Err(_) = bootstrap.persist_node(storage).await {
        panic!("Failed to persist node")
    }

    println!("Shutting down gracefully.");

    Ok(())
}
