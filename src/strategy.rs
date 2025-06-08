use crate::types::{NameType, Order, TickData};

/// The Strategy trait. Every strategy must implement `name()` and `update(&TickData)` → `Order`.
pub trait Strategy: Send {
    /// Return the strategy’s name (as a NameType).
    fn name(&self) -> NameType;

    /// Given a TickData, produce a new Order.
    fn update(&mut self, tick: &TickData) -> Option<Order>;
}
