use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize, PartialEq)]
struct InstrumentFee {
    open: f64,
    close: f64,
    closetoday: f64,
    byvolume: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
struct ContractInfo {
    contract_multiplier: f64,
    min_move: f64,
    // open fee
    open_fee_rate: f64,
    open_fee_fixed: f64,
    // close fee
    close_fee_rate: f64,
    close_fee_fixed: f64,
    // close today fee
    close_today_fee_rate: f64,
    close_today_fee_fixed: f64,
    // long margin
    long_margin_rate: f64,
    long_margin_fixed: f64,
    // short margin
    short_margin_rate: f64,
    short_margin_fixed: f64,
}

/// Read `path` and parse it in one go.
pub fn load_fees<P: AsRef<Path>>(path: P) -> Result<HashMap<String, ContractInfo>> {
    // `?` will automatically convert both io::Error and toml::de::Error
    let s = fs::read_to_string(path)?;
    let map = toml::from_str(&s)?;
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TOML: &str = r#"
    ['CFFEX.IF']
    open       = 100.0
    close      = 102.5
    closetoday = 101.5
    byvolume   = true

    ['CZSE.IC']
    open       = 200.0
    close      = 198.75
    closetoday = 199.0
    byvolume   = false
    "#;

    #[test]
    fn it_parses_multiple_instruments() {
        // bypass the file‐read and call toml::from_str directly
        let map: HashMap<String, InstrumentFee> = toml::from_str(TEST_TOML).expect("parsing should succeed");

        println!("{:?}", &map);
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("CFFEX.IF").unwrap(),
            &InstrumentFee {
                open: 100.0,
                close: 102.5,
                closetoday: 101.5,
                byvolume: true
            }
        );
    }

    #[test]
    fn parse_file() {
        // test use crate root as working directory
        println!("Current dir: {:?}", std::env::current_dir().unwrap());
        let map = load_fees("config/fees.2nd.toml").expect("parse should succeed");
        assert_eq!(map.len(), 83);
        // println!("{:?}", map);
    }
}
