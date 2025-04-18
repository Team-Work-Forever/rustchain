use p2p::{DHTNode, Node};

fn main() {
    let node = Node::test("address".into(), 600);
    let node1 = Node::ed("address".into(), 100);
    let yet_another_node = Node::yet_another_new("address".into(), 900);

    let mut kademlia_node = DHTNode::<u32>::new();
    kademlia_node.routing_table.insert_node(&node);
    kademlia_node.routing_table.insert_node(&node1);
    kademlia_node.routing_table.insert_node(&yet_another_node);

    let nodes = kademlia_node
        .routing_table
        .get_closest_nodes(&yet_another_node, 3);

    println!("{:?}", nodes);
    println!();
    println!("{:?}", kademlia_node.routing_table);
}
