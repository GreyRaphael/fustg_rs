use crate::operator::rolling;
use crate::strategy::Strategy;
use crate::types::{DirectionType, NameType, OffsetFlagType, Order, TickData};

pub struct Aberration {
    ma: rolling::Mean,
    stdev: rolling::StDev,
    name: NameType,
    position: i32,
}

impl Aberration {
    pub fn new(ma_len: usize) -> Self {
        let full_str = format!("Aberration{}", ma_len);
        Self {
            ma: rolling::Mean::new(ma_len),
            stdev: rolling::StDev::new(ma_len),
            name: NameType::from(full_str.as_str()),
            position: 0,
        }
    }
}

impl Strategy for Aberration {
    fn name(&self) -> NameType {
        // as NameType is Copy, so it will copy here
        // is NameType is only Clone, it will move
        self.name
    }

    fn update(&mut self, tick: &TickData) -> Option<Order> {
        let ma = self.ma.update(tick.last);
        let stdev = self.stdev.update(tick.last);

        // 先平仓, 再开仓
        if self.position > 0 {
            if tick.last < ma {
                self.position = 0;
                return Some(Order {
                    stg_name: self.name(),
                    symbol: tick.symbol,
                    timestamp: tick.stamp,
                    price: tick.bp1, // 买一价成交
                    lots: 1,
                    direction: DirectionType::SELL,
                    offset: OffsetFlagType::CLOSE,
                });
            }
        }

        if self.position < 0 {
            if tick.last > ma {
                self.position = 0;
                return Some(Order {
                    stg_name: self.name(),
                    symbol: tick.symbol,
                    timestamp: tick.stamp,
                    price: tick.ap1, // 卖一价成交
                    lots: 1,
                    direction: DirectionType::BUY,
                    offset: OffsetFlagType::CLOSE,
                });
            }
        }

        if self.position == 0 {
            if tick.last > ma + 2.0 * stdev {
                self.position = 1;
                return Some(Order {
                    stg_name: self.name(),
                    symbol: tick.symbol,
                    timestamp: tick.stamp,
                    price: tick.ap1, // 卖一价成交
                    lots: 1,
                    direction: DirectionType::BUY,
                    offset: OffsetFlagType::OPEN,
                });
            }

            if tick.last < ma - 2.0 * stdev {
                self.position = -1;
                return Some(Order {
                    stg_name: self.name(),
                    symbol: tick.symbol,
                    timestamp: tick.stamp,
                    price: tick.bp1, // 买一价成交
                    lots: 1,
                    direction: DirectionType::SELL,
                    offset: OffsetFlagType::OPEN,
                });
            }
        }

        return None;
        // do some strategy to generate order
    }
}
