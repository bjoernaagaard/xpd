"""Refresh the numeric fields from the saved runs without changing the verdicts."""

import json
from copy import deepcopy
from pathlib import Path

import yaml

studies = ["mdp", "merge", "heapsort", "count", "deque", "membership", "cache"]
versions = ["3.13.18", "3.14.10", "3.15.2"]
path = Path("posts.yml")
prefix, *sections = path.read_text().split("\n- url: ")
output = [prefix]
for study, section in zip(studies, sections, strict=True):
    files = [Path("runs", f"{study}-{version}.json") for version in versions]
    data = [json.loads(path.read_text()) for path in files]
    rerun = {"command": "./run.sh " + study, "interpreter_versions": versions,
             "raw_output": str(files[0]), "raw_files": list(map(str, files))}
    noisy = False
    numbers = {}
    for record in data:
        primary = record["rows"][0]
        selected = primary["attempts"][primary["best"]]
        numbers[record["python"]] = summary = {
            "size": primary["size"],
            "type": primary["kind"],
            "seconds": selected["stats"],
            "paired_ratios": selected["ratios"],
            "initial_load_average": selected["loadavg"][0],
            "attempt": primary["best"] + 1,
        }
        for row in record["rows"]:
            attempt = row["attempts"][row["best"]]
            if attempt["max_cv"] > 0.05:
                noisy = True
                summary.setdefault("noisy_sizes", []).append(
                    {
                        "size": row["size"],
                        "type": row["kind"],
                        "max_cv": attempt["max_cv"],
                        "initial_load_average": attempt["loadavg"][0],
                        "seconds": deepcopy(attempt["stats"]),
                    }
                )
    rerun["our_numbers"] = numbers
    fields = yaml.safe_dump({"noisy": noisy, "rerun": rerun}, sort_keys=False)
    original = section.split("\n  noisy:")[0].rstrip()
    output.append(original + "\n" + "\n".join("  " + line for line in fields.splitlines()))
path.write_text("\n- url: ".join(output) + "\n")
