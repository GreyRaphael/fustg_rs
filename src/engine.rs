use crate::strategy::Strategy;
use crate::types::{Order, SymbolType, TickData};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::{mem, thread};
use zmq;

pub struct CtaEngine {
    num_workers: usize,
    senders: Vec<mpsc::Sender<TickData>>,
    handles: Vec<thread::JoinHandle<()>>,

    ctx: zmq::Context,
    /// We store the subscriber as an `Option` so that `stop()` can `.take()` and drop it,
    /// which causes the blocking `recv_into` to return an error.
    tick_subscriber: Option<zmq::Socket>,

    stg_map: HashMap<SymbolType, Vec<Box<dyn Strategy>>>,
    symbol_batches: Vec<HashSet<SymbolType>>,
    order_uri: String,
}

impl CtaEngine {
    pub fn new(tick_uri: &str, order_uri: &str, num_workers: usize) -> Self {
        let ctx = zmq::Context::new();
        let subscriber = ctx.socket(zmq::SUB).expect("Failed to create SUB socket");
        // unlimited RCVHWM, subscriber.recv_into won't block
        subscriber.set_rcvhwm(0).expect("Failed to set rcvhwm");
        // subscriber.set_rcvtimeo(10000).expect("Failed to set rcvtimo");
        subscriber.connect(tick_uri).expect("Failed to connect SUB socket to tick_uri");

        CtaEngine {
            num_workers,
            senders: Vec::with_capacity(num_workers),
            handles: Vec::with_capacity(num_workers),
            ctx,
            tick_subscriber: Some(subscriber),
            stg_map: HashMap::new(),
            symbol_batches: vec![HashSet::new(); num_workers],
            order_uri: order_uri.into(),
        }
    }

    /// Register a strategy for a given symbol.  We store it in stg_map as a
    /// Box<dyn Strategy>.  It will not be shared—only one worker thread gets it.
    pub fn add_strategy(&mut self, symbol: SymbolType, strategy: Box<dyn Strategy>) {
        // If it's the first time seeing `symbol`, subscribe
        if self.stg_map.get(&symbol).is_none() {
            if let Some(ref sock) = self.tick_subscriber {
                sock.set_subscribe(&symbol.0).expect(&format!("Failed to subscribe {:?}", symbol));
            }
        }
        // Push into stg_map (we’ll later drain each Vec into a worker).
        self.stg_map.entry(symbol).or_insert_with(Vec::new).push(strategy);

        // Figure out which worker “owns” this symbol (and all its strategies):
        let worker_id = (symbol.hash_future_symbol() as usize) % self.num_workers;
        self.symbol_batches[worker_id].insert(symbol);
    }

    /// Split `stg_map` into each worker’s “partial_map” and spawn the threads.
    pub fn init(&mut self) {
        for worker_id in 0..self.num_workers {
            // Build this worker’s partial_map from `symbol_batches[worker_id]`.
            let mut partial_map: HashMap<_, _> = self.symbol_batches[worker_id]
                .iter()
                .copied()
                .filter_map(|sym| self.stg_map.remove(&sym).map(|v| (sym, v)))
                .collect();

            let (tx, rx) = mpsc::channel::<TickData>();
            self.senders.push(tx);

            // Each worker gets its own ZMQ context for pushing orders:
            let ctx_clone = self.ctx.clone();
            let order_uri = self.order_uri.clone();

            let handle = thread::spawn(move || {
                let order_pusher = ctx_clone.socket(zmq::PUSH).expect("Failed to create PUSH socket");
                // unlimited SNDHWM, order_pusher.send won't block
                order_pusher.set_sndhwm(0).expect("Failed to set SNDHWM");
                order_pusher.set_linger(0).expect("Failed to set linger");
                order_pusher.connect(&order_uri).expect("Failed to connect PUSH to order_uri");

                for tick in rx {
                    if let Some(strategies) = partial_map.get_mut(&tick.symbol) {
                        for strat in strategies.iter_mut() {
                            let order = strat.update(&tick);
                            println!("[Worker {}] send: {:?}", worker_id, &order);

                            // Serialize the entire `Order` including any padding.
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

                println!("[Worker {}] Exiting thread.", worker_id);
            });

            self.handles.push(handle);
        }
    }

    /// Main loop: recv raw TickData bytes from `tick_subscriber`, deserialize, then hand off to workers.
    pub fn start(&self) {
        // We expect `tick_subscriber` to be `Some(_)` unless `stop()` has been called already.
        let subscriber = self.tick_subscriber.as_ref().expect("Subscriber socket missing in start()");

        let mut tick_buf = [0u8; std::mem::size_of::<TickData>()];
        loop {
            // recv_into listen on Ctrl-C, so it no need to add atomic running
            match subscriber.recv_into(&mut tick_buf, 0) {
                Ok(n) if n == tick_buf.len() => {
                    // SAFELY turn bytes into a TickData
                    let tick: TickData = unsafe {
                        let ptr = tick_buf.as_ptr() as *const TickData;
                        std::ptr::read_unaligned(ptr)
                    };
                    let worker_id = (tick.symbol.hash_future_symbol() as usize) % self.num_workers;
                    if let Err(e) = self.senders[worker_id].send(tick) {
                        eprintln!("Error sending tick to worker {}: {:?}", worker_id, e);
                    }
                }
                Ok(n) => {
                    eprintln!("Warning: received {} bytes (expected {}); ignoring", n, tick_buf.len());
                }
                Err(e) => {
                    // Likely the socket was dropped in stop(), so break
                    eprintln!("SUB socket error or closed: {:?}", e);
                    break;
                }
            }
        }
    }

    /// Gracefully stop: drop the SUB socket (unblocks recv), clear senders (unblocks worker rx loops), then join threads.
    pub fn stop(&mut self) {
        println!("stoping engine...");
        // 1) close subscriber
        if let Some(sub) = self.tick_subscriber.take() {
            drop(sub);
        }

        // 2) Drop all senders so that each worker’s `for tick in rx` ends
        self.senders.clear();

        // 3) Join all worker threads
        for handle in self.handles.drain(..) {
            handle.join().expect("Worker thread panicked");
        }

        println!("All worker threads have exited.");
    }
}
