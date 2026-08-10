use std::collections::BTreeMap;

pub type Key = [u8; 8];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreItem(pub Vec<u8>);

impl StoreItem {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<&[u8]> for StoreItem {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(StoreItem(value.to_vec()))
    }
}

impl From<StoreItem> for Vec<u8> {
    fn from(item: StoreItem) -> Self {
        item.0
    }
}

impl From<u64> for StoreItem {
    fn from(value: u64) -> Self {
        StoreItem(value.to_be_bytes().to_vec())
    }
}

impl From<StoreItem> for u64 {
    fn from(value: StoreItem) -> Self {
        if value.0.len() >= 8 {
            u64::from_be_bytes(value.0[0..8].try_into().unwrap_or([0; 8]))
        } else {
            0
        }
    }
}

pub enum Store {
    Memory(BTreeMap<Key, StoreItem>),
}

impl Store {
    pub fn memory() -> Self {
        Store::Memory(BTreeMap::new())
    }

    pub fn get<T: for<'a> TryFrom<&'a [u8]>>(&self, key: Key) -> Option<T> {
        match self {
            Store::Memory(btree_map) => {
                let item = btree_map.get(&key)?;
                T::try_from(item.as_bytes()).ok()
            }
        }
    }

    pub fn exists(&self, key: Key) -> bool {
        match self {
            Store::Memory(btree_map) => btree_map.contains_key(&key),
        }
    }

    pub fn put<T: Into<Vec<u8>>>(&mut self, key: Key, value: T) {
        match self {
            Store::Memory(btree_map) => {
                btree_map.insert(key, StoreItem(value.into()));
            }
        }
    }

    pub fn upsert<T, F>(&mut self, key: Key, default: T, f: F)
    where
        T: for<'a> TryFrom<&'a [u8]> + Into<Vec<u8>> + Clone,
        F: FnOnce(T) -> T,
    {
        match self {
            Store::Memory(btree_map) => match btree_map.entry(key) {
                std::collections::btree_map::Entry::Vacant(vacant) => {
                    let new_val = f(default);
                    vacant.insert(StoreItem(new_val.into()));
                }
                std::collections::btree_map::Entry::Occupied(mut occupied) => {
                    let current = T::try_from(occupied.get().as_bytes()).unwrap_or(default);
                    let new_val = f(current);
                    occupied.insert(StoreItem(new_val.into()));
                }
            },
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &StoreItem)> {
        match self {
            Store::Memory(btree_map) => btree_map.iter(),
        }
    }
}