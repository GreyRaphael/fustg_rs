// src/strategies/aberration.rs
use crate::strategy::Strategy;
use crate::types::{DirectionType, NameType, OffsetFlagType, Order, TickData};

pub struct Aberration {
    ma_len: u32,
    name: NameType,
}

impl Aberration {
    pub fn new(ma_len: u32) -> Self {
        let full_str = format!("Aberration{}", ma_len);
        Self {
            ma_len,
            name: NameType::from(full_str.as_str()),
        }
    }
}

impl Strategy for Aberration {
    fn name(&self) -> NameType {
        // NameType is Copy, so this just copies the array.
        self.name
    }

    fn update(&self, tick: &TickData) -> Order {
        // In a real implementation, you’d compute your MA‐based logic here.
        // For now, we simply buy 1 lot as an example.
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
