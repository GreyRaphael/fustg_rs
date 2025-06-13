use crate::{
    config::ContractInfo,
    types::{DirectionType, OffsetFlagType, Order, TickData},
};

pub struct PerformanceTracker {
    init_cash: f64,
    available_cash: f64,
    fronzen_cash: f64,
    market_value: f64,
    fee: f64,
    total_fee: f64,
    contract_info: ContractInfo,
    orders: Vec<Order>,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, fee: f64, contract_info: ContractInfo) -> Self {
        Self {
            init_cash,
            available_cash: init_cash,
            fronzen_cash: 0.0,
            market_value: init_cash,
            fee,
            total_fee: 0.0,
            contract_info,
            orders: Vec::new(),
        }
    }

    pub fn on_fill(&mut self, order: &Order, tick: &TickData) {
        match (order.direction, order.offset) {
            (DirectionType::BUY, OffsetFlagType::OPEN) => {
                // 以卖一价格为成交价格
                let value_per_lot = tick.ap1 * self.contract_info.multiplier;
                let fee =
                    self.contract_info.open_fee_rate * value_per_lot * (order.lots as f64) + self.contract_info.open_fee_fixed * (order.lots as f64);
                self.fronzen_cash = self.contract_info.long_margin_rate * value_per_lot * (order.lots as f64)
                    + self.contract_info.long_margin_fixed * (order.lots as f64);
                self.total_fee += fee;
                self.available_cash -= self.fronzen_cash + fee;
            }
            (DirectionType::SELL, OffsetFlagType::CLOSE) => {
                // 为了计算方便，以平昨的费率+买一价格计算
                let value_per_lot = tick.bp1 * self.contract_info.multiplier;
                let fee = self.contract_info.close_fee_rate * value_per_lot * (order.lots as f64)
                    + self.contract_info.close_fee_fixed * (order.lots as f64);
                self.fronzen_cash = 0.0;
                self.total_fee += fee;
            }
            (DirectionType::SELL, OffsetFlagType::OPEN) => {
                // 以买一价格为成交价格
                let value_per_lot = tick.bp1 * self.contract_info.multiplier;
                let fee =
                    self.contract_info.open_fee_rate * value_per_lot * (order.lots as f64) + self.contract_info.open_fee_fixed * (order.lots as f64);
                self.fronzen_cash = self.contract_info.short_margin_rate * value_per_lot * (order.lots as f64)
                    + self.contract_info.short_margin_fixed * (order.lots as f64);
                self.total_fee += fee;
                self.available_cash -= self.fronzen_cash + fee;
            }
            (DirectionType::BUY, OffsetFlagType::CLOSE) => {
                // 为了计算方便，以平昨的费率+卖一计算
                let value_per_lot = tick.ap1 * self.contract_info.multiplier;
                let fee = self.contract_info.close_fee_rate * value_per_lot * (order.lots as f64)
                    + self.contract_info.close_fee_fixed * (order.lots as f64);
                self.fronzen_cash = 0.0;
                self.total_fee += fee;
            }
        };
        // self.cash -= fee;
        self.orders.push(order.clone());
    }

    pub fn on_tick_end(&mut self, tick: &TickData) {}
}
