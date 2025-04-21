use super::{Node, NODE_ID_LENGTH};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Distance(pub [u8; NODE_ID_LENGTH]);

impl std::fmt::Debug for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Distance")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDistance(pub Distance, pub Node);

impl Ord for NodeDistance {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl PartialOrd for NodeDistance {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
