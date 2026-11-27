"""Measure each zero-argument callable returned by a performance adapter."""

from __future__ import annotations

import cProfile
import importlib.util
import json
import pstats
import statistics
import sys
import time
import tracemalloc
from contextlib import redirect_stdout
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable


def measure(name: str, workload: Callable[[], object]) -> dict[str, object]:
    workload()
    elapsed = []
    for _ in range(5):
        started = time.perf_counter()
        workload()
        elapsed.append((time.perf_counter() - started) * 1000)

    profiler = cProfile.Profile(time.process_time)
    profiler.runcall(workload)
    stats = pstats.Stats(profiler)
    functions = []
    project = Path.cwd()
    for (source_file, line, function), (_, calls, own, cumulative, _) in sorted(
        stats.stats.items(), key=lambda item: (-item[1][2], item[0])
    ):
        file = source_file
        if not file.startswith(("<", "~")):
            file = (
                str(Path(file).resolve().relative_to(project))
                if Path(file).resolve().is_relative_to(project)
                else file
            )
        functions.append(
            {
                "file": file,
                "line": line,
                "name": function,
                "cpu_ms": own * 1000,
                "cumulative_cpu_ms": cumulative * 1000,
                "cpu_share": own / stats.total_tt if stats.total_tt else 0.0,
                "calls": calls,
            }
        )

    peak = 0
    for _ in range(5):
        tracemalloc.start()
        try:
            workload()
            peak = max(peak, tracemalloc.get_traced_memory()[1])
        finally:
            tracemalloc.stop()
    return {
        "name": name,
        "median_ms": statistics.median(elapsed),
        "peak_bytes": peak,
        "functions": functions,
    }


def main(path: Path) -> None:
    sys.path[:0] = [str(path.parent), str(Path.cwd())]
    spec = importlib.util.spec_from_file_location("xpd_perf_adapter", path)
    if spec is None or spec.loader is None:
        msg = f"Cannot load performance adapter {path}"
        raise ValueError(msg)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    with redirect_stdout(sys.stderr):
        spec.loader.exec_module(module)
        workloads = [measure(name, workload) for name, workload in module.workloads().items()]
    print(json.dumps({"workloads": workloads}, allow_nan=False))


if __name__ == "__main__":
    try:
        main(Path(sys.argv[1]))
    except Exception as error:  # noqa: BLE001
        print(json.dumps({"error": f"{type(error).__name__}: {error}"}))
        sys.exit(2)
