import polars as pl
import toml


def read_fee(out_filename: str = "contracts.toml", rank: int = 1):
    """
    rank = 1 表示持仓量第1的主力合约
    rank = 2 表示持仓量第2的次主力合约
    rank = N 表示持仓量第N的合约
    """
    df = (
        pl.read_excel(
            "http://openctp.cn/fees.xls",
            columns=[
                "交易所",
                # "合约代码",
                "品种代码",
                "合约乘数",
                "最小跳动",
                "开仓费率（按金额）",
                "开仓费用（按手）",
                "平仓费率（按金额）",
                "平仓费用（按手）",
                "平今费率（按金额）",
                "平今费用（按手）",
                "做多保证金率（按金额）",
                "做多保证金（按手）",
                "做空保证金率（按金额）",
                "做空保证金（按手）",
                "持仓量",
            ],
            schema_overrides={
                "交易所": pl.Utf8,
                # "合约代码": pl.Utf8,
                "品种代码": pl.Utf8,
                "合约乘数": pl.UInt32,
                "最小跳动": pl.Float64,
                "开仓费率（按金额）": pl.Float64,
                "开仓费用（按手）": pl.Float64,
                "平仓费率（按金额）": pl.Float64,
                "平仓费用（按手）": pl.Float64,
                "平今费率（按金额）": pl.Float64,
                "平今费用（按手）": pl.Float64,
                "做多保证金率（按金额）": pl.Float64,
                "做多保证金（按手）": pl.Float64,
                "做空保证金率（按金额）": pl.Float64,
                "做空保证金（按手）": pl.Float64,
                "持仓量": pl.Float64,
            },
        )
        .with_columns(pl.col("持仓量").rank(method="ordinal", descending=True).over("品种代码").alias("rnk"))
        .filter(pl.col("rnk") == rank)  # 主力合约hot rank=1, 次主力rank=2
        .select(pl.exclude("持仓量", "rnk"))
        .rename(
            {
                "交易所": "ex",
                # "合约代码": "code",
                "品种代码": "product_id",
                "合约乘数": "contract_multiplier",
                "最小跳动": "min_move",
                "开仓费率（按金额）": "open_fee_rate",
                "开仓费用（按手）": "open_fee_fixed",
                "平仓费率（按金额）": "close_fee_rate",
                "平仓费用（按手）": "close_fee_fixed",
                "平今费率（按金额）": "close_today_fee_rate",
                "平今费用（按手）": "close_today_fee_fixed",
                "做多保证金率（按金额）": "long_margin_rate",
                "做多保证金（按手）": "long_margin_fixed",
                "做空保证金率（按金额）": "short_margin_rate",
                "做空保证金（按手）": "short_margin_fixed",
            }
        )
        .sort(by=["ex", "product_id"])
    )

    toml_dict = {}
    records = df.rows_by_key(key=["ex", "product_id"], named=True, unique=True)
    for ex, product_id in records:
        unique_name = f"{ex}.{product_id}"
        toml_dict[unique_name] = records[(ex, product_id)]

    with open(out_filename, "w", encoding="utf8") as target:
        toml.dump(toml_dict, target)


if __name__ == "__main__":
    read_fee(out_filename="fees.1st.toml", rank=1)
    read_fee(out_filename="fees.2nd.toml", rank=2)
