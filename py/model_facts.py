"""Measure validation folds with the project's Python environment."""

import json
import statistics
import sys
from collections.abc import Callable
from contextlib import redirect_stdout
from dataclasses import dataclass
from datetime import UTC, datetime, timedelta
from importlib.util import module_from_spec, spec_from_file_location
from itertools import combinations
from math import isfinite
from numbers import Integral
from pathlib import Path
from typing import Any, Protocol

import numpy as np
from sklearn.base import clone, is_classifier
from sklearn.dummy import DummyClassifier, DummyRegressor
from sklearn.metrics import get_scorer

ROLES = ("fit", "early_stopping", "calibration", "validation")


class ModelAdapterError(ValueError):
    """The adapter does not describe a valid model evaluation."""


def unavailable(reason: str) -> dict:
    return {"status": "unavailable", "reason": reason}


class IndexableRows(Protocol):
    def __getitem__(self, key: np.ndarray) -> "IndexableRows": ...


class ProbabilityEstimator(Protocol):
    classes_: np.ndarray

    def predict_proba(self, features: IndexableRows) -> np.ndarray: ...


class IntervalEstimator(Protocol):
    def predict_interval(self, features: IndexableRows) -> tuple[np.ndarray, np.ndarray]: ...


def row_subset(data: IndexableRows, rows: np.ndarray) -> IndexableRows:
    if hasattr(data, "iloc"):
        return data.iloc[rows]
    if isinstance(data, (tuple, list)):
        return np.asarray(data)[rows]
    return data[rows]


def fold_roles(fold: dict, size: int) -> dict[str, np.ndarray]:
    roles = {}
    seen = set()
    for role in ROLES:
        rows = list(fold.get(role, ()))
        if not rows and role in {"fit", "validation"}:
            msg = f"Fold {fold['id']} needs a nonempty {role} partition."
            raise ModelAdapterError(msg)
        if any(
            not isinstance(row, Integral)
            or isinstance(row, (bool, np.bool_))
            or not 0 <= row < size
            for row in rows
        ):
            msg = f"Fold {fold['id']} has invalid positional indices in {role}."
            raise ModelAdapterError(msg)
        unique = set(rows)
        if len(unique) != len(rows) or seen.intersection(unique):
            msg = f"Fold {fold['id']} has overlapping or duplicate rows in {role}."
            raise ModelAdapterError(msg)
        seen.update(unique)
        roles[role] = np.asarray(rows, dtype=np.intp)
    return roles


def as_utc(value: str | datetime | np.datetime64) -> datetime:
    if isinstance(value, np.datetime64):
        value = datetime.fromisoformat(str(value)).replace(tzinfo=UTC)
    timestamp = value if isinstance(value, datetime) else datetime.fromisoformat(value)
    if timestamp.utcoffset() != timedelta(0):
        msg = "Model times must be UTC timestamps."
        raise ModelAdapterError(msg)
    return timestamp


def partition_evidence(
    roles: dict[str, np.ndarray], groups: list | None, times: list | None
) -> dict:
    active = {role: rows for role, rows in roles.items() if len(rows)}
    conflicts = []
    group_evidence = unavailable("No group IDs supplied.")
    time_evidence = unavailable("No UTC times supplied.")
    if groups is not None:
        members = {role: {groups[row] for row in rows} for role, rows in active.items()}
        intersections = {}
        for left, right in combinations(active, 2):
            count = len(members[left] & members[right])
            intersections[f"{left}/{right}"] = count
            if count:
                conflicts.append(f"{left} and {right} share {count} groups.")
        group_evidence = {"status": "measured", "intersections": intersections}
    if times is not None:
        boundaries = {}
        for role, rows in active.items():
            first = min(rows, key=lambda row: (times[row], row))
            last = max(rows, key=lambda row: (times[row], row))
            boundaries[role] = {
                "first_row": int(first),
                "last_row": int(last),
                "first_time": times[first].isoformat(),
                "last_time": times[last].isoformat(),
            }
        for left, right in combinations(active, 2):
            if times[boundaries[left]["last_row"]] >= times[boundaries[right]["first_row"]]:
                conflicts.append(f"{left} does not end before {right} starts.")
        time_evidence = {"status": "measured", "boundaries": boundaries}
    return {
        "counts": {role: len(rows) for role, rows in roles.items()},
        "groups": group_evidence,
        "times": time_evidence,
        "conflicts": conflicts,
    }


def probability_events(
    model: dict, fitted: ProbabilityEstimator, features: IndexableRows, y: np.ndarray
) -> tuple | str:
    if not hasattr(fitted, "predict_proba"):
        return "The estimator does not predict probabilities."
    classes = np.asarray(fitted.classes_)
    if classes.shape != (2,):
        return "Probability calibration requires exactly two classes."
    if "positive_label" not in model:
        return "No positive label was declared."
    label = model["positive_label"]
    column = np.flatnonzero(classes == label)
    actual = np.asarray(y)
    if len(column) != 1 or actual.shape != (len(y),) or not np.isin(actual, classes).all():
        msg = "The positive label and validation labels must belong to the fitted classes."
        raise ModelAdapterError(msg)
    probabilities = np.asarray(fitted.predict_proba(features), dtype=float)
    if (
        probabilities.shape != (len(y), 2)
        or not np.isfinite(probabilities).all()
        or (probabilities < 0).any()
        or (probabilities > 1).any()
    ):
        msg = "Expected one finite binary probability pair in [0, 1] per validation row."
        raise ModelAdapterError(msg)
    return probabilities[:, column[0]], actual == label


def pooled_probabilities(events: list[tuple | str]) -> dict:
    for item in events:
        if isinstance(item, str):
            return unavailable(item)
    probability = np.concatenate([item[0] for item in events])
    observed = np.concatenate([item[1] for item in events])
    membership = np.minimum((5 * probability).astype(int), 4)
    bins = []
    for index in range(5):
        selected = membership == index
        count = int(selected.sum())
        bins.append(
            {
                "lower": index / 5,
                "upper": (index + 1) / 5,
                "count": count,
                "mean_probability": float(probability[selected].mean()) if count else None,
                "observed_rate": float(observed[selected].mean()) if count else None,
            }
        )
    return {
        "status": "measured",
        "count": len(probability),
        "weighting": "Each validation prediction is one event, including repeated rows.",
        "brier_loss": float(np.mean(np.square(probability - observed))),
        "bins": bins,
    }


def coverage_record(level: float, covered: int, eligible: int, width: float | None) -> dict:
    return {
        "confidence_level": level,
        "covered": covered,
        "eligible": eligible,
        "coverage": covered / eligible,
        "coverage_minus_target": covered / eligible - level,
        "mean_width": width,
    }


def fold_intervals(
    conformalizer: IntervalEstimator, levels: list[float], features: IndexableRows, y: np.ndarray
) -> list[dict]:
    _, predictions = conformalizer.predict_interval(features)
    bounds = np.asarray(predictions, dtype=float)
    actual = np.asarray(y, dtype=float)
    if bounds.shape != (len(y), 2, len(levels)) or actual.shape != (len(y),):
        msg = "Interval bounds must have shape (validation rows, 2, levels)."
        raise ModelAdapterError(msg)
    lower, upper = bounds[:, 0, :], bounds[:, 1, :]
    if (
        np.isnan(bounds).any()
        or not np.isfinite(actual).all()
        or (lower > upper).any()
        or np.isposinf(lower).any()
        or np.isneginf(upper).any()
    ):
        msg = "Interval bounds must be ordered and may only be unbounded outward."
        raise ModelAdapterError(msg)
    evidence = []
    for index, level in enumerate(levels):
        left, right = lower[:, index], upper[:, index]
        covered = int(((left <= actual) & (actual <= right)).sum())
        width = float(np.mean(right - left))
        evidence.append(coverage_record(level, covered, len(y), width if isfinite(width) else None))
    return evidence


def pooled_intervals(folds: list[dict], levels: list[float]) -> dict:
    if not levels:
        return unavailable("No conformal interval function was supplied.")
    pooled = []
    for index, level in enumerate(levels):
        records = [fold["interval_coverage"][index] for fold in folds]
        eligible = sum(item["eligible"] for item in records)
        covered = sum(item["covered"] for item in records)
        width = None
        if all(item["mean_width"] is not None for item in records):
            width = sum(item["mean_width"] * (item["eligible"] / eligible) for item in records)
        pooled.append(coverage_record(level, covered, eligible, width))
    return {"status": "measured", "levels": pooled, "weighting": "Pooled validation events."}


@dataclass(frozen=True)
class FoldSettings:
    scorer: Callable
    baseline: Any
    groups: list | None
    times: list[datetime] | None
    levels: list[float]


def evaluate_fold(model: dict, fold: dict, settings: FoldSettings) -> tuple[dict, tuple | str]:
    features, y = model["X"], model["y"]
    roles = fold_roles(fold, len(y))
    partitions = partition_evidence(roles, settings.groups, settings.times)
    rows = {
        role: (row_subset(features, indices), row_subset(y, indices))
        if len(indices)
        else (None, None)
        for role, indices in roles.items()
    }
    fitted = clone(model["estimator"])
    if fit_fold := model.get("fit_fold"):
        fitted = fit_fold(fitted, *rows["fit"], *rows["early_stopping"])
    else:
        if len(roles["early_stopping"]):
            msg = "An early-stopping partition needs a fit_fold callback."
            raise ModelAdapterError(msg)
        fitted.fit(*rows["fit"])
    fitted_baseline = clone(settings.baseline).fit(*rows["fit"])
    train = float(settings.scorer(fitted, *rows["fit"]))
    validation = float(settings.scorer(fitted, *rows["validation"]))
    reference = float(settings.scorer(fitted_baseline, *rows["validation"]))
    if not all(isfinite(value) for value in (train, validation, reference)):
        msg = "Model scores must be finite."
        raise ModelAdapterError(msg)
    evidence = {
        "id": fold["id"],
        "partitions": partitions,
        "valid": not partitions["conflicts"],
        "train_score": train,
        "validation_score": validation,
        "baseline_score": reference,
        "validation_minus_baseline": validation - reference,
        "train_minus_validation": train - validation,
    }
    events = probability_events(model, fitted, *rows["validation"])
    if conformalize := model.get("conformalize"):
        if not len(roles["calibration"]):
            msg = "Intervals require a separate calibration partition."
            raise ModelAdapterError(msg)
        conformalizer = conformalize(fitted, *rows["calibration"])
        evidence["interval_coverage"] = fold_intervals(
            conformalizer, settings.levels, *rows["validation"]
        )
    return evidence, events


def measure_model(model: dict) -> dict:
    estimator = model["estimator"]
    baseline = model.get("baseline")
    if baseline is None:
        baseline = (
            DummyClassifier(strategy="prior")
            if is_classifier(estimator)
            else DummyRegressor(strategy="mean")
        )
    size = len(model["y"])
    shape = getattr(model["X"], "shape", None)
    if (shape[0] if shape is not None else len(model["X"])) != size:
        msg = "X and y must contain the same number of rows."
        raise ModelAdapterError(msg)
    groups = list(model["groups"]) if model.get("groups") is not None else None
    times = [as_utc(value) for value in model["times"]] if model.get("times") is not None else None
    if any(values is not None and len(values) != size for values in (groups, times)):
        msg = "Group IDs and UTC times must be row-aligned."
        raise ModelAdapterError(msg)
    scorer = get_scorer(model["scorer"])
    levels = [float(level) for level in model.get("confidence_levels", ())]
    if (
        bool(model.get("conformalize")) != bool(levels)
        or len(set(levels)) != len(levels)
        or any(not 0 < level < 1 for level in levels)
    ):
        msg = "Provide distinct levels in (0, 1) and a conformalize callback."
        raise ModelAdapterError(msg)
    settings = FoldSettings(scorer, baseline, groups, times, levels)
    folds = []
    events = []
    seen = set()
    for fold in model["folds"]:
        label = fold["id"]
        if not isinstance(label, str) or not label.strip() or label in seen:
            msg = "Fold IDs must be unique nonempty strings."
            raise ModelAdapterError(msg)
        seen.add(label)
        evidence, predictions = evaluate_fold(model, fold, settings)
        folds.append(evidence)
        events.append(predictions)
    if not folds:
        msg = "Each adapter model must declare at least one development fold."
        raise ModelAdapterError(msg)
    scores = [fold["validation_score"] for fold in folds]
    spread = {"mean": statistics.mean(scores), "validation_scores": scores}
    if len(scores) > 1:
        spread.update(
            status="measured",
            standard_deviation=statistics.pstdev(scores),
            range=max(scores) - min(scores),
        )
    else:
        spread.update(unavailable("One validation fold has no between-fold comparison."))
    return {
        "file": model["file"],
        "binding": model["binding"],
        "splitter": model["splitter"],
        "scorer": model["scorer"],
        "baseline": str(baseline),
        "positive_label": model.get("positive_label"),
        "valid": all(fold["valid"] for fold in folds),
        "folds": folds,
        "spread": spread,
        "probability_calibration": pooled_probabilities(events),
        "interval_coverage": pooled_intervals(folds, levels),
    }


def python_scalar(value: object) -> object:
    if isinstance(value, np.generic):
        return value.item()
    msg = f"Cannot serialize {type(value).__name__} as model evidence."
    raise ModelAdapterError(msg)


def main(path: Path) -> None:
    path = path.resolve()
    sys.dont_write_bytecode = True
    sys.path[:0] = [str(path.parent), str(Path.cwd())]
    spec = spec_from_file_location("xpd_model_adapter", path)
    if spec is None or spec.loader is None:
        msg = f"Cannot load Python adapter {path}."
        raise ModelAdapterError(msg)
    module = module_from_spec(spec)
    sys.modules[spec.name] = module
    with redirect_stdout(sys.stderr):
        spec.loader.exec_module(module)
        result = {"models": [measure_model(model) for model in module.models()]}
    print(json.dumps(result, allow_nan=False, default=python_scalar))


if __name__ == "__main__":
    try:
        main(Path(sys.argv[1]))
    except ModelAdapterError as error:
        print(json.dumps({"error": str(error)}))
        sys.exit(2)
