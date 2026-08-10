use std::fs::File;

use crate::{parser::Parser, store::Store, transactions::TransactionsEngine};

mod types;
mod transactions;
mod parser;
mod store;

fn main() {
    let pwd = std::env::current_dir().expect("Unable to get current dir");
    let args = std::env::args().collect::<Vec<String>>();
    let file_name = args
        .get(1)
        .expect("Please provide the name of file, run it like: trx-processor transactions.csv");
    let file = File::open(pwd.join(file_name)).expect("Error on opening file");
    let mut engine = TransactionsEngine::new(Store::memory());

    let instant = std::time::Instant::now();

    engine.process(Parser::from_args().parse(file));

    for line in engine.as_csv() {
        println!("{line}");
    }

    if args.contains(&"--elapsed".to_string()) {
        let dur = instant.elapsed();
        println!("Duration: {dur:?}");
    }
}