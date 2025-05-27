use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::{bid::Bid, item::Item, Currency, PublicKey, Timestamp};

#[derive(Debug, Error)]
pub enum AuctionError {
    #[error("Auction isn't terminated")]
    StillRunning,

    #[error("Failed to fetch highest bid")]
    FailedToFetchHighestBid,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Auction {
    pub id: Uuid,
    pub seller: PublicKey,
    pub item: Item,
    pub start_price: Currency,
    pub goal_price: Currency,
    pub current_price: Currency,
    pub history: Vec<Bid>,
    pub started_at: Timestamp,
    pub cancel_at: Timestamp,
    pub ended_at: Timestamp,
}

impl std::fmt::Debug for Auction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auction")
            .field("id", &self.id)
            .field("seller", &hex::encode(&self.seller))
            .field("item", &self.item)
            .field("start_price", &self.start_price)
            .field("goal_price", &self.goal_price)
            .field("current_price", &self.current_price)
            .field("history", &self.history)
            .field("started_at", &self.started_at)
            .field("cancel_at", &self.cancel_at)
            .field("ended_at", &self.ended_at)
            .finish()
    }
}

impl Auction {
    pub fn new(
        id: Uuid,
        seller: PublicKey,
        item: Item,
        start_price: Currency,
        goal_price: Currency,
    ) -> Self {
        let timestamp = Utc::now().timestamp();

        Self {
            id,
            seller,
            item,
            start_price,
            goal_price,
            current_price: 0,
            history: vec![],
            started_at: timestamp,
            cancel_at: 0,
            ended_at: 0,
        }
    }

    pub fn terminate(&mut self) {
        self.ended_at = Utc::now().timestamp()
    }

    pub fn cancel(&mut self) {
        self.cancel_at = Utc::now().timestamp()
    }

    pub fn get_state(&self) -> String {
        if self.is_canceled() {
            return "Canceled".to_string();
        }

        if self.is_terminated() {
            return "Terminated".to_string();
        }

        "Open".to_string()
    }

    fn is_canceled(&self) -> bool {
        self.cancel_at != 0
    }

    pub fn is_terminated(&self) -> bool {
        self.ended_at != 0
    }

    pub fn can_modify(&self) -> bool {
        !self.is_canceled() || !self.is_terminated()
    }

    fn update_current_price(&mut self) {
        let Some(highest_bid) = self.get_highest_bid() else {
            return;
        };

        self.current_price = highest_bid.amount;
    }

    pub fn add_bid(&mut self, bid: Bid) {
        if !self.can_modify() {
            return;
        }

        if bid.buyer == self.seller {
            return;
        }

        if let Some(last_bid) = self.get_highest_bid() {
            if bid.amount <= last_bid.amount {
                return;
            }
        }

        self.history.push(bid);
        self.update_current_price();
    }

    pub fn get_highest_bid(&self) -> Option<Bid> {
        self.history.iter().cloned().max_by(|a, b| {
            a.amount
                .cmp(&b.amount)
                .then_with(|| b.created_at.cmp(&a.created_at))
        })
    }

    pub fn get_last_five_bids(&self) -> Vec<Bid> {
        let mut history = self.history.clone();
        history.sort_by(|a, b| b.amount.cmp(&a.amount));
        history.iter().take(5).cloned().collect()
    }

    pub fn get_winner(&self) -> Result<Bid, AuctionError> {
        if !self.is_terminated() {
            return Err(AuctionError::StillRunning);
        }

        let Some(highest_bid) = self.get_highest_bid() else {
            return Err(AuctionError::FailedToFetchHighestBid);
        };

        Ok(highest_bid)
    }
}
