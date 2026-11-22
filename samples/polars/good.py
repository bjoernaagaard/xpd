import polars as pl
import polars.selectors as cs
from polars.testing import assert_frame_equal

frame = pl.DataFrame({"key": [1, 2], "amount": [4, 5]})
lookup = pl.DataFrame({"key": [1, 2], "rate": [2, 3]})
query = frame.lazy()
amount = pl.col("amount")

assert_frame_equal(query.collect(), frame)
query.collect_schema()
query.join(lookup.lazy(), on="key", validate="1:1").collect()
query.unpivot(on="amount", index="key").sort("key").head(1).collect()
query.sink_parquet("output.parquet")
frame.join_asof(lookup, on="key")
frame.select(pl.count("amount"))
frame.select(pl.when(amount > 0).then(amount).otherwise(0))
frame.filter((amount > 0) & (amount < 6))
frame.filter(amount.is_not_null())
frame.select(cs.numeric() | cs.by_name("amount"))
frame.select(amount.map_batches(lambda values: values, pl.Int64))
pl.CompatLevel.newest()
pl.Series([1, 2]).cast(pl.String)
pl.Series([[1], []]).list.get(1, null_on_oob=True)
pl.Series(["closed"], dtype=pl.Enum(["open", "closed"]))
pl.DataFrame([[1, 2], [3, 4]], schema=["a", "b"], orient="row")
pl.scan_csv("input.csv", schema_overrides={"key": pl.Int64}).select("key")
pl.SQLContext(frame=frame).execute("SELECT * FROM frame")
len(pl.Series([1, 2]))
