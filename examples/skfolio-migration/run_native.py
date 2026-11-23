# /// script
# requires-python = ">=3.14,<3.15"
# dependencies = ["polars==2.1.0", "numpy==2.5.5", "cvxpy-base==1.9.3", "clarabel==0.11.1"]
# ///
"""Produce the declared migration outputs without pandas or skfolio."""

import argparse
import json
from importlib.util import find_spec
from pathlib import Path

import polars as pl

from native import DATE, native_returns, weekly_folds
from portfolio import observation_folds, walk_forward


def main() -> None:
    """Run the two manifests against packaged price and factor CSV files."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("prices", type=Path)
    parser.add_argument("factors", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    if find_spec("pandas") is not None or find_spec("skfolio") is not None:
        message = (
            "Run this entry point in a clean environment without pandas or skfolio"
        )
        raise RuntimeError(message)

    root = Path(__file__).resolve().parent
    returns_contract = json.loads(
        (root / "returns.manifest.json").read_text(encoding="utf-8")
    )
    walk_contract = json.loads(
        (root / "walk-forward.manifest.json").read_text(encoding="utf-8")
    )
    settings = walk_contract["contract"]

    prices = pl.read_csv(
        args.prices, try_parse_dates=True, schema_overrides={DATE: pl.Date}
    )
    factors = pl.read_csv(
        args.factors, try_parse_dates=True, schema_overrides={DATE: pl.Date}
    )
    returns = native_returns(prices)
    joined = native_returns(prices, factors)
    if isinstance(returns, tuple) or not isinstance(joined, tuple):
        message = "Unexpected native return table shape"
        raise TypeError(message)
    if returns.columns != returns_contract["contract"]["output"]["ordered_columns"]:
        message = "Native return schema differs from the declared ordered columns"
        raise RuntimeError(message)

    selected = returns.select(DATE, *walk_contract["input"]["assets"]).tail(
        walk_contract["input"]["last_rows"]
    )
    observed = observation_folds(
        selected.height,
        train=settings["train_observations"],
        test=settings["test_observations"],
    )
    calendar = weekly_folds(
        selected[DATE].to_list(),
        train_weeks=settings["calendar"]["train_weeks"],
        test_weeks=settings["calendar"]["test_weeks"],
    )
    outputs = {
        "returns": returns,
        "joined-assets": joined[0],
        "joined-factors": joined[1],
    }
    for name, folds in (("observation", observed), ("calendar", calendar)):
        daily, ending = walk_forward(
            selected,
            folds,
            transaction_cost=settings["transaction_cost_per_observation"],
        )
        outputs[f"{name}-path"] = daily
        outputs[f"{name}-endings"] = ending

    args.output.mkdir()
    for name, frame in outputs.items():
        frame.write_parquet(args.output / f"{name}.parquet")
    print(
        json.dumps(
            {"pandas_present": False, "skfolio_present": False, "outputs": len(outputs)}
        )
    )


if __name__ == "__main__":
    main()
