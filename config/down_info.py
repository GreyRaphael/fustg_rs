import polars as pl
import httpx


def download_instruments(out_fileanme: str):
    j = httpx.get("http://dict.openctp.cn/instruments?types=futures&areas=China").json()
    df = pl.from_records(j["data"])
    df.group_by("ProductID").first().with_columns(
        pl.col("ExchangeID").replace(
            {
                "CFFEX": "CFE",
                "CZCE": "CZC",
                "DCE": "DCE",
                "GFEX": "GFE",
                "INE": "INE",
                "SHFE": "SHF",
            }
        )
    ).select("ExchangeID", pl.col("ProductID").str.to_uppercase() + "." + pl.col("ExchangeID")).write_ipc(out_fileanme)


if __name__ == "__main__":
    download_instruments("openctp_product.ipc")
