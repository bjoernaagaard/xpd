"""Sample cumulative import time, including imports triggered by the target."""

import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

module, cache, output = sys.argv[1:]
work = Path(output).parent
with tempfile.TemporaryDirectory(dir=work) as empty:
    settings = os.environ.copy()
    if cache == "cold":
        settings["PYTHONPYCACHEPREFIX"] = empty
    result = subprocess.run(
        [sys.executable, "-X", "importtime", "-c", f"import {module}"],
        capture_output=True,
        text=True,
        check=True,
        env=settings,
    )
    match = re.search(
        rf"^import time:\s*\d+\s*\|\s*(\d+)\s*\|\s*{module}$", result.stderr, re.MULTILINE
    )
    cumulative = int(match[1])
    with open(output, "a") as stream:
        print(cumulative, file=stream)
