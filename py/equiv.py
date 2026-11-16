# Embedded in the xpd binary (include_str!) and run with the project's Python.
# Usage: python -c <this> OLD.py NEW.py
# Contract: each script must leave its output in a variable named `result`. Each script runs in
# its own Python process, started in the same folder, so the second one gets no module, no
# sys.path entry and no global from the first. What a script prints goes to stderr.
# DataFrames and Series are compared with pandas.testing, anything else with
# numpy.testing, so arrays compare element-wise and `python -O` can't skip the check.
# Prints JSON; exit 0 = equivalent, 1 = differs, 2 = bad input or xpd can't run.
import json
import pickle
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import NoReturn

# What each process runs. It leaves `result`, or the reason there is none, in a file. A pickle
# carries any result the old single process could compare, and this script reads only a file
# that its own child wrote.
RUNNER = """
import pickle
import runpy
import sys

script, left = sys.argv[1:]
try:
    ns = runpy.run_path(script, run_name="__xpd_equiv__")
    found = ("result", ns["result"]) if "result" in ns else ("error", "defines no `result`")
except Exception as e:
    found = ("error", f"raised {type(e).__name__}: {e}")
try:
    data = pickle.dumps(found)
except Exception as e:
    data = pickle.dumps(("error", f"leaves a `result` that pickle cannot save. {e}"))
with open(left, "wb") as file:
    file.write(data)
"""


def fail(error: str) -> NoReturn:
    print(json.dumps({"equivalent": None, "error": error}))
    sys.exit(2)


try:
    import pandas as pd
    from numpy import testing as npt
    from pandas import testing as pdt
except ImportError as e:
    fail(f"xpd equiv needs pandas in the project's Python. {e}")


def run(path: str, left: Path) -> object:
    done = subprocess.run(  # noqa: S603  the user asked xpd to run this script
        [sys.executable, "-c", RUNNER, path, str(left)],
        stdin=subprocess.DEVNULL,
        stdout=sys.stderr,
        check=False,
    )
    if not left.is_file():
        fail(f"{path} exited with code {done.returncode} and left no `result`")
    try:
        kind, value = pickle.loads(left.read_bytes())  # noqa: S301  written by the process above
    except Exception as e:  # noqa: BLE001  a class that only the script's process can import
        fail(f"{path} leaves a `result` that only its own process can load. {e}")
    if kind == "error":
        fail(f"{path} {value}")
    return value


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="xpd-equiv-") as folder:
        old = run(sys.argv[1], Path(folder, "old"))
        new = run(sys.argv[2], Path(folder, "new"))
    try:
        if isinstance(old, pd.DataFrame):
            pdt.assert_frame_equal(old, new)
        elif isinstance(old, pd.Series):
            pdt.assert_series_equal(old, new)
        else:
            npt.assert_equal(new, old)
    except AssertionError as e:
        print(json.dumps({"equivalent": False, "detail": str(e)}))
        sys.exit(1)
    print(json.dumps({"equivalent": True}))


if __name__ == "__main__":
    main()
