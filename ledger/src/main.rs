use ledger::blockchain::{BlockChain, Transaction};

fn main() {
    let mut block_chain = BlockChain::new();

    println!("Start to mine block!");
    let mined_block = block_chain.add_block(|mut builder| {
        builder.add_transactions(vec![
            Transaction::new("hash".to_string(), "another".to_string(), "Hit".to_string()),
            Transaction::new("hash".to_string(), "another".to_string(), "Hot".to_string()),
        ]);

        builder
    });
    println!("Blocked Mined and added to the chain");

    for block in block_chain.blocks.iter() {
        println!("{:?}", block)
    }

    println!("Validating block");
    let merkle_root = mined_block.merkle_root;

    if mined_block.validate(merkle_root) {
        println!("Valid!")
    } else {
        print!("Not valid!")
    }
}
