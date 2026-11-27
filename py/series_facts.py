"""Measure a project's time series before any forecast is fitted."""

from __future__ import annotations

import json
import runpy
import sys
import warnings
from collections import Counter
from contextlib import redirect_stdout
from itertools import combinations
from pathlib import Path
from typing import TYPE_CHECKING, Any

import numpy as np
import pandas as pd
from sklearn.cluster import KMeans
from sklearn.metrics import silhouette_samples
from statsmodels.stats.diagnostic import het_arch
from statsmodels.tsa.seasonal import MSTL
from statsmodels.tsa.stattools import adfuller, kpss
from threadpoolctl import threadpool_limits

if TYPE_CHECKING:
    from collections.abc import Callable

MIN_STAMPS = 8
MIN_CADENCE_SHARE = 0.5
ADI_THRESHOLD = 1.32
CV2_THRESHOLD = 0.49
MIN_STATISTICAL_SAMPLE = 30
SIGNIFICANCE = 0.05
MIN_PERIOD = 2
MIN_GROUP_SIZE = 2
MIN_SEASONAL_STRENGTH = 0.3
COHERENT_SILHOUETTE = 0.25
COHERENT_CORRELATION = 0.7
MAX_METADATA_LEVELS = 50
MIN_CLUSTER_SERIES = 6
MAX_CLUSTERS = 6
WEEK = 7
DAILY = pd.Timedelta(days=1)
WEEKDAYS = ("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")
Fact = dict[str, Any]
PanelSource = pd.DataFrame | str | Path | dict[str, Any]
PeriodClaims = float | str | list[float] | tuple[float, ...] | None


def read_panel(data: PanelSource) -> pd.DataFrame:
    if isinstance(data, pd.DataFrame):
        return data
    if isinstance(data, (str, Path)):
        path = Path(data)
        readers = {
            ".parquet": pd.read_parquet,
            ".csv": pd.read_csv,
            ".json": pd.read_json,
            ".tsv": lambda path: pd.read_csv(path, sep="\t"),
        }
        return readers[path.suffix.lower()](path)
    to_dicts = getattr(data, "to_dicts", None)
    if callable(to_dicts):
        return pd.DataFrame(to_dicts())
    return pd.DataFrame(data)


def cadence_of(frame: pd.DataFrame) -> tuple[Fact, Fact, pd.Series, pd.Timedelta | None]:
    raw = frame["ds"]
    identifier = (
        pd.api.types.is_numeric_dtype(raw) or raw.astype(str).str.fullmatch(r"[+-]?\d+").all()
    )
    stamps = (
        pd.to_datetime(raw, format="mixed", errors="coerce", utc=True)
        if not identifier
        else pd.Series(pd.NaT, index=raw.index)
    )
    valid = stamps.dropna()
    ordered = pd.DatetimeIndex(valid.unique()).sort_values()
    steps = ordered[1:] - ordered[:-1]
    step, count = Counter(steps).most_common(1)[0] if len(steps) else (None, 0)
    share = count / len(steps) if len(steps) else 0.0
    values = pd.to_numeric(frame["y"], errors="coerce").replace([np.inf, -np.inf], np.nan)
    y = (
        pd.Series(values.to_numpy(), index=pd.DatetimeIndex(stamps))
        .loc[lambda series: series.index.notna()]
        .groupby(level=0)
        .mean()
    )
    is_time_series = (
        len(ordered) >= MIN_STAMPS
        and len(valid) == len(raw)
        and share >= MIN_CADENCE_SHARE
        and y.max() > y.min()
    )
    if step is not None and share >= MIN_CADENCE_SHARE:
        gaps = int(sum(max(0, delta // step - 1) for delta in steps))
    else:
        gaps = None
    cadence = {
        "n": len(ordered),
        "first": str(ordered[0]) if len(ordered) else None,
        "last": str(ordered[-1]) if len(ordered) else None,
        "modal_step": str(step) if step is not None else None,
        "modal_step_share": share,
        "is_time_series": bool(is_time_series),
    }
    return (
        cadence,
        {
            "missing": gaps,
            "duplicates": int(valid.duplicated().sum()),
            "out_of_order": int((valid.diff() < pd.Timedelta(0)).sum()),
            "unparsed": len(raw) - len(valid),
        },
        y,
        step,
    )


def intermittency_of(y: np.ndarray) -> Fact:
    demand = y[y != 0]
    adi = len(y) / len(demand) if len(demand) else None
    cv2 = float(demand.var() / demand.mean() ** 2) if len(demand) and demand.mean() else None
    sparse = adi is None or adi >= ADI_THRESHOLD
    variable = cv2 is not None and cv2 >= CV2_THRESHOLD
    return {
        "adi": adi,
        "cv2": cv2,
        "class": {
            (False, False): "smooth",
            (False, True): "erratic",
            (True, False): "intermittent",
            (True, True): "lumpy",
        }[sparse, variable],
    }


def p_value(test: Callable[[np.ndarray], Any], y: np.ndarray) -> float | None:
    if np.ptp(y) == 0:
        return None
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        try:
            result = float(test(y)[1])
        except (ValueError, np.linalg.LinAlgError):
            return None
    return result if np.isfinite(result) else None


def stationarity_of(y: np.ndarray) -> Fact | None:
    if len(y) < MIN_STATISTICAL_SAMPLE:
        return None
    level = {"adf": p_value(adfuller, y), "kpss": p_value(kpss, y)}
    difference = {"adf": p_value(adfuller, np.diff(y)), "kpss": p_value(kpss, np.diff(y))}

    def stationary(tests: dict[str, float | None]) -> bool:
        adf, kpss_value = tests["adf"], tests["kpss"]
        return adf is not None and kpss_value is not None and adf < SIGNIFICANCE <= kpss_value

    reading = "stationary" if stationary(level) else "unclear"
    if reading == "unclear" and stationary(difference):
        reading = "difference-stationary"
    if reading == "unclear" and stationary(
        {
            "adf": p_value(lambda values: adfuller(values, regression="ct"), y),
            "kpss": p_value(lambda values: kpss(values, regression="ct"), y),
        }
    ):
        reading = "trend"
    return {"level": level, "difference": difference, "reading": reading}


def variance_and_breaks(y: pd.Series) -> tuple[Fact | None, Fact]:
    values = y.to_numpy()
    left, right = np.array_split(values, 2)
    mean_shift = float(right.mean() - left.mean())
    variance_ratio = float(right.var() / left.var()) if left.var() else None
    standard_error = np.sqrt(left.var() / len(left) + right.var() / len(right))
    peak = int(np.argmax(np.abs(np.cumsum(values - values.mean()))))
    candidates = (
        [str(y.index[peak + 1])]
        if abs(mean_shift) > 2 * standard_error and peak + 1 < len(y)
        else []
    )
    if len(y) < MIN_STATISTICAL_SAMPLE:
        return None, {
            "candidates": candidates,
            "half_mean_shift": mean_shift,
            "half_variance_ratio": variance_ratio,
        }
    lag = np.column_stack((np.ones(len(values) - 1), values[:-1]))
    residual = values[1:] - lag @ np.linalg.lstsq(lag, values[1:], rcond=None)[0]
    return {
        "arch_p": p_value(het_arch, values - values.mean()),
        "lag1_squared_residual_arch_p": p_value(het_arch, residual**2),
        "half_variance_ratio": variance_ratio,
    }, {
        "candidates": candidates,
        "half_mean_shift": mean_shift,
        "half_variance_ratio": variance_ratio,
    }


def seasonality_of(y: pd.Series, step: pd.Timedelta, claims: list[float]) -> list[Fact]:
    regular = y.asfreq(step).interpolate(limit_direction="both")
    signal = regular.to_numpy()
    delta = np.arange(len(signal))
    slope, intercept = np.polyfit(delta, signal, 1)
    power = np.abs(np.fft.rfft(signal - (slope * delta + intercept))) ** 2
    peaks = [
        bin_number
        for bin_number in range(2, len(power))
        if power[bin_number] >= power[bin_number - 1]
        and (bin_number + 1 == len(power) or power[bin_number] >= power[bin_number + 1])
    ]
    ranked = sorted(peaks, key=lambda index: power[index], reverse=True)[:3]
    detected = {round(len(signal) / index): len(signal) / index for index in ranked}
    periods = sorted(
        {*detected, *(round(claim) for claim in claims)} & set(range(2, (len(signal) + 1) // 2))
    )
    if step == DAILY and len(signal) > 2 * WEEK:
        periods = sorted({WEEK, *periods})
    if not periods:
        return []
    decomposition = MSTL(signal, periods=periods).fit()
    components = np.asarray(decomposition.seasonal).reshape(len(signal), len(periods))
    residual = decomposition.resid
    facts = []
    for period, component in zip(periods, components.T, strict=True):
        cycle = (
            delta + (regular.index[0].weekday() if step == DAILY and period == WEEK else 0)
        ) % period
        means = [float(component[cycle == slot].mean()) for slot in range(period)]
        phase_index = int(np.argmax(means))
        variance = np.var(component + residual)
        facts.append(
            {
                "period": period,
                "periodogram_period": detected.get(period),
                "strength": max(0.0, float(1 - np.var(residual) / variance)) if variance else 0.0,
                "phase": WEEKDAYS[phase_index] if step == DAILY and period == WEEK else phase_index,
                "phase_index": phase_index,
                "cycle": means,
            }
        )
    return sorted(facts, key=lambda fact: (-fact["strength"], fact["period"]))


def trend_of(y: np.ndarray) -> Fact:
    time = np.arange(len(y))
    slope, intercept = np.polyfit(time, y, 1)
    residual = (y - (slope * time + intercept)).std()
    histogram = np.histogram(np.diff(y), bins=20)[0]
    probabilities = histogram[histogram > 0] / histogram.sum()
    autocorrelation = np.corrcoef(y[:-1], y[1:])[0, 1] if np.ptp(y[:-1]) and np.ptp(y[1:]) else None
    return {
        "slope": float(slope),
        "trend_strength": float(slope / residual) if residual else 0.0,
        "lag1_autocorrelation": autocorrelation,
        "cv": float(y.std() / abs(y.mean())) if y.mean() else None,
        "minimum": float(y.min()),
        "maximum": float(y.max()),
        "scale": float(y.std()),
        "zero_share": float(np.mean(y == 0)),
        "entropy": float(-np.sum(probabilities * np.log2(probabilities))),
    }


def declared_periods(group: pd.DataFrame, claims: PeriodClaims) -> list[float]:
    values = list(claims if isinstance(claims, (list, tuple)) else [claims])
    for column in ("claimed_period", "claimed_periods", "period", "season_length"):
        if column in group:
            for value in group[column].dropna():
                values.extend(value if isinstance(value, (list, tuple, np.ndarray)) else [value])
    return sorted(
        {
            float(value)
            for value in values
            if value is not None and np.isfinite(float(value)) and float(value) >= MIN_PERIOD
        }
    )


def measure_series(entity: str, frame: pd.DataFrame, claims: PeriodClaims) -> Fact:
    cadence, gaps, series, step = cadence_of(frame)
    result = {
        "unique_id": entity,
        "cadence": cadence,
        "is_time_series": cadence["is_time_series"],
        "gaps": gaps,
    }
    series = series.dropna()
    if not cadence["is_time_series"] or step is None:
        return result
    periods = declared_periods(frame, claims)
    y = series.to_numpy()
    intermittent = intermittency_of(y)
    variance, breaks = variance_and_breaks(series)
    seasonal = seasonality_of(series, step, periods)
    trend = trend_of(y)
    profile = {
        "trend": trend["trend_strength"],
        "adi": intermittent["adi"] or len(y),
        "entropy": trend["entropy"],
        "scale": trend["scale"],
        "cadence_seconds": step.total_seconds(),
    }
    for season in seasonal:
        period = season["period"]
        phase = season["phase_index"]
        profile[f"strength_{period}"] = season["strength"]
        profile[f"phase_{period}"] = phase
    return result | {
        "intermittency": intermittent,
        "stationarity": stationarity_of(y),
        "heteroscedasticity": variance,
        "breaks": breaks,
        "seasonality": seasonal,
        "trend": trend,
        "profile": profile,
        "claimed_periods": periods,
        "unsupported_periods": [period for period in periods if cadence["n"] < 2 * period],
    }


def phase_distance(left: Fact, right: Fact) -> float:
    gap = abs(left["phase_index"] - right["phase_index"])
    return min(gap, left["period"] - gap)


def seasonal_similarity(left: Fact, right: Fact) -> float | None:
    if not left["seasonality"] or not right["seasonality"]:
        return None
    left_peak, right_peak = left["seasonality"][0], right["seasonality"][0]
    if left_peak["period"] != right_peak["period"]:
        return 0.0
    first, second = left_peak["cycle"], right_peak["cycle"]
    return float(np.corrcoef(first, second)[0, 1]) if np.ptp(first) and np.ptp(second) else None


def group_coherence(indices: list[int], facts: list[Fact], silhouettes: np.ndarray | None) -> Fact:
    members = [facts[index] for index in indices]
    pairs = [seasonal_similarity(left, right) for left, right in combinations(members, 2)]
    measured = [value for value in pairs if value is not None]
    correlation = float(np.mean(measured)) if measured else None
    silhouette = float(np.mean(silhouettes[indices])) if silhouettes is not None else None
    peaks = [
        (fact["unique_id"], fact["seasonality"][0])
        for fact in members
        if fact["seasonality"] and fact["seasonality"][0]["strength"] > MIN_SEASONAL_STRENGTH
    ]
    outliers = []
    if peaks:

        def differs(left: Fact, right: Fact) -> bool:
            return left["period"] != right["period"] or phase_distance(left, right) > 1

        _, modal = min(
            peaks, key=lambda candidate: sum(differs(candidate[1], peak) for _, peak in peaks)
        )
        outliers = [entity for entity, peak in peaks if differs(modal, peak)]
    mismatch = len(outliers) / len(peaks) if peaks else 0.0
    coherent = len(indices) < MIN_GROUP_SIZE or (
        mismatch <= 1 / 3
        and (
            (silhouette is not None and silhouette >= COHERENT_SILHOUETTE)
            or (correlation is not None and correlation >= COHERENT_CORRELATION)
        )
    )
    return {
        "members": [fact["unique_id"] for fact in members],
        "silhouette": silhouette,
        "mean_pairwise_correlation": correlation,
        "coherent": coherent,
        "seasonality_mismatch_share": mismatch,
        "outliers": outliers,
    }


def metadata_groups(
    facts: list[Fact], metadata: pd.DataFrame, raw: pd.DataFrame, vectors: np.ndarray
) -> tuple[Fact, list[Fact]]:
    summaries = {}
    groups = []
    for column, series in metadata.items():
        levels = series.dropna().unique()
        if not 1 <= len(levels) <= MAX_METADATA_LEVELS:
            continue
        labels = series.astype("string").fillna("<missing>").to_numpy()
        known = np.flatnonzero(series.notna().to_numpy())
        samples = None
        if 1 < len(levels) < len(known):
            samples = np.zeros(len(raw))
            samples[known] = silhouette_samples(vectors[known], labels[known])
        eta_squared = {}
        observed = raw.iloc[known]
        for field in observed.columns:
            values = observed[field].dropna()
            total = ((values - values.mean()) ** 2).sum()
            labels_for_values = series.reindex(values.index)
            between = sum(
                len(group) * (group.mean() - values.mean()) ** 2
                for _, group in values.groupby(labels_for_values)
            )
            eta_squared[field] = float(between / total) if total else 0.0
        summaries[column] = {
            "eta_squared": eta_squared,
            "silhouette": float(samples[known].mean()) if samples is not None else None,
        }
        for level in sorted(map(str, levels)):
            indices = np.flatnonzero(labels == level).tolist()
            if len(indices) > 1:
                groups.append(
                    {"entity": f"{column}={level}", **group_coherence(indices, facts, samples)}
                )
    return summaries, groups


def small_panel_groups(facts: list[Fact], profiles: pd.DataFrame) -> tuple[list[Fact], list[Fact]]:
    correlations = {
        (left, right): seasonal_similarity(facts[left], facts[right])
        for left, right in combinations(range(len(facts)), 2)
    }
    pairs = [
        {
            "left": facts[left]["unique_id"],
            "right": facts[right]["unique_id"],
            "correlation": correlation,
        }
        for (left, right), correlation in correlations.items()
    ]
    memberships: list[list[int]] = []
    for index in range(len(facts)):
        for members in memberships:
            if all(
                (correlations[member, index] or 0) >= COHERENT_CORRELATION for member in members
            ):
                members.append(index)
                break
        else:
            memberships.append([index])
    groups = [
        {
            "entity": f"cluster={label}",
            "size": len(indices),
            "mean_profile": profiles.iloc[indices].mean().to_dict(),
            **group_coherence(indices, facts, None),
        }
        for label, indices in enumerate(memberships)
    ]
    return groups, pairs


def profile_groups(facts: list[Fact], metadata: pd.DataFrame) -> Fact:
    if not facts:
        return {"metadata": {}, "groups": [], "clusters": [], "pairwise_correlations": []}
    ids = [fact["unique_id"] for fact in facts]
    raw = pd.DataFrame([fact["profile"] for fact in facts], index=ids)
    normalized = ((raw - raw.mean()) / raw.std(ddof=0).replace(0, 1)).fillna(0)
    for entity, row in zip(facts, normalized.to_dict("records"), strict=True):
        entity["normalized_profile"] = row
    vectors = normalized.to_numpy()
    metadata = metadata.reindex(ids)
    summaries, groups = metadata_groups(facts, metadata, raw, vectors)
    clusters = []
    pairs = []
    if len(ids) >= MIN_CLUSTER_SERIES:
        best_score = -np.inf
        best_labels = None
        best_samples = None
        unique = len(np.unique(vectors, axis=0))
        for k in range(2, min(MAX_CLUSTERS, len(ids) - 1, unique) + 1):
            labels = KMeans(n_clusters=k, random_state=0, n_init=10).fit_predict(vectors)
            if len(np.unique(labels)) < MIN_GROUP_SIZE:
                continue
            samples = silhouette_samples(vectors, labels)
            if samples.mean() > best_score:
                best_score, best_labels, best_samples = samples.mean(), labels, samples
        if best_labels is not None:
            for label in sorted(set(best_labels)):
                indices = np.flatnonzero(best_labels == label).tolist()
                clusters.append(
                    {
                        "entity": f"cluster={label}",
                        "size": len(indices),
                        "mean_profile": normalized.iloc[indices].mean().to_dict(),
                        **group_coherence(indices, facts, best_samples),
                    }
                )
    elif len(ids) >= MIN_GROUP_SIZE:
        clusters, pairs = small_panel_groups(facts, normalized)
    return {
        "metadata": summaries,
        "groups": groups,
        "clusters": clusters,
        "pairwise_correlations": pairs,
    }


def hierarchy_errors(data: pd.DataFrame, hierarchy: dict[str, list[str]]) -> list[Fact]:
    if not hierarchy:
        return []
    totals = data.assign(unique_id=data["unique_id"].astype(str)).pivot_table(
        index="ds", columns="unique_id", values="y", aggfunc="sum"
    )
    errors = []
    for parent, children in hierarchy.items():
        shared = totals[[parent, *children]].dropna()
        error = (shared[parent] - shared[children].sum(axis=1)).abs()
        limit = float(shared[parent].abs().mean() * 1e-6) if len(shared) else None
        errors.append(
            {
                "parent": parent,
                "children": children,
                "shared_dates": len(shared),
                "max_absolute_error": float(error.max()) if len(shared) else None,
                "threshold": limit,
            }
        )
    return errors


def measure_panel(data: PanelSource) -> Fact:
    config = data if isinstance(data, dict) and "data" in data else {"data": data}
    panel = read_panel(config["data"])
    if not {"unique_id", "ds", "y"} <= set(panel) or panel["unique_id"].isna().any():
        msg = "series() must supply unique_id, ds and y, with no missing series IDs"
        raise ValueError(msg)
    claims = config.get("periods", panel.attrs.get("periods", {}))
    facts = [
        measure_series(
            str(entity), group, claims.get(str(entity), []) if isinstance(claims, dict) else claims
        )
        for entity, group in panel.groupby("unique_id", sort=True)
    ]
    reserved = {
        "unique_id",
        "ds",
        "y",
        "claimed_period",
        "claimed_periods",
        "period",
        "season_length",
    }
    metadata = (
        panel.assign(unique_id=panel["unique_id"].astype(str))
        .groupby("unique_id")[sorted(set(panel) - reserved)]
        .agg(
            lambda column: (
                column.iloc[0] if column.eq(column.iloc[0]).fillna(value=False).all() else None
            )
        )
    )
    valid = [fact for fact in facts if fact["is_time_series"]]
    with threadpool_limits(limits=1):
        groups = profile_groups(valid, metadata)
    return {
        "series": facts,
        **groups,
        "hierarchy": hierarchy_errors(panel, config.get("hierarchy", {})),
    }


if __name__ == "__main__":
    try:
        adapter = Path(sys.argv[1]).resolve()
        sys.path.insert(0, str(adapter.parent))
        with redirect_stdout(sys.stderr):
            measured = measure_panel(runpy.run_path(str(adapter))["series"]())
        print(json.dumps(measured, allow_nan=False))
    except Exception as error:
        print(json.dumps({"error": f"Time series adapter failed: {error}"}))
        raise SystemExit(2) from error
