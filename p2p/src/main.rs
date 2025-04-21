use p2p::DHTNode;

// Connect to network
// 1. Add bootstrap to my routing table
// 2. FIND_NODE to my self, in order to trigger the ns_look_up on bootstrap
// 3. (FIND_NODE) returns all it's closest nodes
// 4. I add those to my list!

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = DHTNode::<String>::bootstrap("127.0.0.1".into(), 5000)
        .await
        .expect("Error creating bootstrap node");

    let node1 = DHTNode::<String>::new(bootstrap.clone(), "127.0.0.1".into(), 5006)
        .await
        .expect("Error creating node 1");

    let node2 = DHTNode::<String>::new(bootstrap.clone(), "127.0.0.1".into(), 5007)
        .await
        .expect("Error creating node 2");

    match DHTNode::<String>::ping(&node1.core, &node2.core).await {
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
