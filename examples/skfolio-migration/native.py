"""Native preprocessing and calendar windows for the skfolio migration."""

from bisect import bisect_left, bisect_right
from collections.abc import Iterator
from datetime import date, timedelta
from itertools import pairwise

import polars as pl

DATE = "Date"
WEEK = timedelta(days=7)


def nonempty_columns(frame: pl.DataFrame) -> list[str]:
    """Keep the date and price columns with at least one observation."""
    return [
        column
        for column, missing in frame.null_count().row(0, named=True).items()
        if column == DATE or missing < frame.height
    ]


def check_dates(frame: pl.DataFrame) -> None:
    """Require the declared unique, ascending daily date key."""
    if DATE not in frame or frame.schema[DATE] != pl.Date:
        message = "An explicit Polars Date column named Date is required"
        raise ValueError(message)
    dates = frame[DATE]
    if dates.null_count() or not dates.is_sorted() or dates.n_unique() != frame.height:
        message = "Dates must be non-null, unique and ascending"
        raise ValueError(message)


def native_returns(
    prices: pl.DataFrame,
    factors: pl.DataFrame | None = None,
    *,
    log_returns: bool = False,
    nan_threshold: float | None = 1.0,
    join: str = "outer",
    drop_inceptions_nan: bool = True,
    fill_nan: bool = True,
) -> pl.DataFrame | tuple[pl.DataFrame, pl.DataFrame]:
    """Compute returns on unique ascending dates and preserve named asset and factor columns."""
    if nan_threshold is not None and not 0 < nan_threshold <= 1:
        message = "nan_threshold must be between 0 and 1"
        raise ValueError(message)

    check_dates(prices)
    normalized = prices.with_columns(pl.exclude(DATE).cast(pl.Float64).fill_nan(None))
    asset_names = [name for name in prices.columns if name != DATE]
    factor_names = []
    if factors is not None:
        check_dates(factors)
        if join not in {"outer", "inner", "left", "right"}:
            message = f"Unsupported factor join: {join}"
            raise ValueError(message)
        factor_names = [name for name in factors.columns if name != DATE]
        if set(asset_names) & set(factor_names):
            message = "Asset and factor column names must be disjoint"
            raise ValueError(message)
        other = factors.with_columns(pl.exclude(DATE).cast(pl.Float64).fill_nan(None))
        normalized = normalized.join(
            other, on=DATE, how="full" if join == "outer" else join, coalesce=True
        ).sort(DATE)

    prepared = normalized.select(nonempty_columns(normalized))
    available_assets = [name for name in asset_names if name in prepared.columns]
    if nan_threshold is not None and available_assets:
        missing = pl.sum_horizontal(pl.col(available_assets).is_null().cast(pl.UInt32))
        prepared = prepared.filter(missing <= len(available_assets) * nan_threshold)
        prepared = prepared.select(nonempty_columns(prepared))

    columns = [name for name in prepared.columns if name != DATE]
    if columns:
        if fill_nan:
            prepared = prepared.with_columns(pl.col(columns).forward_fill())
        complete = pl.all_horizontal(pl.col(columns).is_not_null())
        if drop_inceptions_nan:
            prepared = prepared.filter(complete.cum_max())
        prepared = prepared.filter(pl.any_horizontal(pl.col(columns).is_not_null()))
        changes = pl.col(columns) / pl.col(columns).shift(1) - 1
        if log_returns:
            changes = changes.log1p()
        result = prepared.with_columns(changes).slice(1)
    else:
        result = prepared.head(0)

    if factors is None:
        return result
    return (
        result.select(DATE, *(name for name in asset_names if name in columns)),
        result.select(DATE, *(name for name in factor_names if name in columns)),
    )


def weekly_folds(
    dates: list[date],
    *,
    train_weeks: int,
    test_weeks: int,
    previous: bool = False,
    purge: int = 0,
    expand_train: bool = False,
    reduce_test: bool = False,
) -> Iterator[tuple[range, range]]:
    """Split explicit daily dates on Friday anchors."""
    if train_weeks < 1 or test_weeks < 1 or purge < 0:
        message = "Train and test weeks must be positive and purge nonnegative"
        raise ValueError(message)
    if any(left >= right for left, right in pairwise(dates)):
        message = "Dates must be unique and ascending"
        raise ValueError(message)
    if not dates:
        return

    first_friday = dates[0] + timedelta(days=(4 - dates[0].weekday()) % 7)
    anchor_count = (dates[-1] - first_friday) // WEEK + 1
    anchors = [first_friday + period * WEEK for period in range(anchor_count)]
    if previous:
        positions = [bisect_right(dates, anchor) - 1 for anchor in anchors]
    else:
        positions = [bisect_left(dates, anchor) for anchor in anchors]
    stop_period = anchor_count if reduce_test else anchor_count - test_weeks
    for period in range(train_weeks, stop_period, test_weeks):
        test_start = positions[period]
        end_period = period + test_weeks
        test_stop = positions[end_period] if end_period < anchor_count else len(dates)
        if test_stop <= test_start:
            continue
        train_start = 0 if expand_train else positions[period - train_weeks]
        train_end = test_start - purge
        if train_end <= train_start:
            message = f"Empty training window before test starting {dates[test_start]}"
            raise ValueError(message)
        yield range(train_start, train_end), range(test_start, test_stop)
