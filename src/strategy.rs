use crate::types::{Order, TickData, NameType};

/// The Strategy trait. Every strategy must implement `name()` and `update(&TickData)` → `Order`.
pub trait Strategy: Send + Sync {
    /// Return the strategy’s name (as a NameType).
    fn name(&self) -> NameType;

    /// Given a TickData, produce a new Order.
    fn update(&self, tick: &TickData) -> Order;
}
