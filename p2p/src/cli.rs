use std::{fs, path::PathBuf};

use clap::{Parser, ValueEnum};
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
pub struct Config {
    pub node_type: NodeType,
    pub host: String,
    pub port: usize,
    pub bootstrap: Vec<String>,
    pub out: PathBuf,
}

impl Arguments {
    pub fn from_with_config() -> Config {
        let args = Arguments::parse();

        let file_config = args
            .config
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|s| toml::from_str::<Config>(&s).ok())
            .unwrap_or_else(|| Config {
                node_type: NodeType::Join,
                host: "127.0.0.1".into(),
                port: 6657,
                bootstrap: vec![],
                out: "out.bin".into(),
            });

        Config {
            node_type: args.node_type.unwrap_or(file_config.node_type),
            host: args.host.unwrap_or(file_config.host),
            port: args.port.unwrap_or(file_config.port),
            bootstrap: args.bootstrap.unwrap_or(file_config.bootstrap),
            out: args.out.unwrap_or(file_config.out),
        }
    }
}
