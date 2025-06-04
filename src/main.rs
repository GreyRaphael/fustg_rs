use crossbeam_channel::bounded;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::{mem, thread};
use threadpool::ThreadPool;
use zmq;

// -----------------------------------------------------------------------------
// Alias for a 16‐byte, C‐style string (char[16])
// -----------------------------------------------------------------------------
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolType(pub [u8; 16]);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StrategyType(pub [u8; 32]);
// pub type StrategyType = [u8; 16]; // ← assumed same as SymbolType

impl SymbolType {
    /// Interpret the bytes as a (possibly NUL-terminated) UTF-8 string.
    pub fn as_str(&self) -> &str {
        // Find the first 0 byte (or use full length if none).
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());
        // SAFELY turn bytes[0..len] into &str (or yield empty on invalid UTF-8).
        std::str::from_utf8(&self.0[..len]).unwrap_or("")
    }
}

impl fmt::Debug for SymbolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl StrategyType {
    /// Interpret the bytes as a (possibly NUL-terminated) UTF-8 string.
    pub fn as_str(&self) -> &str {
        // Find the first 0 byte (or use full length if none).
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());
        // SAFELY turn bytes[0..len] into &str (or yield empty on invalid UTF-8).
        std::str::from_utf8(&self.0[..len]).unwrap_or("")
    }
}

impl fmt::Debug for StrategyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// -----------------------------------------------------------------------------
// TickData: exactly matches the C struct layout
// -----------------------------------------------------------------------------
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

// -----------------------------------------------------------------------------
// C “enum class DirectionType : uint8_t { NONE, BUY, SELL };”
// -----------------------------------------------------------------------------
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectionType {
    NONE = 0,
    BUY = 1,
    SELL = 2,
}

// -----------------------------------------------------------------------------
// C “enum class OffsetFlagType : uint8_t { NONE, OPEN, CLOSE };”
// -----------------------------------------------------------------------------
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OffsetFlagType {
    NONE = 0,
    OPEN = 1,
    CLOSE = 2,
}

// -----------------------------------------------------------------------------
// Order: matches the C struct exactly, assuming StrategyType is [c_char; 16]
// -----------------------------------------------------------------------------
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Order {
    pub stg_name: StrategyType,   // StrategyType stg_name;
    pub symbol: SymbolType,       // SymbolType symbol;
    pub timestamp: i64,           // int64_t timestamp;
    pub volume: u32,              // uint32_t volume;
    pub direction: DirectionType, // DirectionType direction;
    pub offset: OffsetFlagType,   // OffsetFlagType offset;
}

fn str_to_array16(s: &str) -> [u8; 16] {
    let mut buf = [0u8; 16];
    let bytes = s.as_bytes();
    let n = bytes.len().min(16);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

fn str_to_array32(s: &str) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = s.as_bytes();
    let n = bytes.len().min(32);
    buf[..n].copy_from_slice(&bytes[..n]);
    buf
}

trait Strategy: Send + Sync {
    fn name(&self) -> String;
    fn update(&self, tick: &TickData) -> Order;
}

struct Aberration {
    ma_len: u32,
}

impl Aberration {
    fn new(ma_len: u32) -> Self {
        Self { ma_len }
    }
}

impl Strategy for Aberration {
    fn name(&self) -> String {
        format!("Aberration{}", self.ma_len)
    }
    fn update(&self, tick: &TickData) -> Order {
        // do some strategy to generate order
        Order {
            stg_name: StrategyType(str_to_array32(&self.name())),
            symbol: tick.symbol,
            timestamp: tick.stamp,
            volume: 1,
            direction: DirectionType::BUY,
            offset: OffsetFlagType::OPEN,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = zmq::Context::new();
    let subscriber = ctx.socket(zmq::SUB)?;
    subscriber.set_rcvhwm(0)?;
    subscriber.set_subscribe(b"")?;
    subscriber.connect("ipc://@hq")?;

    // let (tx, rx) = mpsc::channel::<Order>();
    let (tx, rx) = bounded::<Order>(1024);

    {
        let ctx_clone = ctx.clone();
        thread::spawn(move || {
            let pusher = ctx_clone.socket(zmq::PUSH).expect("failed to create PUSH");
            pusher.connect("ipc://@orders").expect("failed to bind pusher");
            while let Ok(order) = rx.recv() {
                println!("send: {:?}", &order);
                let bytes: &[u8] = unsafe {
                    let ptr = &order as *const Order as *const u8;
                    std::slice::from_raw_parts(ptr, mem::size_of::<Order>())
                };
                // println!("length of orde is {}", bytes.len());
                if let Err(e) = pusher.send(bytes, 0) {
                    eprintln!("Error sending on PUSH socket: {:?}", e);
                    // In a real service, you might retry or back off here.
                }
            }
        });
    }

    let pool = ThreadPool::new(4);

    let mut stg_map: HashMap<SymbolType, Vec<Box<dyn Strategy>>> = HashMap::new();
    let sym = SymbolType(str_to_array16("rb2505"));

    let mut strategies: Vec<Box<dyn Strategy>> = Vec::new();
    strategies.push(Box::new(Aberration::new(100)));
    strategies.push(Box::new(Aberration::new(200)));

    stg_map.insert(sym, strategies);
    let stg_map = Arc::new(stg_map);

    loop {
        let mut msg = zmq::Message::new();
        subscriber.recv(&mut msg, 0)?;
        let buf = msg.as_ref();
        let tick: TickData = unsafe {
            let ptr = buf.as_ptr() as *const TickData;
            std::ptr::read_unaligned(ptr)
        };
        let tx_clone = tx.clone();
        let stg_map_clone = Arc::clone(&stg_map);
        pool.execute(move || {
            if let Some(strategies) = stg_map_clone.get(&tick.symbol) {
                for stg in strategies.iter() {
                    let order = stg.update(&tick);
                    if let Err(e) = tx_clone.send(order) {
                        eprintln!("Failed to send order: {:?} of {}", e, stg.name());
                    }
                }
            }
            // else {
            //     eprintln!("No strategy found for symbol {:?}", tick.symbol);
            // }
        });
    }
}
