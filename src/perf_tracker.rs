use crate::types::{Order, TickData};

pub struct PerformanceTracker {
    init_cash: f64,
    fee: f64,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, fee: f64) -> Self {
        Self { init_cash, fee }
    }

    pub fn on_fill(&mut self, order: &Order) {}

    pub fn on_tick_end(&mut self, tick: &TickData) {}
}
