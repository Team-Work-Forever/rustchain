use std::any::Any;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::blockchain::{DoubleHasher, HashFunc, TransactionData};

use super::auctions::{item::Item, Currency, PublicKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuction {
    pub id: Uuid,
    pub item: Item,
    pub start_price: Currency,
    pub goal_price: Currency,
}

impl CreateAuction {
    pub fn new(item: Item, start_price: Currency, goal_price: Currency) -> Self {
        Self {
            id: Uuid::new_v4(),
            item,
            start_price,
            goal_price,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceBid {
    pub id: Uuid,
    pub auction_id: Uuid,
    pub amount: Currency,
}

impl PlaceBid {
    pub fn new(auction_id: Uuid, amount: Currency) -> Self {
        Self {
            id: Uuid::new_v4(),
            auction_id,
            amount,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAuction {
    pub auction_id: Uuid,
}

impl CancelAuction {
    pub fn new(auction_id: Uuid) -> Self {
        Self { auction_id }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndAuction {
    pub auction_id: Uuid,
}

impl EndAuction {
    pub fn new(auction_id: Uuid) -> Self {
        Self { auction_id }
    }
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub enum AuctionTransaction {
    Create(CreateAuction),
    Bid(PlaceBid),
    Cancel(CancelAuction),
    End(EndAuction),
}

#[typetag::serde]
impl TransactionData for AuctionTransaction {
    fn get_hash(&self) -> Option<PublicKey> {
        let config = bincode::config::standard();
        let encoded = match bincode::serde::encode_to_vec(self, config) {
            Ok(data) => data,
            Err(_) => return None,
        };

        let hasher = DoubleHasher::default();
        Some(hasher.hash(hex::encode(encoded)))
    }

    fn clone_dyn(&self) -> Box<dyn TransactionData> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
