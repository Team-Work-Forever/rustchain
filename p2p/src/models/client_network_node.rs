use std::{
    collections::HashMap,
    io::{stdout, Write},
    sync::Arc,
    usize,
};

use crossterm::{
    event::{self, Event, KeyCode},
    style, terminal,
};
use inquire::{error::InquireResult, validator::Validation, CustomType, Text};
use log::info;
use uuid::Uuid;

use crate::{
    blockchain::Transaction,
    models::transactions::{CancelAuction, EndAuction},
    term::{self, TermError},
};

use super::{
    auctions::{auction::Auction, bid::Bid, item::Item, Currency},
    network_node::NetworkNode,
    transactions::{AuctionTransaction, CreateAuction, PlaceBid},
};

pub struct ClientNetworkNode {
    pub network_node: Arc<NetworkNode>,
}

impl ClientNetworkNode {
    pub fn new(network_node: Arc<NetworkNode>) -> Self {
        Self { network_node }
    }

    async fn append_transaction(&self, tx_action: AuctionTransaction) -> Option<Transaction> {
        let kademlia_net = Arc::clone(&self.network_node.kademlia_net);
        info!("Lock here 1");

        let transaction = {
            let Ok(kademlia_tx) = kademlia_net.try_lock() else {
                return None;
            };

            Transaction::new(kademlia_tx.core.keys.clone(), tx_action)?
        };
        info!("UnLock here 1");

        let block_chain = Arc::clone(&self.network_node.block_chain);
        {
            info!("Lock here 2");
            let Ok(block_tx) = block_chain.try_lock() else {
                return None;
            };

            info!("UnLock here 2");

            match block_tx
                .transaction_poll
                .add_transaction(transaction.clone())
            {
                Ok(_) => {
                    info!("Added transaction");
                    Some(transaction)
                }
                Err(_) => None,
            }
        }
    }

    pub async fn create_auction(
        &self,
        item: Item,
        start_price: Currency,
        goal_price: Currency,
    ) -> Option<Transaction> {
        self.append_transaction(AuctionTransaction::Create(CreateAuction::new(
            item,
            start_price,
            goal_price,
        )))
        .await
    }

    pub async fn bid_on_auction(&self, auction_id: Uuid, amount: Currency) -> Option<Transaction> {
        let o = self
            .append_transaction(AuctionTransaction::Bid(PlaceBid::new(auction_id, amount)))
            .await;
        o
    }

    pub async fn cancel_auction(&self, auction_id: Uuid) -> Option<Transaction> {
        self.append_transaction(AuctionTransaction::Cancel(CancelAuction::new(auction_id)))
            .await
    }

    pub async fn terminate_auction(&self, auction_id: Uuid) -> Option<Transaction> {
        self.append_transaction(AuctionTransaction::End(EndAuction::new(auction_id)))
            .await
    }

    pub async fn get_auctions(&self) -> Vec<Auction> {
        let mut auctions: HashMap<Uuid, Auction> = HashMap::new();
        let block_chain = Arc::clone(&self.network_node.block_chain);
        let Ok(block_chain_tx) = block_chain.try_lock() else {
            return vec![];
        };

        for block in block_chain_tx.search_blocks_on(|_| true) {
            for (transaction, auction) in block.get_transaction::<AuctionTransaction>() {
                match auction {
                    AuctionTransaction::Create(create_auction) => {
                        let auction = Auction::new(
                            create_auction.id,
                            transaction.from,
                            create_auction.item.clone(),
                            create_auction.start_price,
                            create_auction.goal_price,
                        );

                        auctions.insert(auction.id, auction);
                    }
                    AuctionTransaction::Bid(place_bid) => {
                        let bid = Bid::new(
                            place_bid.id,
                            transaction.from,
                            place_bid.auction_id,
                            place_bid.amount,
                        );

                        let Some(auction) = auctions.get_mut(&bid.auction_id) else {
                            continue;
                        };

                        auction.add_bid(bid);
                    }
                    AuctionTransaction::Cancel(cancel_auction) => {
                        let Some(auction) = auctions.get_mut(&cancel_auction.auction_id) else {
                            continue;
                        };

                        auction.cancel();
                    }
                    AuctionTransaction::End(end_auction) => {
                        let Some(auction) = auctions.get_mut(&end_auction.auction_id) else {
                            continue;
                        };

                        auction.terminate();
                    }
                }
            }
        }

        auctions.values().cloned().collect()
    }

    pub async fn get_auction(&self, auction_id: Uuid) -> Option<Auction> {
        self.get_auctions()
            .await
            .into_iter()
            .find(|auction| auction.id == auction_id)
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

    // pub/sub receive transaction ""
    // 1. Dispach an event that handles when a new block is receved and checks for a specific transaction
    // 2. Or wait for block chain insertion, once inserted, then view and dispach a new transaction

    fn set_amount(&self, message: &str, minimum: u32) -> InquireResult<u32> {
        CustomType::new(message)
            .with_validator(move |&val: &u32| {
                match val <= minimum {
                    true => {
                        return Ok(Validation::Invalid(
                            "Your bid must be higher than the current highest bid.".into(),
                        ));
                    }
                    false => (),
                }
                Ok(Validation::Valid)
            })
            .with_error_message("Please enter a valid positive number.")
            .prompt()
    }

    pub async fn view_auction(&self, auction: &Auction) -> Result<(), TermError> {
        let mut stdout = stdout();
        term::hide_cursor(true)?;
        terminal::enable_raw_mode()?;

        loop {
            term::reset()?;
            term::print_title("===  Auction  ===", style::Color::Cyan)?;

            term::move_cursor(0, 4)?;
            term::println("Incoming Bids", style::Color::Grey)?;

            for (i, bid) in auction.history.iter().enumerate() {
                let color = if i == 0 {
                    style::Color::Green
                } else {
                    style::Color::Grey
                };

                term::println(
                    format!("{} placed a {} € bid", hex::encode(&bid.buyer), bid.amount).as_str(),
                    color,
                )?;
            }

            term::move_cursor(0, 11)?;
            term::println("## Bid - <B> || Exit - <Q>", style::Color::Yellow)?;

            stdout.flush()?;

            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('b') => {
                        term::move_cursor(0, 13)?;

                        let amount =
                            match self.set_amount("Enter the amount: ", auction.current_price) {
                                Ok(value) => value,
                                Err(_) => continue,
                            };

                        self.bid_on_auction(auction.id, amount).await;
                    }
                    _ => {}
                }
            }
        }

        term::hide_cursor(false)?;

        Ok(())
    }

    pub async fn view_auctions(&self) -> Result<(), TermError> {
        let fetch_auctions = self.get_auctions().await;
        let auction_info: Vec<String> = fetch_auctions
            .iter()
            .map(|auction| {
                format!(
                    "{}\r\n \r\t Description: {} \r\n \r\t Goal Price: {} € \r\n  \r\t Current Bid: {} € \r\n",
                    auction.item.name,
                    auction.item.description,
                    auction.goal_price,
                    auction.current_price
                )
            })
            .collect();

        loop {
            let opt = term::menu("=== Incoming Actions ===".to_string(), auction_info.clone())?;

            if opt == usize::MAX {
                break;
            }

            if let Some(auction) = fetch_auctions.get(opt) {
                self.view_auction(auction).await?;
            }
        }

        Ok(())
    }

    pub async fn view_create_auction(&self) -> Result<(), TermError> {
        let mut stdout = stdout();
        term::hide_cursor(true)?;
        terminal::enable_raw_mode()?;

        loop {
            term::reset()?;
            term::print_title("===  Create Auction  ===", style::Color::Cyan)?;

            term::move_cursor(0, 4)?;
            term::println("", style::Color::Yellow)?;

            let name = match Text::new("Enter item name: ").prompt() {
                Ok(value) => value,
                Err(_) => break,
            };

            let description = match Text::new("Enter item description: ").prompt() {
                Ok(value) => value,
                Err(_) => continue,
            };

            let start_price = match self.set_amount("Enter the start price: ", 0) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let goal_price = match self.set_amount("Enter the goal price: ", 0) {
                Ok(value) => value,
                Err(_) => continue,
            };

            self.create_auction(Item::new(name, description), start_price, goal_price)
                .await;

            break;
        }

        stdout.flush()?;
        term::wait_for_enter()?;

        term::hide_cursor(false)?;

        Ok(())
    }

    pub async fn init_ui(&self) -> Result<(), TermError> {
        loop {
            let opt = term::menu(
                "=== Auction Sys ===".to_string(),
                vec!["Create Auction".to_string(), "View Auctions".to_string()],
            )?;

            match opt {
                0 => self.view_create_auction().await?,
                1 => self.view_auctions().await?,
                usize::MAX => break,
                _ => continue,
            }
        }

        Ok(())
    }
}
