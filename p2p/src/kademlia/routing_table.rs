use super::{KBucket, Node, NodeId, KBUCKET_MAX, NODE_ID_BITS, NODE_ID_LENGTH};

#[derive(Clone)]
pub struct NodeDistance(pub NodeId, pub Node);

#[derive(Clone, Debug)]
pub struct RoutingTable {
    node: NodeId,
    kbuckets: Vec<KBucket>,
}

impl RoutingTable {
    pub fn new(node: Node) -> Self {
        let kbuckets = Self::gen_kbuckets();

        let mut routing_table = Self {
            node: node.clone().id,
            kbuckets,
        };

        routing_table.insert_node(&node);
        routing_table
    }

    fn gen_kbuckets() -> Vec<KBucket> {
        (0..NODE_ID_BITS)
            .map(|depth| KBucket::new(depth, (depth + 1).min(KBUCKET_MAX)))
            .collect()
    }

    fn get_bucket_index(&self, node: Node) -> usize {
        let distance = self.node.distance(&node.id).0;

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

    pub fn insert_node(&mut self, node: &Node) {
        let kbucket_index = self.get_bucket_index(node.clone());

        let Some(kbucket) = self.kbuckets.get_mut(kbucket_index) else {
            return;
        };

        if !kbucket.is_full() {
            return kbucket.insert(node.clone());
        }

        if kbucket.contains(&self.node) {
            let (left, right) = kbucket.split();

            self.kbuckets[kbucket_index] = left;
            self.kbuckets.insert(kbucket_index + 1, right);

            return self.insert_node(node);
        }

        let _oldest_node = kbucket.get_oldest_node().expect("");

        // make ping request

        kbucket.envict_and_insert(node.clone());
    }

    pub fn get_closest_nodes(&self, node: &Node, count: usize) -> Vec<Node> {
        let mut closest_nodes = Vec::<NodeDistance>::new();

        for kbucket in self.kbuckets.iter() {
            for knode in kbucket.get_nodes() {
                let distance = node.id.distance(&knode.id);
                closest_nodes.push(NodeDistance(distance, knode.clone()));
            }
        }

        closest_nodes.sort_by_key(|tuple| tuple.clone().0);

        closest_nodes
            .into_iter()
            .take(count)
            .map(|node_distance| node_distance.1)
            .collect()
    }
}
