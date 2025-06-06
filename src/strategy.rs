use crate::types::{NameType, Order, TickData};

/// The Strategy trait. Every strategy must implement `name()` and `update(&TickData)` → `Order`.
pub trait Strategy: Send + Sync {
    /// Return the strategy’s name (as a NameType).
    fn name(&self) -> NameType;

    /// Given a TickData, produce a new Order.
    fn update(&mut self, tick: &TickData) -> Order;
    // fn update(&mut self, tick: &TickData);
    // fn init_broker(&mut self, ctx: &zmq::Context, order_uri: &str, commission_fee: f64, margin_ratio: f64);
}
