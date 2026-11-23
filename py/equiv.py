"""Compare the outputs of scripts executed in separate processes."""

from __future__ import annotations

import json
import math
import pickle
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import TYPE_CHECKING, NamedTuple, NoReturn

if TYPE_CHECKING:
    import polars as pl

# What each process runs. It leaves `result`, or the reason there is none, in a file. A pickle
# carries any result the old single process could compare, and this script reads only a file
# that its own child wrote.
RUNNER = """
from pathlib import Path
import pickle
import runpy
import sys

script, output = sys.argv[1:]
sys.path.insert(0, str(Path(script).resolve().parent))
try:
    ns = runpy.run_path(script, run_name="__xpd_equiv__")
    found = ("result", ns["result"]) if "result" in ns else ("error", "defines no `result`")
except Exception as e:
    found = ("error", f"raised {type(e).__name__}: {e}")
try:
    data = pickle.dumps(found)
except Exception as e:
    data = pickle.dumps(("error", f"leaves a `result` that pickle cannot save. {e}"))
Path(output).write_bytes(data)
"""


def fail(error: str) -> NoReturn:
    print(json.dumps({"equivalent": None, "error": error}))
    sys.exit(2)


try:
    import numpy as np
    import pandas as pd
    from numpy import testing as npt
    from pandas import testing as pdt
except ImportError as e:
    fail(f"xpd equiv needs pandas in the project's Python. {e}")


def run(path: str, output: Path) -> object:
    try:
        done = subprocess.run(  # noqa: S603
            [sys.executable, "-c", RUNNER, path, str(output)],
            stdin=subprocess.DEVNULL,
            stdout=sys.stderr,
            check=False,
        )
    except OSError as e:
        fail(f"cannot start Python for {path}. {e}")
    if not output.is_file():
        fail(f"{path} exited with code {done.returncode} and left no `result`")
    if done.returncode != 0:
        fail(f"{path} exited with code {done.returncode}")
    try:
        raw = output.read_bytes()
    except OSError as e:
        fail(f"cannot read result for {path}. {e}")
    try:
        kind, value = pickle.loads(raw)  # noqa: S301
    except Exception as e:  # noqa: BLE001
        fail(f"{path} leaves a `result` that only its own process can load. {e}")
    if kind == "error":
        fail(f"{path} {value}")
    return value


SUPPORTED_TYPES = {"Boolean", "Float64", "Int64", "String"}
CONTRACT_FIELDS = {
    "input_schema",
    "output_schema",
    "row_identity",
    "materialization",
    "float_atol",
    "missing_float_columns",
}


class Contract(NamedTuple):
    output_schema: dict[str, str]
    float_atol: dict[str, float]
    missing_float_columns: list[str]


class UnsupportedResultError(Exception):
    """A result that cannot be compared under the positional table contract."""


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    """Keep JSON object keys unique so a later value cannot override the contract."""
    fields = dict(pairs)
    if len(fields) != len(pairs):
        message = "JSON objects must have unique keys"
        raise ValueError(message)
    return fields


def schema_fields(field: str, value: object) -> dict[str, str]:
    """Read a schema in the order given by its array."""
    if not isinstance(value, list):
        message = f"{field} must be an array of column definitions"
        raise TypeError(message)
    schema: dict[str, str] = {}
    for column in value:
        if not isinstance(column, dict) or column.keys() != {"name", "dtype"}:
            message = f"{field} entries must contain only name and dtype"
            raise ValueError(message)
        name, dtype = column["name"], column["dtype"]
        if not isinstance(name, str) or not name or name in schema:
            message = f"{field} names must be nonempty strings and unique"
            raise ValueError(message)
        if not isinstance(dtype, str) or dtype not in SUPPORTED_TYPES:
            message = f"{field} has unsupported dtype {dtype!r} for {name!r}"
            raise ValueError(message)
        schema[name] = dtype
    return schema


def read_contract(path: Path) -> Contract:
    """Load the ordered output schema and the comparison policy."""
    data = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object)
    if not isinstance(data, dict) or data.keys() - CONTRACT_FIELDS:
        message = "expected a contract object with only supported fields"
        raise ValueError(message)
    if data.get("row_identity") != "position":
        message = "row_identity must be position"
        raise ValueError(message)
    if data.get("materialization") not in ("eager", "lazy"):
        message = "materialization must be eager or lazy"
        raise ValueError(message)

    schema_fields("input_schema", data.get("input_schema"))
    schema = schema_fields("output_schema", data.get("output_schema"))
    float_columns = {name for name, dtype in schema.items() if dtype == "Float64"}
    tolerances = data.get("float_atol", {})
    if not isinstance(tolerances, dict) or tolerances.keys() - float_columns:
        message = "float_atol must be an object keyed by Float64 output column"
        raise ValueError(message)
    for name, tolerance in tolerances.items():
        if (
            isinstance(tolerance, bool)
            or not isinstance(tolerance, (int, float))
            or not math.isfinite(tolerance)
            or tolerance < 0
        ):
            message = f"float_atol for {name!r} must be finite and nonnegative"
            raise ValueError(message)

    missing = data.get("missing_float_columns", [])
    if not isinstance(missing, list):
        message = "missing_float_columns must be an array"
        raise TypeError(message)
    if any(not isinstance(name, str) or name not in float_columns for name in missing):
        message = "missing_float_columns must reference only Float64 output columns"
        raise ValueError(message)
    if len(set(missing)) != len(missing):
        message = "missing_float_columns entries must be unique"
        raise ValueError(message)
    return Contract(schema, tolerances, missing)


PANDAS_DTYPES = {
    "Boolean": {"bool", "boolean", "bool[pyarrow]"},
    "Float64": {"float64", "Float64", "double[pyarrow]"},
    "Int64": {"int64", "Int64", "int64[pyarrow]"},
}


def has_expected_dtype(column: pd.Series, expected: str) -> bool:
    """Recognize equivalent pandas representations without coercing values."""
    if expected != "String":
        return str(column.dtype) in PANDAS_DTYPES[expected]
    if isinstance(column.dtype, pd.StringDtype) or str(column.dtype) in {
        "string[pyarrow]",
        "large_string[pyarrow]",
    }:
        return True
    return pd.api.types.infer_dtype(column) == "string"


def compare_column(old: pd.Series, new: pl.Series, expected: str, contract: Contract) -> None:
    """Compare columns without converting nullable integers through floats.

    Polars converts a nullable integer column to floats when exported to NumPy,
    which rounds large integers before comparison.
    """
    if not has_expected_dtype(old, expected):
        message = f"pandas column {new.name!r} has dtype {old.dtype}, expected {expected}"
        raise AssertionError(message)

    label = f"column {new.name!r}"
    if expected == "Float64":
        if new.name not in contract.missing_float_columns:
            old_nulls = (
                np.zeros(len(old), dtype=bool)
                if old.dtype == np.dtype("float64")
                else old.isna().to_numpy()
            )
            npt.assert_array_equal(old_nulls, new.is_null().to_numpy(), err_msg=f"{label} nulls")
        npt.assert_allclose(
            new.to_numpy(),
            old.to_numpy(dtype="float64", na_value=np.nan),
            rtol=0,
            atol=contract.float_atol.get(new.name, 0),
            equal_nan=True,
            err_msg=label,
        )
        return

    dtype = {"Boolean": "boolean", "String": "string", "Int64": "Int64"}[expected]
    native_values = pd.Series(new.to_list(), dtype=dtype, name=old.name)
    pdt.assert_series_equal(old.astype(dtype), native_values, check_exact=True)


def compare_contract(old: object, new: object, contract: Contract) -> None:
    """Check native Polars output against an ordered pandas result."""
    import polars as pl  # noqa: PLC0415

    if not isinstance(old, pd.DataFrame) or not isinstance(new, pl.DataFrame):
        message = "the contract requires a pandas DataFrame and a native Polars DataFrame"
        raise UnsupportedResultError(message)
    if not old.index.identical(pd.RangeIndex(len(old))):
        message = "the pandas output must have an unnamed default RangeIndex"
        raise UnsupportedResultError(message)

    actual_schema = [(name, str(dtype)) for name, dtype in new.schema.items()]
    expected_schema = list(contract.output_schema.items())
    if actual_schema != expected_schema:
        message = f"Polars schema: expected {expected_schema}, found {actual_schema}"
        raise AssertionError(message)
    if list(old.columns) != list(contract.output_schema):
        message = (
            f"pandas columns: expected {list(contract.output_schema)}, found {list(old.columns)}"
        )
        raise AssertionError(message)
    if len(old) != new.height:
        message = f"row count: pandas has {len(old)}, Polars has {new.height}"
        raise AssertionError(message)
    for name, expected in contract.output_schema.items():
        compare_column(old[name], new[name], expected, contract)


def explicit_contract(path: str) -> Contract:
    try:
        return read_contract(Path(path))
    except (OSError, TypeError, ValueError, OverflowError) as e:
        fail(f"cannot read comparison contract. {e}")


def main() -> None:
    old_path, new_path, *contract_path = sys.argv[1:]
    contract = explicit_contract(contract_path[0]) if contract_path else None

    try:
        with tempfile.TemporaryDirectory(prefix="xpd-equiv-") as folder:
            old = run(old_path, Path(folder, "old"))
            new = run(new_path, Path(folder, "new"))
    except OSError as e:
        fail(f"cannot prepare equivalence workspace. {e}")
    report: dict[str, object] = {}
    if contract is not None:
        report["comparison"] = {
            "row_identity": "position",
            "float_atol": contract.float_atol,
            "missing_float_columns": contract.missing_float_columns,
        }
    try:
        if contract is not None:
            compare_contract(old, new, contract)
        elif isinstance(old, pd.DataFrame):
            pdt.assert_frame_equal(old, new)
        elif isinstance(old, pd.Series):
            pdt.assert_series_equal(old, new)
        else:
            npt.assert_equal(new, old)
    except UnsupportedResultError as e:
        report.update(equivalent=None, status="unsupported", error=str(e))
        print(json.dumps(report))
        sys.exit(3)
    except AssertionError as e:
        report.update(equivalent=False, detail=str(e))
        print(json.dumps(report))
        sys.exit(1)
    except Exception as e:  # noqa: BLE001  numpy raises its own errors on values it cannot compare
        fail(f"cannot compare the two results. {type(e).__name__}: {e}")
    report["equivalent"] = True
    print(json.dumps(report))


if __name__ == "__main__":
    main()
