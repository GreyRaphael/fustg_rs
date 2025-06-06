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
