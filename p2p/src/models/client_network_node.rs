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
    kademlia::secret_key::SecretPair,
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
    pub key_pair: SecretPair,
}

impl ClientNetworkNode {
    pub fn new(network_node: Arc<NetworkNode>) -> Self {
        let pairkeys = {
            let kadmlia = network_node
                .kademlia_net
                .try_lock()
                .expect("Failed to lock kademlia_net");

            kadmlia.core.keys.clone()
        };

        Self {
            network_node,
            key_pair: pairkeys,
        }
    }

    async fn append_transaction(&self, tx_action: AuctionTransaction) -> Option<Transaction> {
        let transaction = Transaction::new(self.key_pair.clone(), tx_action)?;
        let block_chain = Arc::clone(&self.network_node.block_chain);

        {
            let Ok(block_tx) = block_chain.try_lock() else {
                return None;
            };

            match block_tx
                .transaction_poll
                .add_transaction(transaction.clone())
            {
                Ok(_) => Some(transaction),
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

    fn set_amount(&self, message: &str, minimum: u32, start_value: u32) -> InquireResult<u32> {
        CustomType::new(message)
            .with_validator(move |&val: &u32| {
                if val <= minimum {
                    return Ok(Validation::Invalid(
                        "Your bid must be higher than the current amount.".into(),
                    ));
                }

                if val <= start_value {
                    return Ok(Validation::Invalid(
                        "Your bid must be higher than the start amount.".into(),
                    ));
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

            if auction.is_terminated() {
                let winner = match auction.get_winner() {
                    Ok(winner) => winner,
                    Err(e) => {
                        term::println(format!("Error: {}", e).as_str(), style::Color::Red)?;
                        break;
                    }
                };

                term::println(
                    format!(
                        "Winner {} placed a {} € bid\n",
                        hex::encode(&winner.buyer),
                        winner.amount
                    )
                    .as_str(),
                    style::Color::Yellow,
                )?;
            }

            for (i, bid) in auction.get_last_five_bids().iter().enumerate() {
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
            term::println(
                format!(
                    "## {} {} Exit - <Q>",
                    if auction.seller != self.key_pair.public_key {
                        "Bid - <B> || "
                    } else {
                        ""
                    },
                    if auction.seller != self.key_pair.public_key {
                        ""
                    } else if auction.can_modify() {
                        "Cancel - <C> || Terminate - <T> ||"
                    } else {
                        "Ended"
                    }
                )
                .as_str(),
                style::Color::Yellow,
            )?;

            stdout.flush()?;

            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('t') => {
                        if auction.seller != self.key_pair.public_key {
                            term::println(
                                "You cannot terminate an auction you do not own.",
                                style::Color::Red,
                            )?;
                            continue;
                        }

                        term::move_cursor(0, 13)?;

                        if let Some(_) = self.terminate_auction(auction.id).await {
                            info!("Auction {} terminated successfully", auction.id);
                        } else {
                            info!("Failed to terminate auction {}", auction.id);
                        }
                    }
                    KeyCode::Char('c') => {
                        if auction.seller != self.key_pair.public_key {
                            term::println(
                                "You cannot cancel an auction you do not own.",
                                style::Color::Red,
                            )?;
                            continue;
                        }

                        term::move_cursor(0, 13)?;

                        if let Some(_) = self.cancel_auction(auction.id).await {
                            info!("Auction {} canceled successfully", auction.id);
                        } else {
                            info!("Failed to cancel auction {}", auction.id);
                        }
                    }
                    KeyCode::Char('b') => {
                        if auction.seller == self.key_pair.public_key {
                            term::println(
                                "You cannot bid on your own auction.",
                                style::Color::Red,
                            )?;

                            continue;
                        }

                        term::move_cursor(0, 13)?;

                        let amount = match self.set_amount(
                            "Enter the amount: ",
                            auction.current_price,
                            auction.start_price,
                        ) {
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
                    "{}\r\n \r\t Description: {} \r\n \r\t Start Price: {} € \r\n \r\t Goal Price: {} € \r\n  \r\t Current Bid: {} € \r\n \r\t State: {}",
                    auction.item.name,
                    auction.item.description,
                    auction.start_price,
                    auction.goal_price,
                    auction.current_price,
                    auction.get_state(),
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

            let start_price = match self.set_amount("Enter the start price: ", 0, 0) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let goal_price = match self.set_amount("Enter the goal price: ", 0, 0) {
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

    pub async fn show_block_chain(&self) -> Result<(), TermError> {
        let mut stdout = stdout();
        term::hide_cursor(true)?;
        terminal::enable_raw_mode()?;
        term::reset()?;

        let block_chain = Arc::clone(&self.network_node.block_chain);
        let Ok(block_chain_tx) = block_chain.try_lock() else {
            term::println("Failed to lock the blockchain.", style::Color::Red)?;
            return Ok(());
        };

        for block in block_chain_tx.search_blocks_on(|_| true) {
            let mut color = style::Color::Magenta;

            if block.header.index % 2 == 0 {
                color = style::Color::Cyan;
            }

            term::println(format!("Index: {}", block.header.index).as_str(), color)?;
            term::println(
                format!("Merkle root: {}", hex::encode(block.header.merkle_root)).as_str(),
                color,
            )?;
            term::println(format!("Nonce: {}", block.header.nonce).as_str(), color)?;
            term::println(
                format!("Hash: {}", hex::encode(block.header.hash)).as_str(),
                color,
            )?;
            term::println(
                format!("Prev Hash: {}", hex::encode(block.header.prev_hash)).as_str(),
                color,
            )?;
            term::println(
                format!("Timestamp: {}\n", block.header.timestamp).as_str(),
                color,
            )?;

            for (transaction, auction) in block.get_transaction::<AuctionTransaction>() {
                let color = style::Color::Green;

                match auction {
                    AuctionTransaction::Create(create_auction) => {
                        term::println(
                            format!(
                                "Created Auction: {} with item {}",
                                create_auction.id, create_auction.item.name
                            )
                            .as_str(),
                            color,
                        )?;
                    }
                    AuctionTransaction::Bid(place_bid) => {
                        term::println(
                            format!(
                                "Bid on Auction {}: {} € by {}",
                                place_bid.auction_id,
                                place_bid.amount,
                                hex::encode(&transaction.from)
                            )
                            .as_str(),
                            color,
                        )?;
                    }
                    AuctionTransaction::Cancel(cancel_auction) => {
                        term::println(
                            format!("Cancelled Auction: {}", cancel_auction.auction_id).as_str(),
                            color,
                        )?;
                    }
                    AuctionTransaction::End(end_auction) => {
                        term::println(
                            format!("Ended Auction: {}", end_auction.auction_id).as_str(),
                            color,
                        )?;
                    }
                }
            }

            term::println("", style::Color::Grey)?;
        }

        term::println("End of Block Chain", style::Color::Cyan)?;
        term::move_cursor(0, 0)?;
        term::println("Press Enter to continue...", style::Color::Yellow)?;

        stdout.flush()?;
        term::wait_for_enter()?;

        term::hide_cursor(false)?;
        Ok(())
    }

    pub async fn view_kademlia(&self) -> Result<(), TermError> {
        loop {
            let opt = term::menu(
                "=== Kademlia Actions ===".to_string(),
                vec![
                    "Find Value".to_string(),
                    // "Find Nodes".to_string(),
                    // "Get Tip".to_string(),
                ],
            )?;

            if opt == usize::MAX {
                break;
            }

            match opt {
                0 => self.show_find_value().await?,
                usize::MAX => break,
                _ => continue,
            }
        }

        Ok(())
    }

    pub async fn show_find_value(&self) -> Result<(), TermError> {
        // let mut stdout = stdout();
        term::hide_cursor(true)?;
        terminal::enable_raw_mode()?;
        term::reset()?;

        term::print_title("=== Find Value ===", style::Color::Cyan)?;
        term::move_cursor(0, 4)?;
        term::println("Enter the value to find:", style::Color::Yellow)?;

        // TODO: Modify this
        term::wait_for_enter()?;

        Ok(())
    }

    pub async fn init_ui(&self) -> Result<(), TermError> {
        loop {
            let opt = term::menu(
                "=== Auction Sys ===".to_string(),
                vec![
                    "Create Auction".to_string(),
                    "View Auctions".to_string(),
                    "Show Block Chain".to_string(),
                    "Kademlia".to_string(),
                ],
            )?;

            match opt {
                0 => self.view_create_auction().await?,
                1 => self.view_auctions().await?,
                2 => self.show_block_chain().await?,
                3 => self.show_find_value().await?,
                usize::MAX => break,
                _ => continue,
            }
        }

        Ok(())
    }
}
