use std::{io::Write, sync::{Arc, atomic::{AtomicU64, Ordering}, mpsc::{Sender, channel}}, thread};
use rand::{RngExt, rngs::{SmallRng, ThreadRng}};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

const PRECISION: f64 = 10000.0; // 4 decimals
static CSV_HEADERS: &str = "type,client,tx,amount\n";

// Since we do only have one place to write the file this approach is just to illustrate how can we handle
// parallelism + speed + safety.
// The answer always is: shard everything you can + make strict atomic mutations.
// We use manual precision to avoid Decimal 128 generators and simplify things.
fn main() {
    let pwd = std::env::current_dir().expect("Unable to get current_dir");
    let mut rng: SmallRng = rand::make_rng();
    let mut file = create_file();

    file.write_all(CSV_HEADERS.as_bytes())
        .expect("Unable to write CSV headers to file");

    let (tx, rx) = channel::<String>();

    let handle = thread::spawn(move || {
        while let Ok(text) = rx.recv() {
            file.write_all(text.as_bytes())
                .expect("Unable to write data to file");
            file.write(b"\n").expect("Unable to write data to file");
        }

        file.flush().expect("Unable to flush data to file");
    });

    let wallets = Arc::new([const { AtomicU64::new(0) }; 100]);

    let num_samples = rng.random_range(1_000..u32::MAX / 1000); // Let's have some fun :)
    let inner = tx.clone();
    println!("Generating {num_samples} samples");
    (0..num_samples).into_par_iter().for_each(move |i| {
        let mut rng = rand::rng();
        let inner = inner.clone();
        let num = rng.random_range(0..2);
        let client_id = rng.random_range(0..100);

        match num {
            0 => gen_deposit(client_id, inner, i, rng, wallets.clone()),
            1 => gen_withdrawal(client_id, inner, i, rng, wallets.clone()),
            2 => gen_dispute(client_id, inner, i, rng),
            _ => unreachable!("unreachable")
        }
    });

    drop(tx);
    handle
        .join()
        .expect("Error occur during write task finalization");

    println!("Gen Trx {pwd:?} count {num_samples}");
}

fn gen_deposit(client_id: u16, tx: Sender<String>, trx: u32, mut rng: ThreadRng, wallets: Arc<[AtomicU64; 100]>) {
    let amount: u64 = rng.random_range(1_0000..10_0000);
    let wallet = if let Some(wallet) = wallets.get(client_id as usize) { wallet } else { return; };
    wallet.fetch_add(amount, Ordering::SeqCst);

    let amount = (amount as f64) / PRECISION;

    let _ = tx.send(format!("deposit,{client_id},{trx},{amount:.4}"));

    if rng.random_range(0.0..1.0) <= 0.25 {
        gen_dispute(client_id, tx, trx, rng);
    }
}

fn gen_withdrawal(client_id: u16, tx: Sender<String>, trx: u32, mut rng: ThreadRng, wallets: Arc<[AtomicU64; 100]>) {
    let amount: u64 = rng.random_range(1_000..1_0000);
    let wallet = if let Some(wallet) = wallets.get(client_id as usize) { wallet } else { return; };

    let wallet_balance = wallet.load(Ordering::Relaxed);

    let result = if wallet_balance > amount {
        wallet.compare_exchange(wallet_balance, wallet_balance - amount, Ordering::Acquire, Ordering::Relaxed).is_ok()
    } else { false };

    if !result {
        return;
    }

    let amount = (amount as f64) / PRECISION;
    let _ = tx.send(format!("withdrawal,{client_id},{trx},{amount:.4}"));

    if rng.random_range(0.0..1.0) <= 0.25 {
        gen_dispute(client_id, tx, trx, rng);
    }
}

fn gen_dispute(client_id: u16, tx: Sender<String>, trx: u32, mut rng: ThreadRng) {
    let _ = tx.send(format!("dispute,{client_id},{trx},"));
    let num = rng.random_range(0.0..1.0);
    if num <= 0.25 {
        gen_resolve(client_id, tx, trx);
    } else if num >= 0.25 && num <= 0.2502 { // We decrease the chances to get blocked
        gen_chargeback(client_id, tx, trx);
    }
}

fn gen_resolve(client_id: u16, tx: Sender<String>, trx: u32) {
    let _ = tx.send(format!("resolve,{client_id},{trx},"));
}

fn gen_chargeback(client_id: u16, tx: Sender<String>, trx: u32) {
    let _ = tx.send(format!("chargerback,{client_id},{trx},"));
}

fn create_file() -> std::fs::File {
    let pwd = std::env::current_dir().expect("Unable to get current directory of application.");
    let file_name = get_file_name();
    let file_path = pwd.join(file_name);

    std::fs::File::create(&file_path)
        .unwrap_or_else(|_| panic!("Unable to create or open file {file_path:?}"))
}

fn get_file_name() -> String {
    let file_name = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .unwrap_or(&"transactions.csv".to_string())
        .to_owned();

    if file_name.ends_with(".csv") {
        file_name.to_owned()
    } else {
        format!("{file_name}.csv")
    }
}
