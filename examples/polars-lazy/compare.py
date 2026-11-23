# /// script
# requires-python = ">=3.13"
# dependencies = ["polars==2.1.0"]
# ///

import argparse
import sys
from pathlib import Path
from time import perf_counter

import polars as pl
from polars.testing import assert_frame_equal

ROW_COUNT = 1_000_000
FIRST_RETAINED_ROW = 990_000


def eager(path: Path) -> pl.DataFrame:
    readings = pl.read_parquet(path)
    return readings.filter(pl.col("keep")).select("row_id", "reading")


def dynamic_projection(path: Path, columns: list[str]) -> pl.DataFrame:
    readings = pl.read_parquet(path)
    return readings.filter(pl.col("keep")).select(columns)


def lazy_query(path: Path) -> pl.LazyFrame:
    return pl.scan_parquet(path).filter(pl.col("keep")).select("row_id", "reading")


def lazy(path: Path) -> pl.DataFrame:
    return lazy_query(path).collect(engine="in-memory")


def generate(path: Path) -> None:
    rows = pl.int_range(0, ROW_COUNT, dtype=pl.Int64, eager=True).alias("row_id")
    pl.DataFrame(rows).with_columns(
        (pl.col("row_id") >= FIRST_RETAINED_ROW).alias("keep"),
        (pl.col("row_id") % 500).alias("reading"),
        *((pl.col("row_id") * i % 104_729).alias(f"extra_{i}") for i in range(1, 13)),
    ).write_parquet(path, row_group_size=50_000)


def verify(path: Path) -> None:
    expected = eager(path)
    actual = lazy(path)
    assert_frame_equal(
        expected,
        actual,
        check_row_order=True,
        check_column_order=True,
        check_dtypes=True,
        check_exact=True,
    )
    sys.stdout.write(f"Equal schema, values and row order: {actual.shape} {actual.schema}\n")
    sys.stdout.write(lazy_query(path).explain(optimized=True, engine="in-memory") + "\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=("generate", "verify", "eager", "lazy"))
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    match args.mode:
        case "generate":
            generate(args.path)
            sys.stdout.write(
                f"{ROW_COUNT:,} rows, 15 columns, {args.path.stat().st_size:,} bytes\n"
            )
        case "verify":
            verify(args.path)
        case "eager" | "lazy":
            run = eager if args.mode == "eager" else lazy
            start = perf_counter()
            result = run(args.path)
            elapsed = perf_counter() - start
            sys.stdout.write(
                f"{args.mode}: {elapsed:.6f} s through materialization, {result.height} rows\n"
            )


if __name__ == "__main__":
    main()
