use serde::{Deserialize, Serialize};

use crate::blockchain::TransactionData;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InitAuctionTransaction {
    pub auction_id: String,
}

#[typetag::serde]
impl TransactionData for InitAuctionTransaction {
    fn clone_dyn(&self) -> Box<dyn TransactionData> {
        Box::new(self.clone())
    }
}
