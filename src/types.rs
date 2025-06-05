use std::array;
use std::fmt;

// Alias for a 16-byte, C-style string (char[16])
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolType(pub [u8; 16]);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct NameType(pub [u8; 32]);

impl SymbolType {
    /// Interpret the bytes as a (possibly NUL-terminated) UTF-8 string.
    pub fn as_str(&self) -> &str {
        // Find the first 0 byte (or use full length if none).
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());
        std::str::from_utf8(&self.0[..len]).unwrap_or("")
    }
}

impl fmt::Debug for SymbolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for SymbolType {
    fn from(s: &str) -> Self {
        let bytes = s.as_bytes();
        let arr = array::from_fn(|i: usize| if i < bytes.len() && i < 16 { bytes[i] } else { 0 });
        SymbolType(arr)
    }
}

impl NameType {
    /// Interpret the bytes as a (possibly NUL-terminated) UTF-8 string.
    pub fn as_str(&self) -> &str {
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());
        std::str::from_utf8(&self.0[..len]).unwrap_or("")
    }
}

impl fmt::Debug for NameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for NameType {
    fn from(s: &str) -> Self {
        let bytes = s.as_bytes();
        let arr = array::from_fn(|i: usize| if i < bytes.len() && i < 32 { bytes[i] } else { 0 });
        NameType(arr)
    }
}

/// TickData: exactly matches the C struct layout
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TickData {
    pub symbol: SymbolType, // char symbol[16]
    pub stamp: i64,         // int64_t stamp
    pub open: f64,          // double open
    pub high: f64,          // double high
    pub low: f64,           // double low
    pub last: f64,          // double last
    pub limit_down: f64,    // double limit_down
    pub limit_up: f64,      // double limit_up
    pub preclose: f64,      // double preclose
    pub close: f64,         // double close
    pub presettle: f64,     // double presettle
    pub settle: f64,        // double settle
    pub preoi: f64,         // double preoi
    pub oi: f64,            // double oi
    pub volume: i64,        // int64_t volume
    pub amount: f64,        // double amount
    pub avgprice: f64,      // double avgprice
    pub ap1: f64,           // double ap1
    pub ap2: f64,           // double ap2
    pub ap3: f64,           // double ap3
    pub ap4: f64,           // double ap4
    pub ap5: f64,           // double ap5
    pub bp1: f64,           // double bp1
    pub bp2: f64,           // double bp2
    pub bp3: f64,           // double bp3
    pub bp4: f64,           // double bp4
    pub bp5: f64,           // double bp5
    pub av1: i32,           // int32_t av1
    pub av2: i32,           // int32_t av2
    pub av3: i32,           // int32_t av3
    pub av4: i32,           // int32_t av4
    pub av5: i32,           // int32_t av5
    pub bv1: i32,           // int32_t bv1
    pub bv2: i32,           // int32_t bv2
    pub bv3: i32,           // int32_t bv3
    pub bv4: i32,           // int32_t bv4
    pub bv5: i32,           // int32_t bv5
    pub adj: f64,           // double adj
}

/// C “enum class DirectionType : uint8_t { NONE, BUY, SELL };”
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectionType {
    NONE = 0,
    BUY = 1,
    SELL = 2,
}

/// C “enum class OffsetFlagType : uint8_t { NONE, OPEN, CLOSE };”
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OffsetFlagType {
    NONE = 0,
    OPEN = 1,
    CLOSE = 2,
}

/// Order: matches the C struct exactly, assuming NameType is char[32]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Order {
    pub stg_name: NameType,       // NameType stg_name;
    pub symbol: SymbolType,       // SymbolType symbol;
    pub timestamp: i64,           // int64_t timestamp;
    pub volume: u32,              // uint32_t volume;
    pub direction: DirectionType, // DirectionType direction;
    pub offset: OffsetFlagType,   // OffsetFlagType offset;
}
