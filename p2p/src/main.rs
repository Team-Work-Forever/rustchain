use std::{sync::Arc, time};

use log::{error, info};
use p2p::{
    blockchain::Transaction,
    logger,
    models::{
        network_node::{NetworkMode, NetworkNode},
        transactions::InitAuctionTransaction,
    },
    store::InFileStorage,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger("info", "logs", "ledger");
    let storage = InFileStorage::new("star_wars.bin");
    let std = InFileStorage::new("impire_strikes_back.bin");

    let Some(bootstrap) = NetworkNode::load_node(
        NetworkMode::Bootstrap {
            host: "127.0.0.1".into(),
            port: 3000,
        },
        std.clone(),
    )
    .await
    else {
        panic!("Error creating bootstrap");
    };

    let Ok(conn) = bootstrap.get_connection().await else {
        panic!("Error fetching pub key");
    };

    let bootstrap_list = vec![conn];

    let Some(node1) = NetworkNode::load_node(
        NetworkMode::Join {
            bootstraps: bootstrap_list.clone(),
            host: "127.0.0.1".into(),
            port: 4000,
        },
        storage.clone(),
    )
    .await
    else {
        panic!("Error creating node1");
    };

    let Some(node2) = NetworkNode::new(NetworkMode::Join {
        bootstraps: bootstrap_list,
        host: "127.0.0.1".into(),
        port: 4001,
    })
    .await
    else {
        panic!("Error creating node2");
    };

    // if let Err(_) = node2.sync().await {
    //     panic!("Why");
    // }

    // return Ok(());

    let node_tx = Arc::clone(&node1.block_chain);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(time::Duration::from_secs(15)).await;
            let node_tx = node_tx.lock().await;

            match node_tx.transaction_poll.add_transaction(Transaction::new(
                "Diogo".to_string(),
                "OnlyCavas".to_string(),
                InitAuctionTransaction {
                    auction_id: "cebolas".into(),
                },
            )) {
                Ok(_) => info!("[💰] Added Transaction"),
                Err(_) => error!("Error while submitting transaction"),
            }
        }
    });

    let node2_tx = Arc::clone(&node2.block_chain);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(time::Duration::from_secs(20)).await;
            let node2_tx = node2_tx.lock().await;

            match node2_tx.transaction_poll.add_transaction(Transaction::new(
                "Diogo".to_string(),
                "OnlyCavas".to_string(),
                InitAuctionTransaction {
                    auction_id: "cebolas".into(),
                },
            )) {
                Ok(_) => info!("[💰] Added Transaction"),
                Err(_) => error!("Error while submitting transaction"),
            }
        }
    });

    println!("Node running. Press Ctrl+C to stop.");

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+c");

    if let Err(_) = node1.persist_node(storage).await {
        panic!("Failed to persist node")
    }

    if let Err(_) = bootstrap.persist_node(std).await {
        panic!("Failed to persist node")
    }

    println!("Shutting down gracefully.");

    Ok(())
}
