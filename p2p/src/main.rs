use p2p::{
    cli::{self, Config},
    kademlia::node::Contract,
    logger,
    models::{
        client_network_node::ClientNetworkNode,
        network_node::{NetworkMode, NetworkNode},
    },
    store::InFileStorage,
    DHTNode,
};

pub async fn test() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger("info", "logs", "ledger");
    let storage = InFileStorage::new("star_wars.bin");
    let std = InFileStorage::new("impire_strikes_back.bin");

    let Some(bootstrap) = NetworkNode::load_node(
        NetworkMode {
            host: "127.0.0.1".into(),
            port: 3000,
            bootstraps: vec![],
        },
        std.clone(),
    )
    .await
    else {
        panic!("Error creating bootstrap");
    };

    let bootstrap_list = vec![Contract {
        host: "127.0.0.1".into(),
        port: 3000,
    }];

    let Some(node1) = NetworkNode::load_node(
        NetworkMode {
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

    let Some(node2) = NetworkNode::new(NetworkMode {
        bootstraps: bootstrap_list,
        host: "127.0.0.1".into(),
        port: 4001,
    })
    .await
    else {
        panic!("Error creating node2");
    };

    {
        match DHTNode::ping(
            &node1.kademlia_net.lock().await.core,
            &node2.kademlia_net.lock().await.core,
        )
        .await
        {
            Ok(_) => println!("Yeah"),
            Err(e) => panic!("Error: {}", e),
        }
    }

    // let node_tx = Arc::clone(&node1.block_chain);
    // tokio::spawn(async move {
    //     loop {
    //         tokio::time::sleep(time::Duration::from_secs(15)).await;
    //         let node_tx = node_tx.lock().await;

    //         match node_tx.transaction_poll.add_transaction(Transaction::new(
    //             "Diogo".to_string(),
    //             "OnlyCavas".to_string(),
    //             InitAuctionTransaction {
    //                 auction_id: "cebolas".into(),
    //             },
    //         )) {
    //             Ok(_) => info!("[💰] Added Transaction"),
    //             Err(_) => error!("Error while submitting transaction"),
    //         }
    //     }
    // });

    // let node2_tx = Arc::clone(&node2.block_chain);
    // tokio::spawn(async move {
    //     loop {
    //         tokio::time::sleep(time::Duration::from_secs(20)).await;
    //         let node2_tx = node2_tx.lock().await;

    //         match node2_tx.transaction_poll.add_transaction(Transaction::new(
    //             "Diogo".to_string(),
    //             "OnlyCavas".to_string(),
    //             InitAuctionTransaction {
    //                 auction_id: "cebolas".into(),
    //             },
    //         )) {
    //             Ok(_) => info!("[💰] Added Transaction"),
    //             Err(_) => error!("Error while submitting transaction"),
    //         }
    //     }
    // });

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

pub fn print_node_info(config: Config) {
    println!();
    println!("Type: {}", config.node_type.to_string());
    println!("Host: {} -> listening on {}", config.host, config.port);

    if let Some(path) = config.out.to_str() {
        println!("Persisting at {}", path);
    }

    println!();
    println!("Loaded Bootstrap nodes:");
    for bootstrap in config.get_bootstrap_nodes() {
        println!("\t Host: {} -> {}", bootstrap.host, bootstrap.port);
    }
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init_logger("info", "logs", "ledger");

    let args = match cli::Arguments::from_with_config() {
        Ok(args) => args,
        Err(e) => panic!("{}", e),
    };

    let storage = InFileStorage::new(&args.out);
    print_node_info(args.clone());

    let Some(node) = NetworkNode::load_node(
        NetworkMode {
            bootstraps: args.get_bootstrap_nodes(),
            host: args.host,
            port: args.port,
        },
        storage.clone(),
    )
    .await
    else {
        panic!("Error creating node");
    };

    match args.node_type {
        cli::NodeType::Client => {
            let client = ClientNetworkNode::new(node.clone());
            client.init_ui().await?;
        }
        cli::NodeType::Join => {
            println!("Node running. Press Ctrl+C to stop.");

            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for ctrl+c");
        }
    }

    if let Err(_) = node.persist_node(storage).await {
        panic!("Failed to persist node")
    }

    Ok(())
}
