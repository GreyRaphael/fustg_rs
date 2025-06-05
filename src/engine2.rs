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

use crate::strategy::Strategy;
use crate::types::{Order, SymbolType, TickData};

pub struct Engine {
    quote_addr: String,
    order_addr: String,
    stg_map: Arc<HashMap<SymbolType, Vec<Box<dyn Strategy>>>>,
    running: AtomicBool,
}

impl Engine {
    pub fn new(quote_addr: &str, order_addr: &str) -> Self {
        Engine {
            quote_addr,
            order_addr,
            stg_map: Arc::new(HashMap::new()),
            running: AtomicBool::new(true),
        }
    }

    pub fn add_strategy(&self, symbol: SymbolType, strategy: Box<dyn Strategy>) {
        stg_map.entry(symbol).or_insert_with(Vec::new).push(strategy);
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {}

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}
