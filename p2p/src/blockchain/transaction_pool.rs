use log::error;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use super::{block, Transaction};

#[derive(Clone, Debug, Default)]
pub struct TransactionPool {
    transaction_pool: Arc<Mutex<VecDeque<Transaction>>>,
}

impl TransactionPool {
    pub fn new() -> Self {
        Self {
            transaction_pool: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn get_lock_pool(&self) -> Result<MutexGuard<VecDeque<Transaction>>, ()> {
        self.transaction_pool
            .lock()
            .map_err(|e| error!("Failed to lock transaction pool {}", e))
    }

    pub fn add_transaction(&self, transaction: Transaction) -> Result<(), ()> {
        let mut pool = self.get_lock_pool()?;
        pool.push_back(transaction);

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.get_lock_pool().map_or(true, |p| p.is_empty())
    }

    pub fn arc(&self) -> Arc<Mutex<VecDeque<Transaction>>> {
        Arc::clone(&self.transaction_pool)
    }

    pub(crate) fn fetch_batch_transactions(
        &mut self,
        batch_size: usize,
    ) -> Result<Vec<Transaction>, ()> {
        let mut pool = self.get_lock_pool()?;

        if pool.is_empty() {
            return Ok(vec![]);
        }

        let end = batch_size.min(block::MAX_TRANSACTION).min(pool.len());
        Ok(pool.drain(0..end).collect::<Vec<_>>())
    }
}
