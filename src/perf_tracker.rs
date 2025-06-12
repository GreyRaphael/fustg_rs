use crate::{
    config::ContractInfo,
    types::{DirectionType, OffsetFlagType, Order, TickData},
};

pub struct PerformanceTracker {
    init_cash: f64,
    cash: f64,
    fee: f64,
    contract_info: ContractInfo,
    orders: Vec<Order>,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, fee: f64, contract_info: ContractInfo) -> Self {
        Self {
            init_cash,
            cash: init_cash,
            fee,
            contract_info,
            orders: Vec::new(),
        }
    }

    fn charge(&mut self) -> f64 {
        1e-4
    }

    pub fn on_fill(&mut self, order: &Order, tick: &TickData) {
        match (order.direction, order.offset) {
            (DirectionType::BUY, OffsetFlagType::OPEN) => {
                let fee = tick.ap1 * self.contract_info.multiplier * self.contract_info.open_fee_rate + self.contract_info.open_fee_fixed;
            }
            (DirectionType::SELL, OffsetFlagType::CLOSE) => {}
            (DirectionType::SELL, OffsetFlagType::OPEN) => {
                let fee = tick.bp1 * self.contract_info.multiplier * self.contract_info.open_fee_rate + self.contract_info.open_fee_fixed;
            }
            (DirectionType::BUY, OffsetFlagType::CLOSE) => {}
        }
        self.orders.push(order.clone());
    }

    pub fn on_tick_end(&mut self, tick: &TickData) {}
}
