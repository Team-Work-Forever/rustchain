use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Currency, PublicKey, Timestamp};

#[derive(Clone, Serialize, Deserialize)]
pub struct Bid {
    pub id: Uuid,
    pub auction_id: Uuid,
    pub buyer: PublicKey,
    pub amount: Currency,
    pub created_at: Timestamp,
}

impl std::fmt::Debug for Bid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bid")
            .field("id", &self.id)
            .field("auction_id", &self.auction_id)
            .field("buyer", &hex::encode(&self.buyer))
            .field("amount", &self.amount)
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl Bid {
    pub fn new(id: Uuid, buyer: PublicKey, auction_id: Uuid, amount: Currency) -> Self {
        let created_at = Utc::now().timestamp();

        Self {
            id,
            auction_id,
            buyer,
            amount,
            created_at,
        }
    }
}
