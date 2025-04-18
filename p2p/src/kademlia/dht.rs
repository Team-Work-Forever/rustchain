use std::collections::HashMap;

use super::{Node, NodeId, RoutingTable};

#[derive(Debug)]
pub struct DHTNode<TData> {
    _node: Node, // current node (ip, port, node id)

    pub routing_table: RoutingTable, // Routing table of the current node
    _distributed_hash_tb: HashMap<NodeId, TData>, // actual storage implementation, btw i thing that TData will be the blockchain or the blocks

                                                  // network implementation a.k.a. grpc
}

impl<TData> DHTNode<TData> {
    pub fn new() -> Self {
        let node = Node::new("address".into(), 5000);

        Self {
            _node: node.clone(),
            routing_table: RoutingTable::new(node),
            _distributed_hash_tb: HashMap::new(),
        }
    }

    // start_multi_thread_services (start_miner, start grpc connection and stuff...)
}
