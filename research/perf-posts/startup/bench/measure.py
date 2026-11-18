"""Each record contains one paired hyperfine invocation and its preflight evidence."""

import json
import os
import platform
import shlex
import statistics
import subprocess
import sys
from importlib.metadata import distributions
from pathlib import Path
from tempfile import TemporaryDirectory
from workloads import build

selected, directory = sys.argv[1:]
work = Path(directory)
python = sys.executable
version = platform.python_version()
modern = sys.version_info >= (3, 15)
core = 4
assert os.sched_getaffinity(0) == {4}
prefix = ["taskset", "-c", str(core)]
cases = [
    "nine",
    "sxz",
    "boto",
    "docker",
    "fleet",
    "lemire",
    "stats",
    "struct",
    "posting",
    "vex",
    "django",
    "typing",
    "order",
    "module",
    "touch",
]
if selected != "all" and selected not in cases:
    raise SystemExit(f"Choose one of {cases}")


def measure(case, labels, commands, import_module=None):
    raw = Path("runs", f"{case}-{version}.json")
    record = {
        "version": version,
        "core": core,
        "commands": commands,
        "packages": {d.metadata["Name"]: d.version for d in distributions()},
        "attempts": [],
    }
    if case == "module":
        modules = []
        for label in labels:
            output = subprocess.check_output(
                [python, "-c", f"import {label}; print({label}.__name__)"]
            )
            assert output.decode().strip() == label
            modules.append(label)
        record["imported"] = modules
        record["bare_modules"] = int(
            subprocess.check_output([python, "-c", "import sys; print(len(sys.modules))"])
        )
    elif case == "order":
        outputs = [
            subprocess.check_output(
                [python, "-c", f"import {stmt}; print(asyncio.__name__, logging.__name__)"]
            )
            for stmt in ["asyncio, logging", "logging, asyncio"]
        ]
        assert outputs[0] == outputs[1]
        record["stdout"] = outputs[0].decode()
    elif import_module:
        for label in labels:
            with TemporaryDirectory(dir=work) as empty:
                settings = os.environ.copy()
                if label == "cold":
                    settings["PYTHONPYCACHEPREFIX"] = empty
                checked = subprocess.check_output(
                    [python, "-c", f"import {import_module}; print({import_module}.__name__)"],
                    env=settings,
                )
                assert checked.decode().strip() == import_module
        record["stdout"] = checked.decode()
    else:
        outputs = [subprocess.check_output(command) for command in commands]
        assert all(out == outputs[0] for out in outputs)
        record["stdout"] = outputs[0].decode()
    for _ in range(3):
        export = work / "hyperfine.json"
        if import_module or case in {"order", "module"}:
            for label in labels:
                (work / label).unlink(missing_ok=True)
        args = ["hyperfine", "-N", "--warmup", "2", "--runs", "5", "--export-json", str(export)]
        sample_load = work / "sample-load"
        sample_load.write_text("")
        for label, command in zip(labels, commands, strict=True):
            prepare = [
                "sh",
                "-c",
                'printf "%s " "$1" >> "$2"; cut -d" " -f1-3 /proc/loadavg >> "$2"',
                "load",
                label,
                str(sample_load),
            ]
            args.extend(["--prepare", shlex.join(prepare)])
            args.extend(["-n", label, shlex.join(command)])
        # Read load immediately before hyperfine starts.
        load = work / "load"
        subprocess.run(
            prefix
            + [
                "sh",
                "-c",
                'cut -d" " -f1-3 /proc/loadavg > "$1"; shift; exec "$@"',
                "measure",
                str(load),
                *args,
            ],
            check=True,
            stdout=subprocess.DEVNULL,
        )
        payload = json.loads(export.read_text())
        record["hyperfine_version"] = payload.get("metadata", {}).get("hyperfine_version", "2")
        results = payload["results"]
        summary = {}
        for label, result in zip(labels, results, strict=True):
            if case in {"order", "module"}:
                lines = (work / label).read_text().splitlines()[-5:]
                result["importtime_samples"] = [[int(x) for x in line.split()] for line in lines]
                samples = [row[0] / 1000 for row in result["importtime_samples"]]
            elif import_module:
                samples = [int(x) / 1000 for x in (work / label).read_text().splitlines()][-5:]
            else:
                samples = [row["time_wall_clock"]["value"] * 1000 for row in result["measurements"]]
            summary[label] = dict(
                mean_ms=statistics.mean(samples),
                sd_ms=statistics.stdev(samples),
                median_ms=statistics.median(samples),
                samples_ms=samples,
            )
        cv = max(v["sd_ms"] / v["mean_ms"] for v in summary.values())
        record["attempts"].append(
            dict(
                load=load.read_text().strip(),
                cv=cv,
                sample_loads=sample_load.read_text().splitlines(),
                summary=summary,
                hyperfine=results,
            )
        )
        if cv <= 0.05:
            break
    record["best"] = min(range(len(record["attempts"])), key=lambda i: record["attempts"][i]["cv"])
    raw.write_text(json.dumps(record, indent=2) + "\n")
    print(raw, flush=True)


for case in cases if selected == "all" else [selected]:
    if case == "typing":
        if modern:
            labels, commands = [], []
            for release in ["3.13.18", "3.14.10", "3.15.2"]:
                exe = subprocess.check_output(
                    ["uv", "python", "find", "--system", "--managed-python", release], text=True
                ).strip()
                for label, statement in [("empty", "pass"), ("typing", "import typing")]:
                    labels.append(f"{release}_{label}")
                    commands.append([exe, "-c", statement])
            measure(case, labels, commands)
        continue
    if case == "touch":
        labels = ["empty", "eager"]
        bodies = ["pass", "import asyncio"]
        if modern:
            labels.extend(["unread", "read"])
            bodies.extend(["lazy import asyncio", "lazy import asyncio; asyncio.run"])
        measure(case, labels, [[python, "-c", body] for body in bodies])
        continue
    if case in {"order", "module"}:
        labels = ["asyncio_first", "logging_first"] if case == "order" else ["asyncio", "email"]
        measure(
            case,
            labels,
            [[python, "bench/graph.py", case, label, str(work / label)] for label in labels],
        )
        continue
    if case in {"struct", "posting"}:
        for module in ["typing", "dataclasses"] if case == "struct" else ["openapi_pydantic"]:
            labels = ["warm", "cold"]
            measure(
                case + "-" + module,
                labels,
                [
                    [python, "bench/import_time.py", module, label, str(work / label)]
                    for label in labels
                ],
                module,
            )
        continue
    if case == "vex":
        measure(
            case,
            ["imported", "literal"],
            [
                [python, "-c", "from langgraph.graph.state import END; print(END)"],
                [python, "-c", 'END = "__end__"; print(END)'],
            ],
        )
        continue
    if case == "django":
        statement = "from google.cloud.asset_v1 import AssetServiceClient"
        measure(
            case,
            ["eager", "deferred"],
            [
                [python, "-c", statement + "\nprint('ready')"],
                [
                    python,
                    "-c",
                    "def client():\n    "
                    + statement
                    + "\n    return AssetServiceClient\nprint('ready')",
                ],
            ],
        )
        continue
    build(case, work)
    if case == "stats":
        labels = ["inline", "top"]
        commands = [[python, str(work / f"stats_{label}.py"), "--help"] for label in labels]
        if modern:
            for layout in ["inline", "top"]:
                labels.append(layout + "_global")
                commands.append(
                    [python, "-X", "lazy_imports=all", str(work / f"stats_{layout}.py"), "--help"]
                )
    else:
        args = {
            "sxz": ["--version"],
            "boto": ["version"],
            "nine": [],
            "docker": ["--version"],
            "lemire": ["--version"],
            "fleet": ["version"],
        }[case]
        labels = ["eager", "lazy"]
        files = [work / f"{case}_{label}.py" for label in labels]
        commands = [[python, str(file), *args] for file in files]
        if not modern and case != "boto":
            probe = subprocess.run(commands.pop(), capture_output=True, text=True)
            assert probe.returncode and "SyntaxError" in probe.stderr
            labels.pop()
        if case == "fleet":
            labels.append("module_list")
            commands.append([python, str(work / "fleet_lazy_modules.py"), *args])
    measure(case, labels, commands)
    if modern and case == "lemire":
        data = work / "data.csv"
        data.write_text("x,y\n" + "".join(f"{i},{i * i}\n" for i in range(1000)))
        for subcommand in ["mean", "stats"]:
            measure(
                case + "-" + subcommand,
                labels,
                [cmd[:-1] + [subcommand, str(data)] for cmd in commands],
            )
