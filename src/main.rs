use ctrlc;
use std::array;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::{mem, thread};
use threadpool::ThreadPool;
use zmq;

// Alias for a 16‐byte, C‐style string (char[16])
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
        // SAFELY turn bytes[0..len] into &str (or yield empty on invalid UTF-8).
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
        // Find the first 0 byte (or use full length if none).
        let len = self.0.iter().position(|&b| b == 0).unwrap_or(self.0.len());
        // SAFELY turn bytes[0..len] into &str (or yield empty on invalid UTF-8).
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

// TickData: exactly matches the C struct layout
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

// C “enum class DirectionType : uint8_t { NONE, BUY, SELL };”
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DirectionType {
    NONE = 0,
    BUY = 1,
    SELL = 2,
}

// C “enum class OffsetFlagType : uint8_t { NONE, OPEN, CLOSE };”
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OffsetFlagType {
    NONE = 0,
    OPEN = 1,
    CLOSE = 2,
}

// Order: matches the C struct exactly, assuming NameType is char[32]
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

pub trait Strategy: Send + Sync {
    fn name(&self) -> NameType;
    fn update(&mut self, tick: &TickData) -> Order;
}

struct Aberration {
    ma_len: u32,
    name: NameType,
}

impl Aberration {
    fn new(ma_len: u32) -> Self {
        let full_str = format!("Aberration{}", ma_len);
        Self {
            ma_len,
            name: NameType::from(full_str.as_str()),
        }
    }
}

impl Strategy for Aberration {
    fn name(&self) -> NameType {
        // as NameType is Copy, so it will copy here
        // is NameType is only Clone, it will move
        self.name
    }
    fn update(&mut self, tick: &TickData) -> Order {
        self.ma_len += 1;
        // do some strategy to generate order
        Order {
            stg_name: self.name,
            symbol: tick.symbol,
            timestamp: tick.stamp,
            volume: 1,
            direction: DirectionType::BUY,
            offset: OffsetFlagType::OPEN,
        }
    }
}

impl SymbolType {
    /// 等价于 C++ 中的 `hashFutureSymbol`，将前两个字母打包成 u16。
    /// 如果第一个字符不是 A–Z/a–z，返回 0；
    /// 否则如果第二个字符也是字母，返回 (c0<<8 | c1)，否则返回 (c0<<8)。
    pub const fn hash_future_symbol(&self) -> u16 {
        let c0 = self.0[0];
        // 如果 c0 不是 A–Z 或 a–z，返回 0
        if !((c0 >= b'A' && c0 <= b'Z') || (c0 >= b'a' && c0 <= b'z')) {
            return 0;
        }
        let c1 = self.0[1];
        // 如果 c1 也是字母，就把 c0<<8 | c1
        if (c1 >= b'A' && c1 <= b'Z') || (c1 >= b'a' && c1 <= b'z') {
            return (c0 as u16) << 8 | (c1 as u16);
        }
        // 否则只返回 c0<<8
        (c0 as u16) << 8
    }
}

pub struct CtaEngine {
    num_workers: usize,
    senders: Vec<mpsc::Sender<TickData>>,
    handles: Vec<thread::JoinHandle<()>>,
    started: bool,
    ctx: zmq::Context,
    tick_subscriber: zmq::Socket,
    stg_map: HashMap<SymbolType, Vec<Box<dyn Strategy>>>,
    symbol_batches: Vec<Vec<SymbolType>>,
    order_uri: String,
}

impl CtaEngine {
    pub fn new(tick_uri: &str, order_uri: &str, num_workers: usize) -> Self {
        let ctx = zmq::Context::new();

        let tick_subscriber = ctx.socket(zmq::SUB).expect("msg");
        tick_subscriber.set_rcvhwm(0).expect("msg");
        tick_subscriber.connect(tick_uri).expect("msg");

        CtaEngine {
            num_workers,
            senders: Vec::with_capacity(num_workers),
            handles: Vec::with_capacity(num_workers),
            started: false,
            ctx,
            tick_subscriber,
            stg_map: HashMap::new(),
            symbol_batches: vec![Vec::new(); num_workers],
            order_uri: order_uri.into(),
        }
    }

    /// Register a strategy for a given symbol.  We store it in stg_map as a
    /// Box<dyn Strategy>.  It will not be shared—only one worker thread gets it.
    pub fn add_strategy(&mut self, symbol: SymbolType, strategy: Box<dyn Strategy>) {
        // Subscribe to exactly this symbol’s bytes on the ZMQ subscriber:
        self.tick_subscriber
            .set_subscribe(&symbol.0)
            .expect(&format!("failed to subscribe {:?}", symbol));

        // Push into stg_map (we’ll later drain each Vec into a worker).
        self.stg_map.entry(symbol).or_insert_with(Vec::new).push(strategy);

        // Figure out which worker “owns” this symbol (and all its strategies):
        let worker_id = (symbol.hash_future_symbol() as usize) % self.num_workers;
        self.symbol_batches[worker_id].push(symbol);
    }

    pub fn init(&mut self) {
        if self.started {
            eprintln!("Engine::init() called more than once without stop()");
            return;
        }
        self.started = true;

        for worker_id in 0..self.num_workers {
            let mut partial_map: HashMap<_, _> = self.symbol_batches[worker_id]
                .iter()
                .copied()
                .filter_map(|sym| self.stg_map.remove(&sym).map(|v| (sym, v)))
                .collect();

            let (tx, rx) = mpsc::channel::<TickData>();
            self.senders.push(tx);

            let ctx_clone = self.ctx.clone();
            let order_uri = self.order_uri.clone();
            let handle = thread::spawn(move || {
                // Each worker also has a PUSH for sending Orders out:
                let order_pusher = ctx_clone.socket(zmq::PUSH).expect("failed to create PUSH");
                order_pusher.connect(&order_uri).expect("failed to connect to orders");

                for tick in rx {
                    // Look up that symbol’s Vec<Box<dyn Strategy>>:
                    if let Some(strategies) = partial_map.get_mut(&tick.symbol) {
                        // For each Box<dyn Strategy>, we have &mut Box<…>,
                        // so we can call `update(&mut self)` directly.
                        for strat in strategies.iter_mut() {
                            let order = strat.update(&tick);
                            println!("send: {:?}", &order);

                            // Serialize the Order back to bytes:
                            let bytes: &[u8] = unsafe {
                                let ptr = &order as *const Order as *const u8;
                                std::slice::from_raw_parts(ptr, mem::size_of::<Order>())
                            };
                            if let Err(e) = order_pusher.send(bytes, 0) {
                                eprintln!("Error sending on PUSH socket: {:?}", e);
                            }
                        }
                    }
                }
                println!("[Worker {}] Channel closed, exiting.", worker_id);
            });

            self.handles.push(handle);
        }
    }

    /// The main thread’s loop: it simply pulls raw TickData from `tick_subscriber`
    /// and pushes each tick into the appropriate worker’s channel
    pub fn start(&self) {
        let mut tick_buf = [0u8; std::mem::size_of::<TickData>()];
        loop {
            match self.tick_subscriber.recv_into(&mut tick_buf, 0) {
                Ok(n) if n == tick_buf.len() => {
                    let tick: TickData = unsafe {
                        let ptr = tick_buf.as_ptr() as *const TickData;
                        std::ptr::read_unaligned(ptr)
                    };
                    let worker_id = (tick.symbol.hash_future_symbol() as usize) % self.num_workers;
                    if let Err(e) = self.senders[worker_id].send(tick) {
                        eprintln!("Error sending tick to worker {}: {:?}", worker_id, e);
                    }
                }
                Err(e) => {
                    eprintln!("SUB socket error (or closed): {:?}", e);
                    break;
                }
                Ok(n) => {
                    eprintln!("Warning: received {} bytes (expected {}); ignoring", n, tick_buf.len());
                }
            }
        }
    }

    /// Gracefully stop all workers. Drops all `Sender`s so each worker's receiver loop ends,
    /// then joins each thread. After calling `stop()`, you cannot `send()` any more items.
    /// Calling `stop()` twice is a no-op.
    pub fn stop(&mut self) {
        if !self.started {
            // If never started or already stopped, do nothing.
            return;
        }
        self.started = false;

        // Close the tick subscriber so that the start() loop exits.
        drop(self.tick_subscriber);

        // 1) Drop all senders. This closes each channel, so each `for item in rx` loop breaks.
        self.senders.clear();

        // 2) Join all worker threads.
        for handle in self.handles.drain(..) {
            handle.join().expect("Worker thread panicked");
        }

        println!("All worker threads have exited.");
    }
}

fn main() {
    let mut engine = CtaEngine::new("ipc://@hq", "ipc://@orders", 4);
    // let stg1=Aberration::new(100);
    engine.add_strategy(SymbolType::from("rb2505"), Box::new(Aberration::new(100)));
    engine.add_strategy(SymbolType::from("rb2505"), Box::new(Aberration::new(200)));
    engine.add_strategy(SymbolType::from("MA505"), Box::new(Aberration::new(300)));
    engine.add_strategy(SymbolType::from("MA505"), Box::new(Aberration::new(400)));
    engine.init();
    engine.start();
}
