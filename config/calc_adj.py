import polars as pl


def calc_adj(ipc_source: str, out_filename: str):
    """
    根据日线计算后复权因子: code, dt, adj
    """
    (
        pl.read_ipc(ipc_source)
        .sort(["code", "dt"])
        .with_columns((pl.col("settle").shift(1) / pl.col("presettle")).fill_null(1).cum_prod().over("code").alias("adj"))
        .select("code", "dt", "adj")
        .write_ipc(out_filename)
    )


if __name__ == "__main__":
    calc_adj("/mnt/d/bars/fu1d/fu_bar1d.ipc", "futures_adj.ipc")
