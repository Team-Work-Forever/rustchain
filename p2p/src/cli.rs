use std::{fs, net::SocketAddr, path::PathBuf};

use clap::{Parser, ValueEnum};
use serde::Deserialize;
use thiserror::Error;

use crate::kademlia::node::Contract;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Failed to load from the configuration file")]
    FailedToLoadConfig,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "p2p",
    version,
    about = "create and connect to the peer to peer network"
)]
pub struct Arguments {
    #[arg(long)]
    pub node_type: Option<NodeType>,

    #[arg(long)]
    pub host: Option<String>,

    #[arg(long)]
    pub port: Option<usize>,

    #[arg(long, value_delimiter = ',')]
    pub bootstrap: Option<Vec<String>>,

    #[arg(long)]
    pub out: Option<PathBuf>,

    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Copy, Clone, Debug, Deserialize, ValueEnum)]
pub enum NodeType {
    Join,
    Client,
}

impl ToString for NodeType {
    fn to_string(&self) -> String {
        match self {
            NodeType::Join => "join".to_string(),
            NodeType::Client => "client".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub node_type: NodeType,
    pub host: String,
    pub port: usize,
    bootstrap: Vec<String>,
    pub out: PathBuf,
}

impl Config {
    pub fn get_bootstrap_nodes(&self) -> Vec<Contract> {
        self.bootstrap
            .iter()
            .filter_map(|addr| {
                let socket = match addr.parse::<SocketAddr>() {
                    Ok(socket) => socket,
                    Err(_) => panic!("Bootstrap address, must have <host>:<port>"),
                };

                Some(Contract {
                    host: socket.ip().to_string(),
                    port: socket.port() as usize,
                })
            })
            .collect()
    }
}

impl Arguments {
    pub fn from_with_config() -> Result<Config, CliError> {
        let args = Arguments::parse();

        let file_config = if let Some(path) = args.config.as_ref() {
            let content = fs::read_to_string(path).map_err(|_| CliError::FailedToLoadConfig)?;
            let config =
                toml::from_str::<Config>(&content).map_err(|_| CliError::FailedToLoadConfig)?;
            config
        } else {
            Config {
                node_type: NodeType::Join,
                host: "127.0.0.1".into(),
                port: 6657,
                bootstrap: vec![],
                out: "out.bin".into(),
            }
        };

        Ok(Config {
            node_type: args.node_type.unwrap_or(file_config.node_type),
            host: args.host.unwrap_or(file_config.host),
            port: args.port.unwrap_or(file_config.port),
            bootstrap: args.bootstrap.unwrap_or(file_config.bootstrap),
            out: args.out.unwrap_or(file_config.out),
        })
    }
}
