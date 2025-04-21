use bincode::Encode;
use p2p::{kademlia::dht::KademliaData, DHTNode};

// Connect to network
// 1. Add bootstrap to my routing table
// 2. FIND_NODE to my self, in order to trigger the ns_look_up on bootstrap
// 3. (FIND_NODE) returns all it's closest nodes
// 4. I add those to my list!

#[derive(Clone, Encode, Debug)]
struct MyData {}

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

    match DHTNode::<MyData>::ping(&node1.core, &node2.core).await {
        Ok(_) => print!("They speak!"),
        Err(_) => panic!("Well they tried..."),
    }

    println!("Node 1");
    println!("{:?}", node1.routing_table);
    println!();
    println!("Node 2");
    println!("{:?}", node2.routing_table);

    Ok(())
}
