use std::env;

use p2p::{
    cli::{self, Config},
    logger,
    models::{
        client_network_node::ClientNetworkNode,
        network_node::{NetworkMode, NetworkNode},
    },
    store::InFileStorage,
    vars,
};

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

    env::set_var(vars::STORAGE_PATH, &args.out);
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
