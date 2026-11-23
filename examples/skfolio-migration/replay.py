# /// script
# requires-python = ">=3.14,<3.15"
# dependencies = ["skfolio==1.14.4", "polars==2.1.0", "pandas==3.1.1", "pyarrow==26.0.1", "numpy==2.5.5", "cvxpy-base==1.9.3"]
# ///
"""Verify both migration manifests against the pinned skfolio baseline."""

import hashlib
import json
import subprocess
from importlib.metadata import distribution
from itertools import product
from pathlib import Path
from tempfile import TemporaryDirectory

import numpy as np
import pandas as pd
import polars as pl
from skfolio.datasets import load_factors_dataset, load_sp500_dataset
from skfolio.model_selection import WalkForward, cross_val_predict
from skfolio.optimization import MeanRisk
from skfolio.preprocessing import prices_to_returns

from native import DATE, native_returns, weekly_folds


def fixture(details: dict[str, object]) -> Path:
    """Return the package file specified by the manifest."""
    resource = details["resource"]
    if not isinstance(resource, str):
        message = "Expected the fixture path to be a string"
        raise TypeError(message)
    path = Path(distribution("skfolio").locate_file(resource))
    if hashlib.sha256(path.read_bytes()).hexdigest() != details["sha256"]:
        message = f"Fixture differs from manifest: {path.name}"
        raise ValueError(message)
    return path


def polars_dates(frame: pd.DataFrame) -> pl.DataFrame:
    """Use the original dates as an explicit date key in proof fixtures."""
    return pl.from_pandas(frame.rename_axis(DATE).reset_index()).with_columns(
        pl.col(DATE).cast(pl.Date)
    )


def compare_returns(old: pd.DataFrame, new: pl.DataFrame, tolerance: float) -> float:
    """Compare an ordered table and report its largest numeric difference."""
    converted = new.to_pandas().set_index(DATE)
    converted.index = pd.DatetimeIndex(
        converted.index, dtype=old.index.dtype, name=DATE
    )
    pd.testing.assert_frame_equal(
        old.rename_axis(DATE), converted, check_exact=False, rtol=0, atol=tolerance
    )
    return float((old - converted).abs().max().max())


def check_preprocessing(
    prices: pd.DataFrame, factors: pd.DataFrame, joins: list[str], tolerance: float
) -> dict[str, object]:
    """Exercise join and missingness options against the real reference function."""
    native_prices = polars_dates(prices)
    native_factors = polars_dates(factors)
    joined_rows = {}
    for join in joins:
        old = prices_to_returns(prices, factors, join=join)
        new = native_returns(native_prices, native_factors, join=join)
        for expected, actual in zip(old, new, strict=True):
            compare_returns(expected, actual, tolerance)
        joined_rows[join] = [len(table) for table in old]

    staggered_assets = prices.iloc[:8, :2]
    staggered_factors = prices.iloc[3:11, 2:4]
    staggered_rows = {}
    for join in joins:
        old = prices_to_returns(
            staggered_assets, staggered_factors, join=join, drop_inceptions_nan=False
        )
        new = native_returns(
            polars_dates(staggered_assets),
            polars_dates(staggered_factors),
            join=join,
            drop_inceptions_nan=False,
        )
        for expected, actual in zip(old, new, strict=True):
            compare_returns(expected, actual, tolerance)
        staggered_rows[join] = len(old[0])

    edge = prices.iloc[:9, :3].copy()
    edge.iloc[:, 2] = np.nan
    edge.iloc[0, 0] = np.nan
    edge.iloc[3, :] = np.nan
    edge.iloc[5, 1] = np.nan
    native_edge = polars_dates(edge)
    cases = list(product((True, False), (True, False), (1.0, None, 0.4), (True, False)))
    for fill, inception, threshold, log in cases:
        options = {
            "fill_nan": fill,
            "drop_inceptions_nan": inception,
            "nan_threshold": threshold,
            "log_returns": log,
        }
        compare_returns(
            prices_to_returns(edge, **options),
            native_returns(native_edge, **options),
            tolerance,
        )
    return {
        "join_rows": joined_rows,
        "staggered_calendar_join_rows": staggered_rows,
        "missingness_cases": len(cases),
    }


def check_weekly_windows(frame: pd.DataFrame, train_weeks: int) -> int:
    """Compare holiday-aware weekly boundaries over all declared option combinations."""
    dates = frame.index.date.tolist()
    cases = list(product((True, False), (True, False), (True, False), (0, 2), (1, 3)))
    for previous, expand, reduce, purge, test_weeks in cases:
        common = {"previous": previous, "expand_train": expand, "reduce_test": reduce}
        old = WalkForward(
            train_size=train_weeks,
            test_size=test_weeks,
            freq="W-FRI",
            purged_size=purge,
            **common,
        )
        new = weekly_folds(
            dates, train_weeks=train_weeks, test_weeks=test_weeks, purge=purge, **common
        )
        for (train, test), (new_train, new_test) in zip(
            old.split(frame), new, strict=True
        ):
            np.testing.assert_array_equal(train, list(new_train))
            np.testing.assert_array_equal(test, list(new_test))
    return len(cases)


def compare_backtest(
    directory: Path,
    name: str,
    frame: pd.DataFrame,
    splitter: WalkForward,
    settings: dict,
) -> dict[str, int | float]:
    """Compare persisted dated strategy returns and ending holdings."""
    old = cross_val_predict(
        MeanRisk(transaction_costs=settings["transaction_cost_per_observation"]),
        frame,
        cv=splitter,
        portfolio_params={
            "weight_drift": settings["weight_drift"],
            "compounded": settings["compounded"],
        },
    )
    actual = pl.read_parquet(directory / f"{name}-path.parquet")
    holdings = pl.read_parquet(directory / f"{name}-endings.parquet")
    expected_dates = pd.DatetimeIndex(old.observations).date.tolist()
    expected_ending_dates = [
        pd.Timestamp(portfolio.observations[-1]).date() for portfolio in old.portfolios
    ]
    np.testing.assert_array_equal(actual[DATE].to_list(), expected_dates)
    np.testing.assert_array_equal(holdings[DATE].to_list(), expected_ending_dates)
    np.testing.assert_array_equal(holdings.columns, [DATE, *frame.columns])
    old_holdings = np.stack([portfolio.ending_weights for portfolio in old.portfolios])
    pairs = (
        (actual["return"].to_numpy(), old.returns),
        (actual["wealth"].to_numpy(), old.cumulative_returns),
        (holdings.drop(DATE).to_numpy(), old_holdings),
    )
    for observed, expected in pairs:
        np.testing.assert_allclose(
            observed, expected, rtol=0, atol=settings["float_atol"]
        )
    declared = settings if name == "observation" else settings["calendar"]
    np.testing.assert_equal(actual.height, declared["output_observations"])
    np.testing.assert_equal(holdings.height, declared["output_portfolios"])
    return {
        "observations": actual.height,
        "portfolios": holdings.height,
        "max_return_delta": float(np.max(np.abs(pairs[0][0] - pairs[0][1]))),
        "max_wealth_delta": float(np.max(np.abs(pairs[1][0] - pairs[1][1]))),
        "max_weight_delta": float(np.max(np.abs(pairs[2][0] - pairs[2][1]))),
    }


def main() -> None:
    """Verify live outputs in an isolated native runtime and print the evidence."""
    root = Path(__file__).resolve().parent
    returns_manifest = json.loads(
        (root / "returns.manifest.json").read_text(encoding="utf-8")
    )
    backtest_manifest = json.loads(
        (root / "walk-forward.manifest.json").read_text(encoding="utf-8")
    )
    contract = returns_manifest["contract"]
    settings = backtest_manifest["contract"]
    price_file = fixture(returns_manifest["input"])
    factor_file = fixture(contract["factor_input"])
    prices = load_sp500_dataset()
    factors = load_factors_dataset()
    np.testing.assert_equal(len(prices), returns_manifest["input"]["rows"])
    np.testing.assert_equal(len(factors), contract["factor_input"]["rows"])
    np.testing.assert_array_equal(prices.columns, returns_manifest["input"]["assets"])
    np.testing.assert_array_equal(factors.columns, contract["factor_input"]["columns"])

    old_returns = prices_to_returns(prices)
    selected = old_returns.loc[:, backtest_manifest["input"]["assets"]].tail(
        backtest_manifest["input"]["last_rows"]
    )
    preprocessing = check_preprocessing(
        prices, factors, contract["factor_joins"], contract["float_atol"]
    )
    options = check_weekly_windows(selected, settings["calendar"]["train_weeks"])

    with TemporaryDirectory() as folder:
        output = Path(folder) / "native"
        run = subprocess.run(
            [
                "uv",
                "run",
                "--isolated",
                "--no-project",
                "--no-build",
                str(root / "run_native.py"),
                str(price_file),
                str(factor_file),
                str(output),
            ],
            check=True,
            capture_output=True,
            text=True,
            timeout=90,
        )
        presence = json.loads(run.stdout)
        actual = pl.read_parquet(output / "returns.parquet")
        np.testing.assert_equal(actual.height, contract["output"]["rows"])
        np.testing.assert_equal(
            str(actual.schema[DATE]), contract["output"]["date_dtype"]
        )
        np.testing.assert_array_equal(
            actual.columns, contract["output"]["ordered_columns"]
        )
        max_delta = compare_returns(old_returns, actual, contract["float_atol"])
        for expected, name in zip(
            prices_to_returns(prices, factors),
            ("joined-assets", "joined-factors"),
            strict=True,
        ):
            compare_returns(
                expected,
                pl.read_parquet(output / f"{name}.parquet"),
                contract["float_atol"],
            )

        calendar = settings["calendar"]
        observed = compare_backtest(
            output,
            "observation",
            selected,
            WalkForward(
                train_size=settings["train_observations"],
                test_size=settings["test_observations"],
            ),
            settings,
        )
        weekly = compare_backtest(
            output,
            "calendar",
            selected,
            WalkForward(
                train_size=calendar["train_weeks"],
                test_size=calendar["test_weeks"],
                freq=calendar["frequency"],
            ),
            settings,
        )

    print(
        json.dumps(
            {
                "pandas_free_runtime": presence,
                "price_rows": len(prices),
                "factor_rows": len(factors),
                "returns_rows": len(old_returns),
                "max_return_delta": max_delta,
                "preprocessing": preprocessing,
                "weekly_split_configurations": options,
                "observation_backtest": observed,
                "calendar_backtest": weekly,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
