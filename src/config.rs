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

/// Read `path` and parse it in one go.
pub fn load_fees<P: AsRef<Path>>(path: P) -> Result<HashMap<String, InstrumentFee>> {
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
            &InstrumentFee { open: 100.0, close: 102.5, closetoday: 101.5, byvolume: true }
        );
    }
}
