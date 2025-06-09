use crate::types::{Order, TickData};

pub struct PerformanceTracker {
    init_cash: f64,
    fee: f64,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, fee: f64) -> Self {
        Self { init_cash, fee }
    }

    fn charge(&mut self) -> f64 {
        1e-4
    }

    pub fn on_fill(&mut self, order: &Order) {}

    pub fn on_tick_end(&mut self, tick: &TickData) {}
}
