pub type ClientId = u16;
pub type TrxId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxStatus {
    Normal = 0,
    Disputed = 1,
    ChargedBack = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionRecord {
    pub client_id: ClientId,
    pub tx_id: TrxId,
    pub amount: u64,
    pub status: TxStatus,
}

impl TransactionRecord {
    pub fn to_bytes(&self) -> [u8; 15] {
        let mut bytes = [0u8; 15];
        bytes[0..2].copy_from_slice(&self.client_id.to_be_bytes());
        bytes[2..6].copy_from_slice(&self.tx_id.to_be_bytes());
        bytes[6..14].copy_from_slice(&self.amount.to_be_bytes());
        bytes[14] = self.status as u8;
        bytes
    }
}

impl TryFrom<&[u8]> for TransactionRecord {
    type Error = ();

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 15 {
            return Err(());
        }
        let client_id = u16::from_be_bytes(bytes[0..2].try_into().map_err(|_| ())?);
        let tx_id = u32::from_be_bytes(bytes[2..6].try_into().map_err(|_| ())?);
        let amount = u64::from_be_bytes(bytes[6..14].try_into().map_err(|_| ())?);
        let status = match bytes[14] {
            0 => TxStatus::Normal,
            1 => TxStatus::Disputed,
            2 => TxStatus::ChargedBack,
            _ => return Err(()),
        };
        Ok(Self {
            client_id,
            tx_id,
            amount,
            status,
        })
    }
}

impl From<TransactionRecord> for Vec<u8> {
    fn from(rec: TransactionRecord) -> Self {
        rec.to_bytes().to_vec()
    }
}

impl From<&TransactionRecord> for Vec<u8> {
    fn from(rec: &TransactionRecord) -> Self {
        rec.to_bytes().to_vec()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Amount(pub u64);

impl TryFrom<&[u8]> for Amount {
    type Error = ();

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() < 8 {
            return Err(());
        }
        let val = u64::from_be_bytes(bytes[0..8].try_into().map_err(|_| ())?);
        Ok(Amount(val))
    }
}

impl From<Amount> for Vec<u8> {
    fn from(amt: Amount) -> Self {
        amt.0.to_be_bytes().to_vec()
    }
}

impl From<&Amount> for Vec<u8> {
    fn from(amt: &Amount) -> Self {
        amt.0.to_be_bytes().to_vec()
    }
}

impl From<u64> for Amount {
    fn from(v: u64) -> Self {
        Amount(v)
    }
}

impl From<Amount> for u64 {
    fn from(amt: Amount) -> Self {
        amt.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LockStatus(pub bool);

impl TryFrom<&[u8]> for LockStatus {
    type Error = ();

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.is_empty() {
            return Err(());
        }
        Ok(LockStatus(bytes[0] != 0))
    }
}

impl From<LockStatus> for Vec<u8> {
    fn from(lock: LockStatus) -> Self {
        vec![lock.0 as u8]
    }
}

impl From<&LockStatus> for Vec<u8> {
    fn from(lock: &LockStatus) -> Self {
        vec![lock.0 as u8]
    }
}

impl From<bool> for LockStatus {
    fn from(b: bool) -> Self {
        LockStatus(b)
    }
}

impl From<LockStatus> for bool {
    fn from(l: LockStatus) -> Self {
        l.0
    }
}

pub enum CsvItem {
    Deposit(ClientId, TrxId, u64),
    Withdrawal(ClientId, TrxId, u64),
    Dispute(ClientId, TrxId),
    Resolve(ClientId, TrxId),
    ChargerBack(ClientId, TrxId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_record_bytes_roundtrip() {
        let record = TransactionRecord {
            client_id: 42,
            tx_id: 1001,
            amount: 50000,
            status: TxStatus::Disputed,
        };

        let bytes = record.to_bytes();
        let parsed = TransactionRecord::try_from(&bytes[..]).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn test_amount_bytes_roundtrip() {
        let amount = Amount(123456789);
        let bytes: Vec<u8> = amount.into();
        let parsed = Amount::try_from(&bytes[..]).unwrap();
        assert_eq!(amount, parsed);
    }

    #[test]
    fn test_lock_status_bytes_roundtrip() {
        let lock = LockStatus(true);
        let bytes: Vec<u8> = lock.into();
        let parsed = LockStatus::try_from(&bytes[..]).unwrap();
        assert_eq!(lock, parsed);
    }
}