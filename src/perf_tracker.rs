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

    pub fn on_fill(&mut self, order: &Order, tick: &TickData) {
        let fee = match (order.direction, order.offset) {
            (DirectionType::BUY, OffsetFlagType::OPEN) => {
                // 以卖一价格为成交价格
                tick.ap1 * self.contract_info.multiplier * self.contract_info.open_fee_rate * (tick.volume as f64) + self.contract_info.open_fee_fixed
            }
            (DirectionType::SELL, OffsetFlagType::CLOSE) => {
                // 为了计算方便，以平昨的费率+买一价格计算
                tick.bp1 * self.contract_info.multiplier * self.contract_info.close_fee_rate * (tick.volume as f64)
                    + self.contract_info.close_fee_fixed
            }
            (DirectionType::SELL, OffsetFlagType::OPEN) => {
                // 以买一价格为成交价格
                tick.bp1 * self.contract_info.multiplier * self.contract_info.open_fee_rate * (tick.volume as f64) + self.contract_info.open_fee_fixed
            }
            (DirectionType::BUY, OffsetFlagType::CLOSE) => {
                // 为了计算方便，以平昨的费率+卖一计算
                tick.ap1 * self.contract_info.multiplier * self.contract_info.close_fee_rate * (tick.volume as f64)
                    + self.contract_info.close_fee_fixed
            }
        };
        self.cash -= fee;
        self.orders.push(order.clone());
    }

    pub fn on_tick_end(&mut self, tick: &TickData) {}
}
