use crate::{
    config::ContractInfo,
    types::{DirectionType, OffsetFlagType, Order, TickData},
};
use std::collections::HashMap;

/// 单向持仓
#[derive(Debug, Clone)]
struct Position {
    lots: u32,
    avg_price: f64,
    /// 已占用的保证金
    margin: f64,
}

impl Position {
    fn new(lots: u32, price: f64, margin_rate: f64, margin_fixed: f64, multiplier: f64) -> Self {
        let value_per_lot = price * multiplier;
        let margin = (margin_rate * value_per_lot + margin_fixed) * (lots as f64);

        Position {
            lots,
            avg_price: price,
            margin,
        }
    }

    /// 平仓时减少手数和释放保证金
    fn reduce(&mut self, lots_closed: u32, close_price: f64, margin_rate: f64, margin_fixed: f64, multiplier: f64) -> f64 {
        let closed_value_per_lot = close_price * multiplier;
        let release_margin = (margin_rate * closed_value_per_lot + margin_fixed) * (lots_closed as f64);

        self.lots -= lots_closed;
        self.margin = (margin_rate * self.avg_price * multiplier + margin_fixed) * (self.lots as f64);
        release_margin
    }

    /// 当前浮动盈亏
    fn unrealized_pnl(&self, last_price: f64, multiplier: f64, direction: DirectionType) -> f64 {
        let diff = match direction {
            DirectionType::BUY => last_price - self.avg_price,
            DirectionType::SELL => self.avg_price - last_price,
        };
        diff * multiplier * self.lots as f64
    }
}

pub struct PerformanceTracker {
    init_cash: f64,
    available_cash: f64,
    /// 所有方向的持仓
    positions: HashMap<DirectionType, Position>,
    contract_info: ContractInfo,

    /// 以下字段可以每次 on_tick_end 里重新计算
    frozen_cash: f64,
    market_value: f64,
    total_fee: f64,
    orders: Vec<Order>,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, contract_info: ContractInfo) -> Self {
        Self {
            init_cash,
            available_cash: init_cash,
            positions: HashMap::new(),
            contract_info,

            frozen_cash: 0.0,
            market_value: init_cash,
            total_fee: 0.0,
            orders: Vec::new(),
        }
    }

    pub fn on_fill(&mut self, order: &Order, tick: &TickData) {
        let price = match order.direction {
            // 以卖一价成交
            DirectionType::BUY => tick.ap1,
            // 以买一价成交
            DirectionType::SELL => tick.bp1,
        };

        // 1) 计算手续费
        let (fee_rate, fee_fixed) = match order.offset {
            OffsetFlagType::OPEN => (self.contract_info.open_fee_rate, self.contract_info.open_fee_fixed),
            // 为了计算方便，使用更高的平昨费率
            OffsetFlagType::CLOSE => (self.contract_info.close_fee_rate, self.contract_info.close_fee_fixed),
        };
        let value_per_lot = price * self.contract_info.multiplier;
        let fee = (fee_rate * value_per_lot + fee_fixed) * (order.lots as f64);
        self.total_fee += fee;
        self.available_cash -= fee;

        // 保证金率
        let (margin_rate, margin_fixed) = match order.direction {
            DirectionType::BUY => (self.contract_info.long_margin_rate, self.contract_info.long_margin_fixed),
            DirectionType::SELL => (self.contract_info.short_margin_rate, self.contract_info.short_margin_fixed),
        };

        // 2) 更新持仓和保证金
        match order.offset {
            OffsetFlagType::OPEN => {
                // 新增/累加持仓
                let pos = self
                    .positions
                    .entry(order.direction)
                    .or_insert_with(|| Position::new(0, price, margin_rate, margin_fixed, self.contract_info.multiplier));
                // 如果已有仓位，重新计算加权均价和保证金
                let total_lots = pos.lots + order.lots;
                let new_avg = (pos.avg_price * pos.lots as f64 + price * order.lots as f64) / total_lots as f64;
                pos.avg_price = new_avg;
                pos.lots += order.lots;

                pos.margin = (margin_rate * pos.avg_price * self.contract_info.multiplier + margin_fixed) * (pos.lots as f64);
                // 冻结保证金
                self.available_cash -= pos.margin - (self.frozen_cash); // 增量冻结
            }
            OffsetFlagType::CLOSE => {
                if let Some(pos) = self.positions.get_mut(&order.direction) {
                    // 释放对应保证金
                    let released_margin = pos.reduce(order.lots, price, margin_rate, margin_fixed, self.contract_info.multiplier);
                    self.available_cash += released_margin;
                    if pos.lots == 0 {
                        self.positions.remove(&order.direction);
                    }
                }
            }
        }

        self.orders.push(order.clone());
    }

    /// 每个 tick 结束后，重新计算浮动盈亏、市值和已冻保证金
    pub fn on_tick_end(&mut self, tick: &TickData) {
        let mut total_unreal = 0.0;
        let mut total_margin = 0.0;

        for (&direction, pos) in &self.positions {
            total_unreal += pos.unrealized_pnl(tick.last, self.contract_info.multiplier, direction);
            total_margin += pos.margin;
        }

        self.frozen_cash = total_margin;
        self.market_value = self.available_cash + total_unreal + total_margin;
    }
}
