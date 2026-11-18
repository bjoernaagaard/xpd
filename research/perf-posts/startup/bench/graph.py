"""Keep the cumulative importtime rows separate from the outer process timing."""

import re
import subprocess
import sys
from pathlib import Path

kind, label, filename = sys.argv[1:]
out = Path(filename)
if kind == "module":
    child = subprocess.run(
        [sys.executable, "-X", "importtime", "-c", f"import {label},sys;print(len(sys.modules))"],
        capture_output=True,
        text=True,
        check=True,
    )
    line = next(line for line in child.stderr.splitlines() if line.endswith("| " + label))
    evidence = f"{int(line.split('|')[1])} {int(child.stdout)}"
else:
    statement = "import " + label.replace("_first", "")
    if label == "asyncio_first":
        statement += ", logging"
    else:
        statement += ", asyncio"
    child = subprocess.run(
        [sys.executable, "-X", "importtime", "-c", statement],
        capture_output=True,
        text=True,
        check=True,
    )
    # A top level import has one space after the final pipe. Children are indented.
    rows = dict(
        (name, int(time))
        for time, name in re.findall(
            r"^import time:\s*\d+\s*\|\s*(\d+)\s*\| (asyncio|logging)$", child.stderr, re.MULTILINE
        )
    )
    evidence = f"{rows['asyncio']} {rows.get('logging', 0)}"
with out.open("a") as stream:
    print(evidence, file=stream)
