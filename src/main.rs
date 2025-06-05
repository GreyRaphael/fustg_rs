use ctrlc;
use std::array;
use std::collections::HashMap;
use std::fmt;
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
    fn update(&self, tick: &TickData) -> Order;
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
    fn update(&self, tick: &TickData) -> Order {
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

static RUNNING: AtomicBool = AtomicBool::new(true);

fn main1() -> Result<(), Box<dyn std::error::Error>> {
    // 1) Install a Ctrl-C handler that flips RUNNING to false
    ctrlc::set_handler(move || {
        // This closure is run in a signal‐handler context; it should be lightweight.
        eprintln!("Caught SIGINT! Shutting down …");
        RUNNING.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // 2) Create ZMQ context + SUB socket
    let ctx = zmq::Context::new();
    let subscriber = ctx.socket(zmq::SUB)?;
    subscriber.set_rcvhwm(0)?;
    subscriber.set_subscribe(b"")?;
    subscriber.connect("ipc://@hq")?;

    // 3) Create MPSC channel for Orders
    let (tx, rx) = mpsc::channel::<Order>();

    // 4) Spawn a PUSH‐socket thread that drains rx.recv()
    let pusher = ctx.socket(zmq::PUSH)?;
    pusher.set_sndhwm(0)?;
    pusher.set_linger(0)?; // *key*: do not block on close even if the orders not hand-off
    pusher.connect("ipc://@orders")?;
    let push_handle = thread::spawn(move || {
        // Loop until `rx.recv()` errors (i.e., channel closed)
        while let Ok(order) = rx.recv() {
            println!("send: {:?}", &order);
            let bytes: &[u8] = unsafe {
                let ptr = &order as *const Order as *const u8;
                std::slice::from_raw_parts(ptr, mem::size_of::<Order>())
            };
            // println!("length of orde is {}", bytes.len());
            // send won't block as the sndhwm is unlimited
            if let Err(e) = pusher.send(bytes, 0) {
                eprintln!("Error sending on PUSH socket: {:?}", e);
                // In a real service, you might retry or back off here.
            }
        }

        // Once `rx` is closed, we drop out here.
        eprintln!("Push thread: channel closed, exiting.");
    });

    // 5) Build a ThreadPool + strategy‐map (Arc)
    let pool = ThreadPool::new(4);

    let mut stg_map: HashMap<SymbolType, Vec<Box<dyn Strategy>>> = HashMap::new();

    let mut stg_group1: Vec<Box<dyn Strategy>> = Vec::new();
    stg_group1.push(Box::new(Aberration::new(100)));
    stg_group1.push(Box::new(Aberration::new(200)));
    let mut stg_group2: Vec<Box<dyn Strategy>> = Vec::new();
    stg_group2.push(Box::new(Aberration::new(300)));
    stg_group2.push(Box::new(Aberration::new(500)));

    stg_map.insert(SymbolType::from("rb2505"), stg_group1);
    stg_map.insert(SymbolType::from("MA505"), stg_group2);

    let stg_map = Arc::new(stg_map);

    // 6) Reuse a receive buffer for TickData
    let mut buffer = [0u8; std::mem::size_of::<TickData>()];
    while RUNNING.load(Ordering::SeqCst) {
        // recv_into won't block as the rcvhwm is unlimited
        match subscriber.recv_into(&mut buffer, 0) {
            Ok(n) if n == buffer.len() => {
                let tick: TickData = unsafe {
                    let ptr = buffer.as_ptr() as *const TickData;
                    std::ptr::read_unaligned(ptr)
                };
                // println!("recv: {:?}", tick);
                let tx_clone = tx.clone();
                let stg_map_clone = Arc::clone(&stg_map);
                pool.execute(move || {
                    if let Some(strategies) = stg_map_clone.get(&tick.symbol) {
                        for stg in strategies.iter() {
                            let order = stg.update(&tick);
                            if let Err(e) = tx_clone.send(order) {
                                eprintln!("Failed to send order: {:?} of {:?}", e, stg.name());
                            }
                        }
                    }
                    // else {
                    //     eprintln!("No strategy found for symbol {:?}", tick.symbol);
                    // }
                });
            }
            // ZMQ returned something else (maybe the socket was closed, or real error).
            Err(e) => {
                eprintln!("SUB socket error (or closed): {:?}", e);
                break;
            }

            // If we got fewer/more bytes than `size_of::<TickData>()`
            Ok(n) => {
                eprintln!("Warning: received {} bytes (expected {}); ignoring", n, buffer.len());
                // Just keep going. If you prefer to stop, set RUNNING to false here.
            }
        }
    }

    // 8) We exit the loop => time to shut down
    eprintln!("Main loop detected RUNNING = false. Shutting down ...");
    // 8a) Drop the sending side of the channel so the push thread’s `rx.recv()` returns Err.
    drop(tx);
    // 8b) Wait for the threadpool to finish any inflight jobs.
    pool.join(); // This blocks until all submitted tasks have completed

    // 8c) Join the push‐socket thread
    if let Err(join_err) = push_handle.join() {
        eprintln!("Warning: push thread panicked or failed: {:?}", join_err);
    }

    eprintln!("Clean shutdown complete. Exiting.");

    Ok(())
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

pub struct Engine {
    ctx: zmq::Context,
    quote_socket: zmq::Socket,
    push_sockets: Vec<zmq::Socket>,
    handlers: Vec<thread::JoinHandle<()>>,
    num_workers: usize,
}

impl Engine {
    fn new(quote_url: &str, num_workers: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let ctx = zmq::Context::new();

        // setup quote socket
        let quote_socket = ctx.socket(zmq::SUB)?;
        quote_socket.set_rcvhwm(0)?;
        quote_socket.connect(quote_url)?;

        Ok(Engine {
            ctx,
            quote_socket,
            push_sockets: Vec::with_capacity(num_workers),
            handlers: Vec::with_capacity(num_workers),
            num_workers,
        })
    }

    pub fn add_strategy(&mut self, symbol: SymbolType, strategy: Box<dyn Strategy>) {
        self.quote_socket.set_subscribe(&symbol.0).expect(&format!("subscibe {:?}", symbol));
    }

    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for i in 0..self.num_workers {
            let endpoint = format!("inproc://worker-{}", i);
            let pusher = self.ctx.socket(zmq::PUSH)?;
            pusher.bind(&endpoint)?;
            self.push_sockets.push(pusher);

            let ctx_clone = self.ctx.clone();
            let handler = thread::spawn(move || {
                let order_pusher = ctx_clone.socket(zmq::PUSH).expect("failed to create");
                order_pusher.connect("ipc://@orders").expect("failed to connect");
                let puller = ctx_clone.socket(zmq::PULL).expect("failed to create");
                puller.connect(&endpoint).expect("failed to connect");
                let mut buffer = [0u8; std::mem::size_of::<TickData>()];
                loop {
                    match puller.recv_into(&mut buffer, 0) {
                        Ok(n) if n == buffer.len() => {
                            // do something
                            let order = Order {
                                stg_name: NameType::from("Aberration"),
                                symbol: SymbolType::from("rb2505"),
                                timestamp: 1749141737,
                                volume: 1,
                                direction: DirectionType::BUY,
                                offset: OffsetFlagType::OPEN,
                            };
                            println!("send: {:?}", &order);
                            let bytes: &[u8] = unsafe {
                                let ptr = &order as *const Order as *const u8;
                                std::slice::from_raw_parts(ptr, mem::size_of::<Order>())
                            };
                            // println!("length of orde is {}", bytes.len());
                            // send won't block as the sndhwm is unlimited
                            if let Err(e) = order_pusher.send(bytes, 0) {
                                eprintln!("Error sending on PUSH socket: {:?}", e);
                                // In a real service, you might retry or back off here.
                            }
                        }
                        // ZMQ returned something else (maybe the socket was closed, or real error).
                        Err(e) => {
                            eprintln!("SUB socket error (or closed): {:?}", e);
                            break;
                        }
                        // If we got fewer/more bytes than `size_of::<TickData>()`
                        Ok(n) => {
                            eprintln!("Warning: received {} bytes (expected {}); ignoring", n, buffer.len());
                            // Just keep going. If you prefer to stop, set RUNNING to false here.
                        }
                    }
                }
            });
            self.handlers.push(handler);
        }

        Ok(())
    }

    pub fn start(&self) {
        let mut buffer = [0u8; std::mem::size_of::<TickData>()];
        loop {
            match self.quote_socket.recv_into(&mut buffer, 0) {
                Ok(n) if n == buffer.len() => {
                    let tick: TickData = unsafe {
                        let ptr = buffer.as_ptr() as *const TickData;
                        std::ptr::read_unaligned(ptr)
                    };
                    let worker_id = (tick.symbol.hash_future_symbol() as usize) % self.num_workers;
                    // send won't block as the sndhwm is unlimited
                    if let Err(e) = self.push_sockets[worker_id].send(buffer.as_slice(), 0) {
                        eprintln!("Error sending on PUSH socket: {:?}", e);
                        // In a real service, you might retry or back off here.
                    }
                }
                // ZMQ returned something else (maybe the socket was closed, or real error).
                Err(e) => {
                    eprintln!("SUB socket error (or closed): {:?}", e);
                    break;
                }

                // If we got fewer/more bytes than `size_of::<TickData>()`
                Ok(n) => {
                    eprintln!("Warning: received {} bytes (expected {}); ignoring", n, buffer.len());
                    // Just keep going. If you prefer to stop, set RUNNING to false here.
                }
            }
        }
    }
}

fn main() {}
