use crate::{
    config::ContractInfo,
    types::{DirectionType, OffsetFlagType, Order, TickData},
};

/// 单向持仓
#[derive(Debug, Clone, Copy)]
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

    /// 加仓，返回之前的保证金
    fn increase(&mut self, lots: u32, price: f64, margin_rate: f64, margin_fixed: f64, multiplier: f64) -> f64 {
        // recompute position
        let total_lots = self.lots + lots;
        self.avg_price = (self.avg_price * self.lots as f64 + price * lots as f64) / (total_lots as f64);
        self.lots = total_lots;

        // recompute margin
        let prev_margin = self.margin;
        self.margin = (margin_rate * self.avg_price * multiplier + margin_fixed) * (self.lots as f64);

        prev_margin
    }

    /// 减仓，返回释放的保证金
    fn decrease(&mut self, lots: u32, price: f64, margin_rate: f64, margin_fixed: f64, multiplier: f64) -> f64 {
        let closed_value_per_lot = price * multiplier;
        let released_margin = (margin_rate * closed_value_per_lot + margin_fixed) * (lots as f64);

        self.lots -= lots.min(self.lots);
        self.margin = (margin_rate * self.avg_price * multiplier + margin_fixed) * (self.lots as f64);
        released_margin
    }

    /// 计算 PnL（不区分已实现/未实现）
    fn pnl(&self, lots: u32, price: f64, multiplier: f64, direction: DirectionType) -> f64 {
        let diff = match direction {
            DirectionType::BUY => price - self.avg_price,
            DirectionType::SELL => self.avg_price - price,
        };
        diff * multiplier * (lots as f64)
    }

    /// 当前浮动盈亏
    fn unrealized_pnl(&self, last_price: f64, multiplier: f64, direction: DirectionType) -> f64 {
        self.pnl(self.lots, last_price, multiplier, direction)
    }

    /// 平仓时的已实现盈亏
    fn realized_pnl(&self, lots: u32, price: f64, multiplier: f64, direction: DirectionType) -> f64 {
        self.pnl(lots, price, multiplier, direction)
    }
}

pub struct PerformanceTracker {
    init_cash: f64,
    info: ContractInfo,
    available_cash: f64,
    long_position: Option<Position>,
    short_position: Option<Position>,
    market_values: Vec<f64>,
    total_fee: f64,
    total_realized_pnl: f64,
    orders: Vec<Order>,
}

impl PerformanceTracker {
    pub fn new(init_cash: f64, info: ContractInfo) -> Self {
        Self {
            init_cash,
            info,
            available_cash: init_cash,
            long_position: None,
            short_position: None,
            market_values: vec![init_cash],
            total_fee: 0.0,
            total_realized_pnl: 0.0,
            orders: Vec::with_capacity(1024),
        }
    }

    pub fn on_fill(&mut self, order: &Order, tick: &TickData) {
        let (price, margin_rate, margin_fixed, pos_opt_slot) = match order.direction {
            DirectionType::BUY => (
                tick.ap1,                    // 卖一价成交
                self.info.long_margin_rate,  // 多头开仓保证金(按金额)
                self.info.long_margin_fixed, // 多头开仓保证金(按手数)
                &mut self.long_position,     // 多头持仓
            ),
            DirectionType::SELL => (
                tick.bp1,                     // 买一价成交
                self.info.short_margin_rate,  // 空头开仓保证金(按金额)
                self.info.short_margin_fixed, // 空头开仓保证金(按手数)
                &mut self.short_position,     // 空头持仓
            ),
        };

        let (fee_rate, fee_fixed) = match order.offset {
            OffsetFlagType::OPEN => (
                self.info.open_fee_rate,  // 多空开仓手续费(按金额)
                self.info.open_fee_fixed, // 多空开仓手续费(按手数)
            ),
            // 为了计算方便，使用更高的平昨费率
            OffsetFlagType::CLOSE => (
                self.info.close_fee_rate,  // 多空平仓手续费(按金额)
                self.info.close_fee_fixed, // 多空平仓手续费(按手数)
            ),
        };

        // 1) 计算手续费
        let value_per_lot = price * self.info.multiplier;
        let fee = (fee_rate * value_per_lot + fee_fixed) * (order.lots as f64);
        self.total_fee += fee;
        self.available_cash -= fee;

        // 2) 更新持仓和保证金
        match order.offset {
            OffsetFlagType::OPEN => {
                // 新增/累加持仓
                let pos = pos_opt_slot.get_or_insert_with(|| Position::new(0, price, margin_rate, margin_fixed, self.info.multiplier));
                // 如果已有仓位，重新计算加权均价和保证金
                let prev_margin = pos.increase(order.lots, price, margin_rate, margin_fixed, self.info.multiplier);
                // 冻结保证金
                self.available_cash -= pos.margin - prev_margin; // 增量冻结
            }
            OffsetFlagType::CLOSE => {
                if let Some(pos) = pos_opt_slot {
                    // 已经实现的pnl
                    let closed_lots = order.lots.min(pos.lots);
                    let realized_pnl = pos.realized_pnl(closed_lots, price, self.info.multiplier, order.direction);
                    self.available_cash += realized_pnl;
                    self.total_realized_pnl += realized_pnl;
                    // 释放对应保证金
                    let released_margin = pos.decrease(closed_lots, price, margin_rate, margin_fixed, self.info.multiplier);
                    self.available_cash += released_margin;
                    // 清理仓位
                    if pos.lots == 0 {
                        pos_opt_slot.take();
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

        if let Some(pos) = &self.long_position {
            total_unreal += pos.unrealized_pnl(tick.last, self.info.multiplier, DirectionType::BUY);
            total_margin += pos.margin;
        }
        if let Some(pos) = &self.short_position {
            total_unreal += pos.unrealized_pnl(tick.last, self.info.multiplier, DirectionType::SELL);
            total_margin += pos.margin;
        }

        self.market_values.push(self.available_cash + total_unreal + total_margin);
    }
}
