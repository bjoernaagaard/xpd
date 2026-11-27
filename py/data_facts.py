import json
import math
import runpy
import sys
from contextlib import redirect_stdout
from itertools import combinations
from pathlib import Path

import numpy as np
import pandas as pd
from scipy import stats

MIN_SKEW_ROWS = 3
MIN_KURTOSIS_ROWS = 4
MIN_CORRELATION_ROWS = 30
MISSINGNESS_CORRELATION = 0.3
ZERO_INFLATION_SHARE = 0.5


class DataAdapterError(ValueError):
    @classmethod
    def invalid_suffix(cls, suffix: str) -> "DataAdapterError":
        return cls(f"Unsupported adapter data file type: {suffix}")

    @classmethod
    def invalid_columns(cls) -> "DataAdapterError":
        return cls("columns() must return a pandas or Polars frame, a path, or a dictionary")

    @classmethod
    def duplicate_names(cls) -> "DataAdapterError":
        return cls("Adapter column names must be unique")


def as_frame(value: object) -> pd.DataFrame:
    if isinstance(value, pd.DataFrame):
        return value
    if isinstance(value, dict):
        return pd.DataFrame(value)
    if isinstance(value, (str, Path)):
        path = Path(value)
        readers = {
            ".csv": pd.read_csv,
            ".parquet": pd.read_parquet,
            ".feather": pd.read_feather,
            ".json": pd.read_json,
            ".xlsx": pd.read_excel,
        }
        reader = readers.get(path.suffix.lower())
        if reader is None:
            raise DataAdapterError.invalid_suffix(path.suffix)
        return reader(path)
    if type(value).__module__.split(".")[0] == "polars" and hasattr(value, "to_dict"):
        return pd.DataFrame(value.to_dict(as_series=False))
    raise DataAdapterError.invalid_columns()


def share(count: int, rows: int) -> float | None:
    return count / rows if rows else None


def finite(value: float) -> float | None:
    return float(value) if math.isfinite(value) else None


def numeric_facts(series: pd.Series) -> dict[str, object]:
    values = series.dropna().to_numpy(dtype=float)
    values = values[np.isfinite(values)]
    sentinel_codes = [-999, -9999, 999, 9999]
    if not (values[~np.isin(values, [*sentinel_codes, -1])] < 0).any():
        sentinel_codes.append(-1)
    sentinel = np.isin(values, sentinel_codes)
    observed = values[~sentinel]
    zero_share = share(int((observed == 0).sum()), len(series))
    result = {
        "sentinel_share": share(int(sentinel.sum()), len(series)),
        "sentinels": {
            str(code): share(int((values == code).sum()), len(series))
            for code in sentinel_codes
            if (values == code).any()
        },
        "zero_share": zero_share,
        "zero_inflated": zero_share is not None and zero_share > ZERO_INFLATION_SHARE,
    }
    if not len(observed):
        return result
    lower, upper = np.quantile(observed, [0.25, 0.75])
    fence = 1.5 * (upper - lower)
    outliers = (observed < lower - fence) | (observed > upper + fence)
    varied = observed.max() != observed.min()
    return {
        **result,
        "min": float(observed.min()),
        "skew": finite(stats.skew(observed)) if varied and len(observed) >= MIN_SKEW_ROWS else None,
        "excess_kurtosis": finite(stats.kurtosis(observed))
        if varied and len(observed) >= MIN_KURTOSIS_ROWS
        else None,
        "outlier_share": share(int(outliers.sum()), len(observed)),
        "largest_values": sorted(map(float, observed), reverse=True)[:10],
    }


def missingness(frame: pd.DataFrame) -> list[dict[str, object]] | None:
    if len(frame) < MIN_CORRELATION_ROWS:
        return None
    masks = frame.isna()
    masks = masks.loc[:, masks.any() & ~masks.all()]
    correlations = masks.corr()
    return [
        {"columns": [left, right], "r": float(correlations.loc[left, right])}
        for left, right in combinations(masks.columns, 2)
        if abs(correlations.loc[left, right]) >= MISSINGNESS_CORRELATION
    ]


def measure(
    frame: pd.DataFrame, target: str | None, model_family: str, profile: dict
) -> dict[str, object]:
    frame = frame.set_axis([str(column) for column in frame.columns], axis=1)
    if not frame.columns.is_unique:
        raise DataAdapterError.duplicate_names()
    rows = len(frame)
    target = target or next(
        (name for name in frame.columns if name in {"target", "y", "label"}), None
    )
    memory = frame.memory_usage(deep=True, index=False)
    findings = list(profile["wide_findings"](list(frame.columns)))
    numeric_frame = frame.select_dtypes(include="number")
    columns = []
    for index, (name, series) in enumerate(frame.items()):
        unique = profile["unique_count"](series)
        used = int(memory[name])
        numeric = name in numeric_frame
        categorical = pd.api.types.is_string_dtype(series.dtype) or isinstance(
            series.dtype, pd.CategoricalDtype
        )
        key_like = index == 0 or name.lower() in {"id", "key"} or name.lower().endswith("_id")
        nulls = int(series.isna().sum())
        column = {
            "column": name,
            "dtype": str(series.dtype),
            "nulls": nulls,
            "missing_share": share(nulls, rows),
            "unique": unique,
            "memory_bytes": used,
            "numeric": numeric,
            "categorical": categorical,
            "key_like": key_like,
        }
        if finding := profile["category_finding"](series, unique, used):
            findings.append(finding)
        counts = (
            series.value_counts()
            if unique is not None and (categorical or key_like or name == target)
            else None
        )
        if counts is not None and not counts.empty:
            if key_like or name == target:
                column["duplicate_share"] = share(int((counts - 1).clip(lower=0).sum()), rows)
                column["largest_repeat_count"] = int(counts.max())
            if categorical:
                column["rarest_level_share"] = share(int(counts[counts > 0].min()), rows)
        if numeric:
            column.update(numeric_facts(series))
            if name != target and target in numeric_frame:
                column["target_correlation"] = finite(series.corr(numeric_frame[target]))
        columns.append(column)
    try:
        duplicate_rows = share(int(frame.duplicated().sum()), rows) if len(frame.columns) else None
    except TypeError:
        duplicate_rows = None
    return {
        "rows": rows,
        "columns": columns,
        "target": target,
        "model_family": model_family,
        "findings": findings,
        "duplicate_row_share": duplicate_rows,
        "missingness_correlations": missingness(frame),
    }


def main(profile: dict, target: str | None) -> None:
    path = Path(sys.argv[1]).resolve()
    sys.path.insert(0, str(path.parent))
    with redirect_stdout(sys.stderr):
        adapter = runpy.run_path(str(path))
        frame = as_frame(adapter["columns"]())
        result = measure(
            frame, target or adapter.get("target"), adapter.get("model_family", "unknown"), profile
        )
    print(json.dumps(result, allow_nan=False))
