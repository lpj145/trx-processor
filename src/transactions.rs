use crate::{
    store::{Key, Store},
    types::{Amount, CsvItem, LockStatus, TransactionRecord, TxStatus},
};

pub struct TransactionsEngine {
    store: Store,
}

impl TransactionsEngine {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn process<I: Iterator<Item = CsvItem>>(&mut self, data: I) {
        for item in data {
            match item {
                CsvItem::Deposit(client_id, trx, amount) => {
                    if self.store.get::<LockStatus>(locked_key(client_id)).map_or(false, |l| l.0) {
                        // println!("[warn] account {client_id} is locked, cannot receive funds!");
                        continue;
                    }
                    if self.store.exists(trx_key(client_id, trx)) {
                        //println!("[warn] duplicated deposit detected {trx}");
                        continue;
                    }

                    let tx_record = TransactionRecord {
                        client_id,
                        tx_id: trx,
                        amount,
                        status: TxStatus::Normal,
                    };
                    self.store.put(trx_key(client_id, trx), tx_record);

                    self.store.upsert(available_key(client_id), Amount(0), |value| {
                        Amount(value.0 + amount)
                    });

                    self.store.upsert(total_key(client_id), Amount(0), |value| {
                        Amount(value.0 + amount)
                    });
                }
                CsvItem::Withdrawal(client_id, _trx, amount) => {
                    if self.store.get::<LockStatus>(locked_key(client_id)).map_or(false, |l| l.0) {
                        // println!("[warn] account {client_id} is locked, cannot withdraw funds!");
                        continue;
                    }
                    let available: Amount = self.store.get(available_key(client_id)).unwrap_or_default();
                    if available.0 < amount {
                        continue;
                    }

                    self.store.upsert(available_key(client_id), Amount(0), |value| {
                        Amount(value.0 - amount)
                    });

                    self.store.upsert(total_key(client_id), Amount(0), |value| {
                        Amount(value.0 - amount)
                    });
                }
                CsvItem::Dispute(client_id, trx) => {
                    if self.store.get::<LockStatus>(locked_key(client_id)).map_or(false, |l| l.0) {
                        continue;
                    }
                    let mut tx_record = if let Some(tx) = self.store.get::<TransactionRecord>(trx_key(client_id, trx)) {
                        tx
                    } else {
                        //println!("[warn] trx {trx} not exists and could not be disputed!");
                        continue;
                    };

                    if tx_record.client_id != client_id || tx_record.status != TxStatus::Normal {
                        //println!("[warn] trx {trx} is already disputed");
                        continue;
                    }

                    let amount = tx_record.amount;
                    tx_record.status = TxStatus::Disputed;
                    self.store.put(trx_key(client_id, trx), tx_record);

                    self.store.upsert(available_key(client_id), Amount(0), |value| {
                        Amount(value.0.saturating_sub(amount))
                    });

                    self.store.upsert(held_key(client_id), Amount(0), |value| {
                        Amount(value.0 + amount)
                    });
                }
                CsvItem::Resolve(client_id, trx) => {
                    if self.store.get::<LockStatus>(locked_key(client_id)).map_or(false, |l| l.0) {
                        continue;
                    }
                    let mut tx_record = if let Some(tx) = self.store.get::<TransactionRecord>(trx_key(client_id, trx)) {
                        tx
                    } else {
                        //println!("[warn] trx {trx} not exists and could not be resolved!");
                        continue;
                    };

                    if tx_record.client_id != client_id || tx_record.status != TxStatus::Disputed {
                        continue;
                    }

                    let amount = tx_record.amount;
                    tx_record.status = TxStatus::Normal;
                    self.store.put(trx_key(client_id, trx), tx_record);

                    self.store.upsert(available_key(client_id), Amount(0), |value| {
                        Amount(value.0 + amount)
                    });

                    self.store.upsert(held_key(client_id), Amount(0), |value| {
                        Amount(value.0.saturating_sub(amount))
                    });
                }
                CsvItem::ChargerBack(client_id, trx) => {
                    if self.store.get::<LockStatus>(locked_key(client_id)).map_or(false, |l| l.0) {
                        continue;
                    }
                    let mut tx_record = if let Some(tx) = self.store.get::<TransactionRecord>(trx_key(client_id, trx)) {
                        tx
                    } else {
                        //println!("[warn] trx {trx} not exists and could not be disputed!");
                        continue;
                    };

                    if tx_record.client_id != client_id || tx_record.status != TxStatus::Disputed {
                        continue;
                    }

                    let amount = tx_record.amount;
                    tx_record.status = TxStatus::ChargedBack;
                    self.store.put(trx_key(client_id, trx), tx_record);

                    self.store.upsert(held_key(client_id), Amount(0), |value| {
                        Amount(value.0.saturating_sub(amount))
                    });

                    self.store.upsert(total_key(client_id), Amount(0), |value| {
                        Amount(value.0.saturating_sub(amount))
                    });

                    self.store.put(locked_key(client_id), LockStatus(true));
                }
            }
        }
    }

    pub fn as_csv(&self) -> impl Iterator<Item = String> {
        let mut client_ids = std::collections::BTreeSet::new();
        for (key, _) in self.store.iter() {
            if key.len() >= 4 && (&key[0..2] == b"ca" || &key[0..2] == b"ch" || &key[0..2] == b"ct" || &key[0..2] == b"cl") {
                let client_id = u16::from_be_bytes([key[2], key[3]]);
                client_ids.insert(client_id);
            }
        }

        let mut lines = vec!["client,available,held,total,locked".to_string()];
        for client_id in client_ids {
            let available = self.store.get::<Amount>(available_key(client_id)).unwrap_or_default().0;
            let held = self.store.get::<Amount>(held_key(client_id)).unwrap_or_default().0;
            let total = self.store.get::<Amount>(total_key(client_id)).unwrap_or_default().0;
            let locked = self.store.get::<LockStatus>(locked_key(client_id)).unwrap_or_default().0;

            lines.push(format!(
                "{client_id},{},{},{},{locked}",
                format_amount(available),
                format_amount(held),
                format_amount(total)
            ));
        }
        lines.into_iter()
    }
}

fn format_amount(val: u64) -> String {
    let int_part = val / 10000;
    let frac_part = val % 10000;
    format!("{int_part}.{frac_part:04}")
}

#[inline]
fn available_key(client_id: u16) -> Key {
    let mut tmp = [0u8; 8];
    tmp[0..2].copy_from_slice(b"ca");
    tmp[2..4].copy_from_slice(&client_id.to_be_bytes());

    tmp
}

#[inline]
fn held_key(client_id: u16) -> Key {
    let mut tmp = [0u8; 8];
    tmp[0..2].copy_from_slice(b"ch");
    tmp[2..4].copy_from_slice(&client_id.to_be_bytes());

    tmp
}

#[inline]
fn total_key(client_id: u16) -> Key {
    let mut tmp = [0u8; 8];
    tmp[0..2].copy_from_slice(b"ct");
    tmp[2..4].copy_from_slice(&client_id.to_be_bytes());

    tmp
}

#[inline]
fn locked_key(client_id: u16) -> Key {
    let mut tmp = [0u8; 8];
    tmp[0..2].copy_from_slice(b"cl");
    tmp[2..4].copy_from_slice(&client_id.to_be_bytes());

    tmp
}

#[inline]
fn trx_key(client_id: u16, trx: u32) -> Key {
    let mut tmp = [0u8; 8];
    tmp[0..2].copy_from_slice(b"tx");
    tmp[2..4].copy_from_slice(&client_id.to_be_bytes());
    tmp[4..8].copy_from_slice(&trx.to_be_bytes());

    tmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;
    use crate::types::CsvItem;

    #[test]
    fn test_deposit_and_withdrawal() {
        let mut engine = TransactionsEngine::new(Store::memory());
        let items = vec![
            CsvItem::Deposit(1, 1, 10000), // deposit 1.0000
            CsvItem::Withdrawal(1, 2, 4000), // withdraw 0.4000
        ];
        engine.process(items.into_iter());

        let lines: Vec<String> = engine.as_csv().collect();
        assert_eq!(lines[0], "client,available,held,total,locked");
        assert_eq!(lines[1], "1,0.6000,0.0000,0.6000,false");
    }

    #[test]
    fn test_dispute_and_resolve() {
        let mut engine = TransactionsEngine::new(Store::memory());
        let items = vec![
            CsvItem::Deposit(1, 1, 10000),
            CsvItem::Dispute(1, 1),
        ];
        engine.process(items.into_iter());

        let lines: Vec<String> = engine.as_csv().collect();
        assert_eq!(lines[1], "1,0.0000,1.0000,1.0000,false");

        let resolve_items = vec![CsvItem::Resolve(1, 1)];
        engine.process(resolve_items.into_iter());

        let lines: Vec<String> = engine.as_csv().collect();
        assert_eq!(lines[1], "1,1.0000,0.0000,1.0000,false");
    }

    #[test]
    fn test_dispute_and_chargeback() {
        let mut engine = TransactionsEngine::new(Store::memory());
        let items = vec![
            CsvItem::Deposit(1, 1, 20000),
            CsvItem::Dispute(1, 1),
            CsvItem::ChargerBack(1, 1),
        ];
        engine.process(items.into_iter());

        let lines: Vec<String> = engine.as_csv().collect();
        assert_eq!(lines[1], "1,0.0000,0.0000,0.0000,true");

        // Subsequent deposit should be ignored because locked is true
        engine.process(vec![CsvItem::Deposit(1, 2, 50000)].into_iter());
        let lines: Vec<String> = engine.as_csv().collect();
        assert_eq!(lines[1], "1,0.0000,0.0000,0.0000,true");
    }
}