use std::collections::VecDeque;

use super::{Node, NodeId};

#[derive(Clone, Debug)]
pub(crate) struct KBucket {
    nodes: VecDeque<Node>,
    bucket_size: usize,
    pub depth: usize,
}

impl KBucket {
    pub fn new(depth: usize, bucket_size: usize) -> Self {
        Self {
            nodes: VecDeque::with_capacity(bucket_size),
            bucket_size,
            depth,
        }
    }

    pub fn contains(&self, node_id: &NodeId) -> bool {
        self.nodes.iter().any(|n| n.id == *node_id)
    }

    pub fn is_full(&self) -> bool {
        return self.nodes.len() == self.bucket_size;
    }

    pub fn envict_and_insert(&mut self, node: Node) {
        self.nodes.pop_front();
        self.insert(node);
    }

    pub fn remove(&mut self, node: Node) {
        if let Some(pos) = self.nodes.iter().position(|n| n.id == node.id) {
            self.nodes.remove(pos);
        }
    }

    pub fn get_first_node(&self) -> Option<Node> {
        self.nodes.get(0).cloned()
    }

    pub fn get_oldest_node(&self) -> Option<Node> {
        self.nodes.front().cloned()
    }

    pub fn insert(&mut self, node: Node) {
        if let Some(node_pos) = self.nodes.iter().position(|n| n.id == node.id) {
            self.nodes.remove(node_pos);
            self.nodes.push_back(node);
            return;
        }

        if self.nodes.len() >= self.bucket_size {
            return;
        }

        self.nodes.push_back(node);
    }

    pub fn split(&self) -> (KBucket, KBucket) {
        let next_depth = self.depth + 1;

        let mut left_bucket = KBucket::new(next_depth, self.bucket_size);
        let mut right_bucket = KBucket::new(next_depth, self.bucket_size);

        for node in self.nodes.iter() {
            if Self::get_bit(&node.id, self.depth) {
                right_bucket.insert(node.clone());
            } else {
                left_bucket.insert(node.clone());
            }
        }

        (left_bucket, right_bucket)
    }

    fn get_bit(id: &NodeId, bit_index: usize) -> bool {
        let byte_index = bit_index / 8;
        let bit_offset = 7 - (bit_index % 8);

        if byte_index >= id.0.len() {
            return false;
        }

        (id.0[byte_index] >> bit_offset) & 1 == 1
    }

    pub fn get_nodes(&self) -> impl Iterator<Item = &Node> {
        return self.nodes.iter();
    }
}
