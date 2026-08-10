use std::io::Read;
use aho_corasick::AhoCorasick;
use crate::types::CsvItem;

const BUF_SIZE: usize = 32 * 1024; // 32KB buffer

pub fn parse_corasick<S: Read>(mut stream: S) -> impl Iterator<Item = CsvItem> {
    let patterns = &["de", "wi", "di", "re", "ch"];
    let ac = AhoCorasick::new(patterns).expect("Unable to build AhoCorasick");

    let mut items = Vec::new();
    let mut buff = vec![0u8; BUF_SIZE];
    let mut buf_len = 0;

    loop {
        let free_space = BUF_SIZE - buf_len;
        if free_space == 0 {
            // Buffer is completely full without any newline, reset or break
            break;
        }

        let n = match stream.read(&mut buff[buf_len..]) {
            Ok(0) => 0,
            Ok(n) => n,
            Err(_) => break,
        };

        if n == 0 && buf_len == 0 {
            break;
        }

        buf_len += n;

        // Process up to the last newline in the buffer
        let process_len = if n == 0 {
            buf_len
        } else if let Some(pos) = buff[..buf_len].iter().rposition(|&b| b == b'\n') {
            pos + 1
        } else {
            // If no newline is found, read more data into the second half of the buffer
            continue;
        };

        for mat in ac.find_iter(&buff[..process_len]) {
            let start = mat.start();
            // Ensure pattern starts at beginning of a line
            if start != 0 && buff[start - 1] != b'\n' {
                continue;
            }

            let pattern_id = mat.pattern().as_usize();

            let line_end = match buff[start..process_len].iter().position(|&b| b == b'\n') {
                Some(pos) => start + pos,
                None => {
                    if n == 0 {
                        process_len
                    } else {
                        continue;
                    }
                }
            };

            let line_bytes = &buff[start..line_end];
            if let Some(item) = parse_line_bytes(line_bytes, pattern_id) {
                items.push(item);
            }
        }

        if n == 0 {
            break;
        }

        let remaining = buf_len - process_len;
        if remaining > 0 {
            buff.copy_within(process_len..buf_len, 0);
        }
        buf_len = remaining;
    }

    items.into_iter()
}

fn parse_line_bytes(bytes: &[u8], pattern_id: usize) -> Option<CsvItem> {
    let mut comma_iter = bytes.split(|&b| b == b',');

    let _type_part = comma_iter.next()?;
    let client_part = comma_iter.next()?;
    let client_id = parse_u16(client_part)?;

    let tx_part = comma_iter.next()?;
    let trx_id = parse_u32(tx_part)?;

    match pattern_id {
        0 => { // deposit ("de")
            let amount_part = comma_iter.next()?;
            let amount = parse_amount_bytes(amount_part)?;
            Some(CsvItem::Deposit(client_id, trx_id, amount))
        }
        1 => { // withdrawal ("wi")
            let amount_part = comma_iter.next()?;
            let amount = parse_amount_bytes(amount_part)?;
            Some(CsvItem::Withdrawal(client_id, trx_id, amount))
        }
        2 => { // dispute ("di")
            Some(CsvItem::Dispute(client_id, trx_id))
        }
        3 => { // resolve ("re")
            Some(CsvItem::Resolve(client_id, trx_id))
        }
        4 => { // chargerback ("ch")
            Some(CsvItem::ChargerBack(client_id, trx_id))
        }
        _ => None,
    }
}

fn parse_u16(mut bytes: &[u8]) -> Option<u16> {
    while let Some((&b, rest)) = bytes.split_first() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    while let Some((&b, rest)) = bytes.split_last() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    if bytes.is_empty() { return None; }

    let mut val: u16 = 0;
    for &b in bytes {
        if b >= b'0' && b <= b'9' {
            val = val.checked_mul(10)?.checked_add((b - b'0') as u16)?;
        } else {
            return None;
        }
    }
    Some(val)
}

fn parse_u32(mut bytes: &[u8]) -> Option<u32> {
    while let Some((&b, rest)) = bytes.split_first() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    while let Some((&b, rest)) = bytes.split_last() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    if bytes.is_empty() { return None; }

    let mut val: u32 = 0;
    for &b in bytes {
        if b >= b'0' && b <= b'9' {
            val = val.checked_mul(10)?.checked_add((b - b'0') as u32)?;
        } else {
            return None;
        }
    }
    Some(val)
}

fn parse_amount_bytes(mut bytes: &[u8]) -> Option<u64> {
    while let Some((&b, rest)) = bytes.split_first() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    while let Some((&b, rest)) = bytes.split_last() {
        if b == b' ' || b == b'\t' || b == b'\r' { bytes = rest; } else { break; }
    }
    if bytes.is_empty() { return None; }

    let mut parts = bytes.splitn(2, |&b| b == b'.');
    let int_part = parts.next()?;
    let mut int_val: u64 = 0;
    for &b in int_part {
        if b >= b'0' && b <= b'9' {
            int_val = int_val.checked_mul(10)?.checked_add((b - b'0') as u64)?;
        } else {
            return None;
        }
    }

    if let Some(frac_part) = parts.next() {
        let mut frac_val: u64 = 0;
        let mut digits = 0;
        for &b in frac_part {
            if b >= b'0' && b <= b'9' {
                if digits < 4 {
                    frac_val = frac_val * 10 + (b - b'0') as u64;
                    digits += 1;
                }
            } else {
                break;
            }
        }
        while digits < 4 {
            frac_val *= 10;
            digits += 1;
        }
        Some(int_val * 10000 + frac_val)
    } else {
        Some(int_val * 10000)
    }
}
