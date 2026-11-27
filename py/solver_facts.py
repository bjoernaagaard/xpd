"""Measure solver facts using the problem instances supplied by an explicit adapter."""

from __future__ import annotations

import json
import runpy
import sys
from contextlib import redirect_stdout, suppress
from datetime import date, datetime
from importlib.metadata import version
from pathlib import Path
from typing import TYPE_CHECKING, TypedDict

import numpy as np

if TYPE_CHECKING:
    from cvxpy import Problem as CvxpyProblem
    from pymoo.core.algorithm import Algorithm
    from pymoo.core.population import Population
    from pymoo.core.problem import Problem as PymooProblem


class Metric(TypedDict, total=False):
    value: object
    method: str
    reason: str
    units: str


type Facts = dict[str, Metric]
MATRIX_DIMENSIONS = 2


def metric(value: object, method: str, *, reason: str = "", units: str = "") -> Metric:
    result: Metric = {"value": value, "method": method}
    if value is None:
        result["reason"] = reason
    if units:
        result["units"] = units
    return result


def finite_array(value: object) -> np.ndarray | None:
    if value is None:
        return None
    array = np.asarray(value, dtype=float)
    return array if array.size and np.isfinite(array).all() else None


def finite_scalar(value: object) -> float | None:
    array = finite_array(value)
    return float(array.item()) if array is not None and array.size == 1 else None


def dual_value(value: object) -> object:
    if isinstance(value, (list, tuple)):
        parts = [dual_value(part) for part in value]
        return parts if parts and all(part is not None for part in parts) else None
    array = finite_array(value)
    return None if array is None else array.tolist()


def cvxpy_constraints(problem: CvxpyProblem, *, usable: bool) -> Facts:
    margins = []
    largest = 0.0
    winning_constraint = None
    missing = not usable
    for index, constraint in enumerate(problem.constraints):
        violation = None
        if usable:
            with suppress(ValueError, TypeError):
                violation = finite_array(constraint.violation())
        missing |= violation is None
        if violation is not None and (winning_constraint is None or violation.max() > largest):
            largest = float(violation.max())
            winning_constraint = index
        dual = dual_value(constraint.dual_value) if usable else None
        expression = (
            constraint.expr if type(constraint).__name__ in {"Inequality", "NonPos"} else None
        )
        slack = (
            finite_array(expression.value)
            if usable and expression is not None and expression.is_affine()
            else None
        )
        margins.append(
            {
                "constraint": index,
                "label": str(constraint),
                "signed_slack": metric(
                    None if slack is None else (-slack).tolist(),
                    "-constraint.expr.value",
                    reason="Affine inequality slack is unavailable.",
                    units="constraint units",
                ),
                "dual": metric(
                    dual,
                    "constraint.dual_value",
                    reason="The solver supplied no finite dual.",
                    units="objective units per constraint unit",
                ),
            }
        )
    return {
        "constraint-violation": metric(
            None
            if missing
            else {
                "maximum": largest,
                "constraint": winning_constraint,
                "label": None
                if winning_constraint is None
                else str(problem.constraints[winning_constraint]),
            },
            "maximum native constraint.violation() component",
            reason="A usable solution or a finite constraint component is missing.",
            units="constraint units",
        ),
        "constraint-binding-hints": metric(
            margins if usable else None,
            "CVXPY affine slack and native dual values",
            reason="No usable solution.",
        ),
    }


def cvxpy_facts(problem: CvxpyProblem, solve_kwargs: dict | None = None) -> Facts:
    from cvxpy.error import DCPError  # noqa: PLC0415

    rejection = None
    if solve_kwargs is not None:
        try:
            problem.solve(**solve_kwargs)
        except DCPError as error:
            rejection = str(error)
    stats = problem.solver_stats if rejection is None else None
    solver = stats.solver_name if stats is not None else None
    status = problem.status if rejection is None else "dcp_error"
    usable = status in {"optimal", "optimal_inaccurate"}
    gap = None
    if solver == "SCS" and usable and isinstance(stats.extra_stats, dict):
        gap = finite_scalar(stats.extra_stats.get("info", {}).get("gap"))
    details = {
        "status": status,
        "solver": solver,
        "objective": finite_scalar(problem.value) if usable else None,
    }
    if rejection is not None:
        details["error"] = rejection
    requested = solve_kwargs or {}
    return {
        "runtime-dcp": metric(
            {
                "is_dcp": problem.is_dcp(),
                "mode": "gp" if requested.get("gp") else "qcp" if requested.get("qcp") else "dcp",
            },
            "Problem.is_dcp() and requested solve mode",
        ),
        "solver-status": metric(
            details if status is not None else None,
            "Problem.status and solver_stats",
            reason="The problem has no reported solve status.",
        ),
        "duality-gap": metric(
            None if gap is None else abs(gap),
            'absolute SCS extra_stats["info"]["gap"]',
            reason="A usable SCS solution with a finite conic gap is required.",
            units="conic objective units",
        ),
        **cvxpy_constraints(problem, usable=usable),
    }


def matrix_facts(matrices: dict) -> Facts:
    conditions = []
    scales = []
    for name, entry in sorted(matrices.items()):
        descriptor = entry if isinstance(entry, dict) else {"matrix": entry}
        raw = np.asarray(descriptor.get("matrix", descriptor.get("value")))
        if raw.dtype.kind not in "iuf" or raw.ndim != MATRIX_DIMENSIONS:
            msg = f"Matrix {name} must be a non-empty finite dense numeric matrix."
            raise ValueError(msg)
        matrix = finite_array(raw)
        if matrix is None:
            msg = f"Matrix {name} must be a non-empty finite dense numeric matrix."
            raise ValueError(msg)
        identity = {
            "name": name,
            "role": descriptor.get("role"),
            "shape": list(matrix.shape),
            "units": descriptor.get("units"),
        }
        condition = finite_scalar(np.linalg.cond(matrix))
        conditions.append(
            identity
            | {
                "condition_number": metric(
                    condition,
                    "numpy.linalg.cond(matrix, p=2)",
                    reason="Singular matrix.",
                    units="dimensionless",
                ),
            }
        )
        nonzero = np.abs(matrix[matrix != 0])
        low = float(nonzero.min()) if nonzero.size else None
        high = float(nonzero.max()) if nonzero.size else None
        ratio = finite_scalar(high / low) if high is not None and low is not None else None
        scales.append(
            identity
            | {
                "min_nonzero_abs": low,
                "max_nonzero_abs": high,
                "ratio": metric(
                    ratio,
                    "max(nonzero absolute entries) / min(nonzero absolute entries)",
                    reason="All entries are zero." if low is None else "The ratio is not finite.",
                    units="dimensionless",
                ),
            }
        )
    return {
        "conditioning": metric(
            conditions or None,
            "2-norm condition numbers of declared matrices",
            reason="No matrices declared.",
        ),
        "data-scale": metric(
            scales or None,
            "nonzero absolute scale of declared matrices",
            reason="No matrices declared.",
        ),
    }


def index_value(value: object) -> object:
    if isinstance(value, (datetime, date)):
        return value.isoformat()
    if isinstance(value, np.datetime64):
        return np.datetime_as_string(value)
    return value.item() if isinstance(value, np.generic) else value


def fold_cutoff(run: dict) -> dict:
    fit = list(run["fit_indices"])
    evaluation = list(run.get("evaluation_indices", []))
    fit_end = max(fit) if fit else None
    evaluation_start = min(evaluation) if evaluation else None
    ordered = None
    if fit_end is not None and evaluation_start is not None:
        if isinstance(fit_end, (int, np.integer)) and isinstance(
            evaluation_start, (int, np.integer)
        ):
            ordered = bool(fit_end < evaluation_start)
        elif isinstance(fit_end, (date, np.datetime64)) and isinstance(
            evaluation_start, (date, np.datetime64)
        ):
            with suppress(TypeError):
                ordered = bool(fit_end < evaluation_start)
    return {
        "fit_end": index_value(fit_end),
        "evaluation_start": index_value(evaluation_start),
        "fit_count": len(fit),
        "evaluation_count": len(evaluation),
        "ordered": metric(
            ordered,
            "latest fit index < earliest evaluation index",
            reason="No comparable fit and evaluation boundaries.",
        ),
    }


def skfolio_facts(runs: list[dict]) -> Facts:
    cutoffs = []
    solved = []
    for run in runs:
        cutoffs.append(fold_cutoff(run))
        estimator = run["estimator"]
        if estimator.get_params(deep=False).get("save_problem") is not None:
            estimator.set_params(save_problem=True)
        evidence = {}
        try:
            estimator.fit(*run["fit_args"], **run.get("fit_kwargs", {}))
        except Exception as error:  # noqa: BLE001
            evidence["fit_error"] = f"{type(error).__name__}: {error}"
        errors = getattr(estimator, "error_", None)
        if isinstance(errors, (list, tuple)):
            evidence["failed_portfolios"] = [
                {"index": index, "error": str(error)}
                for index, error in enumerate(errors)
                if error is not None
            ]
        saved = getattr(estimator, "problem_", None)
        if saved is None:
            evidence["problem"] = metric(
                None, "estimator.problem_", reason="The fitted estimator exposes no saved problem."
            )
        else:
            evidence.update(cvxpy_facts(saved))
        solved.append(evidence)
    return {
        "fold-cutoff": metric(
            cutoffs or None,
            "adapter-declared fit and evaluation indices",
            reason="No runs supplied.",
        ),
        "skfolio-saved-problem": metric(
            solved or None, "public problem_ after one fit per run", reason="No runs supplied."
        ),
    }


def front_checkpoint(
    population: Population, reference: np.ndarray | None, scales: np.ndarray
) -> dict:
    from pymoo.indicators.hv import HV  # noqa: PLC0415
    from pymoo.util.nds.non_dominated_sorting import NonDominatedSorting  # noqa: PLC0415

    count = 0
    value = None
    reason = "There are no feasible candidates."
    if population is not None and len(population):
        objectives = np.asarray(population.get("F"), dtype=float)
        feasible = np.asarray(population.get("FEAS"), dtype=bool).reshape(-1)
        points = objectives[feasible] / scales
        if not np.isfinite(points).all():
            msg = "The feasible front contains non-finite objectives."
            raise ValueError(msg)
        if len(points):
            points = points[NonDominatedSorting().do(points, only_non_dominated_front=True)]
            count = len(points)
            if reference is None:
                reason = "Hypervolume requires multiple objectives and a declared reference."
            elif np.any(reference / scales <= points):
                reason = "The reference point must be strictly worse than each included point."
            else:
                value = finite_scalar(HV(ref_point=reference / scales)(points))
                reason = "The computed hypervolume is not finite."
    return {
        "feasible_count": count,
        "hypervolume": metric(
            value,
            "feasible nondominated minimization front against fixed scaled reference",
            reason=reason,
            units="product of scaled objective units",
        ),
    }


def pymoo_residuals(problem: PymooProblem, population: Population | None) -> Facts:
    maximum = None
    margins = []
    if population is not None and len(population):
        for candidate, individual in enumerate(population):
            components = [
                ("inequality", np.maximum(0, individual.G), individual.G),
                ("equality", np.abs(individual.H), individual.H),
            ]
            if problem.xl is not None:
                lower = problem.xl - individual.X
                components.append(("lower-bound", np.maximum(0, lower), lower))
            if problem.xu is not None:
                upper = individual.X - problem.xu
                components.append(("upper-bound", np.maximum(0, upper), upper))
            for kind, residuals, raw in components:
                for index, (residual, signed) in enumerate(zip(residuals, raw, strict=True)):
                    value = finite_scalar(residual)
                    if value is None:
                        msg = "A pymoo residual is not finite."
                        raise ValueError(msg)
                    location = {"candidate": candidate, "kind": kind, "constraint": index}
                    margins.append(location | {"margin": float(signed)})
                    if maximum is None or value > maximum["maximum"]:
                        maximum = location | {"maximum": value}
    return {
        "pymoo-constraint-violation": metric(
            maximum,
            "max(0,G), abs(H), and candidate bound excess",
            reason="No returned candidates or residual components.",
            units="constraint units",
        ),
        "constraint-binding-hints": metric(
            margins or None,
            "raw margins from candidate constraints and bounds",
            reason="No returned margins.",
            units="constraint units",
        ),
    }


def pymoo_facts(descriptor: dict) -> Facts:
    from pymoo.core.callback import Callback  # noqa: PLC0415
    from pymoo.optimize import minimize  # noqa: PLC0415

    problem = descriptor["problem"]
    scales = np.asarray(descriptor.get("objective_scales", np.ones(problem.n_obj)), dtype=float)
    if scales.shape != (problem.n_obj,) or not np.isfinite(scales).all() or np.any(scales <= 0):
        msg = "Objective scales must be finite positive factors for each objective."
        raise ValueError(msg)
    reference = finite_array(descriptor.get("reference_point"))
    if problem.n_obj > 1 and (reference is None or reference.shape != scales.shape):
        msg = "A multiobjective run requires a finite reference point with one value per objective."
        raise ValueError(msg)
    checkpoints = set(descriptor.get("checkpoints", []))
    trace = []

    class FrontTrace(Callback):
        def notify(self, algorithm: Algorithm) -> None:
            if algorithm.n_gen in checkpoints:
                snapshot = front_checkpoint(
                    algorithm.opt, reference if problem.n_obj > 1 else None, scales
                )
                trace.append(
                    {
                        "generation": algorithm.n_gen,
                        "evaluations": algorithm.evaluator.n_eval,
                        **snapshot,
                    }
                )

    result = minimize(
        problem,
        descriptor["algorithm"],
        termination=descriptor["termination"],
        seed=descriptor["seed"],
        callback=FrontTrace(),
        return_least_infeasible=True,
    )
    candidates = result.opt
    feasible = (
        [bool(individual.FEAS.item()) for individual in candidates]
        if candidates is not None
        else []
    )
    return {
        "pymoo-status": metric(
            {
                "status": "completed-with-feasible-front"
                if any(feasible)
                else "completed-without-feasible-front",
                "feasible": feasible,
                "evaluations": result.algorithm.evaluator.n_eval,
                "seed": descriptor["seed"],
                "termination": descriptor["termination"],
            },
            "pymoo minimize result",
        ),
        "front-convergence": metric(
            {
                "checkpoints": trace,
                "reference_point": reference.tolist(),
                "objective_scales": scales.tolist(),
            }
            if problem.n_obj > 1 and reference is not None
            else None,
            "pymoo Callback on algorithm.opt filtered by FEAS",
            reason="Single-objective problem.",
        ),
        "duality-gap": metric(
            None, "pymoo population search", reason="The algorithm supplies no dual certificate."
        ),
        **pymoo_residuals(problem, candidates),
    }


def measure_problem(problem_id: str, descriptor: dict) -> dict:
    library = descriptor["library"]
    entry = {"id": problem_id, "library": library}
    if reason := descriptor.get("unmeasured"):
        return entry | {"unmeasured": reason}
    packages = {library, "numpy"}
    if library == "cvxpy":
        facts = cvxpy_facts(descriptor["problem"], descriptor.get("solve_kwargs", {}))
    elif library == "skfolio":
        packages.add("cvxpy")
        facts = skfolio_facts(descriptor["runs"])
    elif library == "pymoo":
        facts = pymoo_facts(descriptor)
    else:
        msg = f"Unsupported solver library {library!r}."
        raise ValueError(msg)
    if "SCS" in json.dumps(facts, allow_nan=False):
        packages.add("scs")
    facts.update(matrix_facts(descriptor.get("matrices", {})))
    return entry | {
        "facts": facts,
        "versions": {package: version(package) for package in sorted(packages)},
    }


def main(adapter: Path) -> None:
    sys.path.insert(0, str(adapter.resolve().parent))
    sys.path.insert(0, str(Path.cwd()))
    with redirect_stdout(sys.stderr):
        namespace = runpy.run_path(str(adapter), run_name="__xpd_opt_review__")
        problems = namespace["problems"]()
        entries = [
            measure_problem(problem_id, descriptor)
            for problem_id, descriptor in sorted(problems.items())
        ]
    print(json.dumps({"problems": entries}, allow_nan=False))


if __name__ == "__main__":
    main(Path(sys.argv[1]))
