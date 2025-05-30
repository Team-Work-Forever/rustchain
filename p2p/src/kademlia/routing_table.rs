use crate::DHTNode;

use super::{
    distance::NodeDistance, k_bucket::KBucket, Node, NodeId, KBUCKET_MAX, NODE_ID_BITS,
    NODE_ID_LENGTH,
};

#[derive(Clone, Debug)]
pub struct RoutingTable {
    host: Node,
    kbuckets: Vec<KBucket>,
}

impl RoutingTable {
    pub async fn new(node: Node) -> Self {
        let kbuckets = Self::gen_kbuckets();

        let mut routing_table = Self {
            host: node.clone(),
            kbuckets,
        };

        routing_table.insert_node(&node).await;
        routing_table
    }

    fn gen_kbuckets() -> Vec<KBucket> {
        (0..NODE_ID_BITS)
            .map(|depth| KBucket::new(depth, (depth + 1).min(KBUCKET_MAX)))
            .collect()
    }

    fn get_bucket_index(&self, node: Node) -> usize {
        let distance = self.host.id.distance(&node.id).0;

        if distance.iter().all(|bit| *bit == 0) {
            return 0;
        }

        for i in 0..NODE_ID_LENGTH {
            for j in (0..8).rev() {
                if (distance[i] >> (7 - j)) & 0x1 != 0 {
                    return i * 8 + j;
                }
            }
        }

        NODE_ID_LENGTH - 1
    }

    pub(crate) async fn insert_node(&mut self, node: &Node) {
        let kbucket_index = self.get_bucket_index(node.clone());

        let Some(kbucket) = self.kbuckets.get_mut(kbucket_index) else {
            return;
        };

        if !kbucket.is_full() {
            return kbucket.insert(node.clone());
        }

        if kbucket.contains(&self.host.id) {
            let (left, right) = kbucket.split();

            self.kbuckets[kbucket_index] = left;
            self.kbuckets.insert(kbucket_index + 1, right);

            return Box::pin(self.insert_node(node)).await;
        }

        let oldest_node = kbucket.get_oldest_node().expect("");

        let Err(_) = DHTNode::ping(&self.host, &oldest_node).await else {
            return;
        };

        kbucket.envict_and_insert(node.clone());
    }

    pub(crate) fn remove(&mut self, node: &Node) {
        let node_clone = node.clone();
        let kbucket_index = self.get_bucket_index(node_clone);

        let Some(kbucket) = self.kbuckets.get_mut(kbucket_index) else {
            return;
        };

        kbucket.remove(node.clone());
    }

    pub(crate) async fn get_closest_nodes(
        &mut self,
        key: &NodeId,
        count: usize,
    ) -> Vec<NodeDistance> {
        let mut distances = self
            .kbuckets
            .iter()
            .flat_map(|bucket| bucket.get_nodes())
            .filter(|kbucket| kbucket.id != self.host.id)
            .map(|knode| NodeDistance(key.distance(&knode.id), knode.clone()))
            .collect::<Vec<_>>();

        distances.sort();
        let sort_distances = distances.into_iter().take(count).collect::<Vec<_>>();

        for distance_node in &sort_distances {
            self.insert_node(&distance_node.1).await;
        }

        sort_distances
    }
}
