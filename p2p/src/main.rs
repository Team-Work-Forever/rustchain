use bincode::{Decode, Encode};
use p2p::{
    kademlia::{dht::KademliaData, NodeId},
    DHTNode,
};

#[derive(Clone, Encode, Debug, Decode)]
struct MyData {
    pub name: String,
}

impl MyData {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl KademliaData for MyData {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = DHTNode::<MyData>::bootstrap("127.0.0.1".into(), 5000)
        .await
        .expect("Error creating bootstrap node");

    let node1 = DHTNode::<MyData>::new(bootstrap.clone(), "127.0.0.1".into(), 5006)
        .await
        .expect("Error creating node 1");

    let node2 = DHTNode::<MyData>::new(bootstrap.clone(), "127.0.0.1".into(), 5007)
        .await
        .expect("Error creating node 2");

    let Some(store_key) = NodeId::random() else {
        panic!("Failed to create key");
    };

    let value_store = MyData::new("Diogo Assunção".into());
    if let Err(_) = node1.store(&store_key, value_store).await {
        panic!("Failed to propagate value thru network");
    };

    if let Ok(Some(value)) = node2.find_value(&store_key).await {
        println!("My data is something like: {}, ain't ya ;)", value.name);
    }

    println!();
    println!("Node 1");
    println!("{:?}", node1.distributed_hash_tb);
    println!();
    println!("Node 2");
    println!("{:?}", node2.distributed_hash_tb);

    Ok(())
}
