use std::io::Read;

use crate::types::CsvItem;

pub mod corasick;
pub mod csv;

pub enum Parser {
    Corasick,
    Csv,
}

impl Parser {
    pub fn from_args() -> Self {
        let args = std::env::args().collect::<Vec<_>>();
        let arg = args.iter().find(|arg| *arg == "--parse=corasick");

        if arg.is_some() {
            Self::Corasick
        } else {
            Self::Csv
        }
    }

    pub fn parse<S: Read + 'static>(self, stream: S) -> Box<dyn Iterator<Item = CsvItem>> {
        match self {
            Self::Csv => Box::new(csv::parse_csv(stream)),
            Self::Corasick => Box::new(corasick::parse_corasick(stream)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "\
type,client,tx,amount
deposit,1,1,1.0000
withdrawal,1,2,0.5000
dispute,1,1,
resolve,1,1,
chargerback,1,1,
";

    #[test]
    fn test_csv_parser() {
        let items: Vec<CsvItem> = Parser::Csv.parse(SAMPLE_CSV.as_bytes()).collect();
        assert_eq!(items.len(), 5);
        match &items[0] {
            CsvItem::Deposit(c, t, a) => {
                assert_eq!(*c, 1);
                assert_eq!(*t, 1);
                assert_eq!(*a, 10000);
            }
            _ => panic!("Expected deposit"),
        }
        match &items[1] {
            CsvItem::Withdrawal(c, t, a) => {
                assert_eq!(*c, 1);
                assert_eq!(*t, 2);
                assert_eq!(*a, 5000);
            }
            _ => panic!("Expected withdrawal"),
        }
    }

    #[test]
    fn test_corasick_parser() {
        let items: Vec<CsvItem> = Parser::Corasick.parse(SAMPLE_CSV.as_bytes()).collect();
        assert_eq!(items.len(), 5);
        match &items[0] {
            CsvItem::Deposit(c, t, a) => {
                assert_eq!(*c, 1);
                assert_eq!(*t, 1);
                assert_eq!(*a, 10000);
            }
            _ => panic!("Expected deposit"),
        }
    }
}
