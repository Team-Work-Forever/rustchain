use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "p2p",
    version,
    about = "create and connect to the peer to peer network"
)]
pub struct Arguments {
    #[arg(long, default_value_t = NodeType::Join)]
    pub node_type: NodeType,

    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long)]
    pub port: usize,

    #[arg(long, value_delimiter = ',')]
    pub bootstrap: Vec<String>,

    #[arg(long, default_value = "./out.bin")]
    pub out: PathBuf,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
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
