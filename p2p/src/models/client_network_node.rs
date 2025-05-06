use std::sync::Arc;

use super::network_node::NetworkNode;

pub struct ClientNetworkNode {
    pub network_node: Arc<NetworkNode>,
}

impl ClientNetworkNode {
    pub fn new(network_node: Arc<NetworkNode>) -> Self {
        Self { network_node }
    }

    // If is client do a check auth thinghy?
    // 1. ask for email + password
    // 2. hash it
    // 3. sign it
    // 4. store it

    // verify
    // 1. hash(email + password)
    // 2. get it (accountId + pubkey)
    // 3. verify signature
    // 4. hash == hash - all good

    // add transaction
    // Create Auction
    // Terminate Action
    // Bid Action
    // Get All Auctions
    // Get Auction

    // pub/sub receive transaction ""
    // 1. Dispach an event that handles when a new block is receved and checks for a specific transaction
    // 2. Or wait for block chain insertion, once inserted, then view and dispach a new transaction

    pub fn menu(&self) {
        println!("Menu stuff")
    }
}
