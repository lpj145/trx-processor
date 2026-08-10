use std::io::Read;
use serde::Deserialize;
use crate::types::CsvItem;

#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "type")]
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<String>,
}

pub fn parse_csv<S: Read>(stream: S) -> impl Iterator<Item = CsvItem> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(stream);

    let mut items = Vec::new();

    for result in reader.deserialize::<Record>() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let tx_type = record.tx_type.to_lowercase();
        match tx_type.as_str() {
            "deposit" => {
                if let Some(amt_str) = record.amount {
                    if let Some(amt) = parse_amount(&amt_str) {
                        items.push(CsvItem::Deposit(record.client, record.tx, amt));
                    }
                }
            }
            "withdrawal" => {
                if let Some(amt_str) = record.amount {
                    if let Some(amt) = parse_amount(&amt_str) {
                        items.push(CsvItem::Withdrawal(record.client, record.tx, amt));
                    }
                }
            }
            "dispute" => {
                items.push(CsvItem::Dispute(record.client, record.tx));
            }
            "resolve" => {
                items.push(CsvItem::Resolve(record.client, record.tx));
            }
            "chargerback" | "chargeback" => {
                items.push(CsvItem::ChargerBack(record.client, record.tx));
            }
            _ => {}
        }
    }

    items.into_iter()
}

fn parse_amount(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split('.');
    let int_str = parts.next()?;
    let int_val: u64 = int_str.parse().ok()?;
    if let Some(frac_str) = parts.next() {
        let mut frac_str = frac_str.to_string();
        if frac_str.len() > 4 {
            frac_str.truncate(4);
        } else {
            while frac_str.len() < 4 {
                frac_str.push('0');
            }
        }
        let frac_val: u64 = frac_str.parse().ok()?;
        Some(int_val * 10000 + frac_val)
    } else {
        Some(int_val * 10000)
    }
}
