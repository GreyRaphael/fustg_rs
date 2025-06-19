import polars as pl
import httpx


def download_times():
    j = httpx.get("http://dict.openctp.cn/times?types=futures&areas=China").json()
    (
        pl.from_records(j["data"])
        .group_by(
            ["ExchangeID", "TimeBegin", "TimeEnd"],
        )
        .first()
        .sort(
            ["ExchangeID", "TimeBegin", "TimeEnd"],
        )
        .write_csv("exchange_times.csv")
    )


if __name__ == "__main__":
    download_times()
