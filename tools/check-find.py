# ruff: noqa: INP001
"""Measure xpd find's top-five rule coverage across shuffled library questions."""

import argparse
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path
from random import Random
from time import perf_counter

ROOT = Path(__file__).resolve().parent.parent
QUESTIONS = Path(__file__).with_name("find-questions.json")
LIMIT = 5
MIN_LIBRARIES = 3

type Evidence = tuple[str, str]
type Questions = dict[str, list[Evidence]]


def load_questions() -> tuple[Questions, dict[str, list[str]]]:
    raw_suites: dict[str, dict[str, list[list[str]]]] = json.loads(QUESTIONS.read_text())
    overlap = raw_suites["questions"].keys() & raw_suites["upcoming"].keys()
    if overlap:
        message = f"Duplicate benchmark cases across suites: {sorted(overlap)}"
        raise ValueError(message)
    packs = {path.parent.name for path in (ROOT / "rules").glob("*/pack.md")}
    questions: Questions = {}
    upcoming: dict[str, list[str]] = {}
    covered = set()
    for suite_name in ("questions", "upcoming"):
        for case, raw_evidence in raw_suites[suite_name].items():
            evidence: list[Evidence] = [(prompt, rule) for prompt, rule in raw_evidence]
            libraries = {rule.split("/", 1)[0] for _, rule in evidence}
            absent = libraries - packs
            if absent and suite_name == "upcoming":
                upcoming[case] = sorted(absent)
                continue
            for _, rule in evidence:
                if not (ROOT / "rules" / f"{rule}.md").is_file():
                    message = f"{case} refers to a missing rule: {rule}"
                    raise ValueError(message)
            if len(evidence) != 1 and len(libraries - {"python", "xpd"}) < MIN_LIBRARIES:
                message = f"{case} must cover three libraries"
                raise ValueError(message)
            questions[case] = evidence
            covered.update(libraries)
    if covered != packs:
        message = f"Pack coverage differs: {sorted(covered ^ packs)}"
        raise ValueError(message)
    return questions, upcoming


def find(binary: Path, query: str) -> list[str]:
    result = subprocess.run(  # noqa: S603
        [binary, "find", "--kind", "rule", "--limit", str(LIMIT), "--format", "json", query],
        cwd=ROOT,
        capture_output=True,
        check=True,
        text=True,
    )
    if "Search uses keywords only" in result.stderr:
        raise RuntimeError(result.stderr.strip())
    rules = json.loads(result.stdout)["rules"]
    return [rule["name"] for rule in rules]


def check(binary: Path, questions: Questions, seed: int) -> dict[str, object]:
    random = Random(seed)  # noqa: S311
    cases = list(questions.items())
    random.shuffle(cases)
    rows = []
    mixed_cases = complete_mixed = complete_single = 0
    available: Counter[str] = Counter()
    retrieved: Counter[str] = Counter()
    for name, evidence in cases:
        shuffled = random.sample(evidence, k=len(evidence))
        query = " ".join(prompt for prompt, _ in shuffled)
        expected = {rule for _, rule in shuffled}
        start = perf_counter()
        found = find(binary, query)
        elapsed = perf_counter() - start
        matches = expected.intersection(found)
        if len(evidence) > 1:
            mixed_cases += 1
            complete_mixed += matches == expected
        else:
            complete_single += matches == expected
        available.update(rule.split("/", 1)[0] for rule in expected)
        retrieved.update(rule.split("/", 1)[0] for rule in matches)
        rows.append(
            {
                "case": name,
                "query": query,
                "expected": sorted(expected),
                "found": found,
                "missing": sorted(expected - matches),
                "seconds": round(elapsed, 3),
            }
        )
    return {
        "seed": seed,
        "limit": LIMIT,
        "complete_mixed": complete_mixed,
        "mixed_cases": mixed_cases,
        "complete_single": complete_single,
        "expected_by_pack": dict(sorted(available.items())),
        "found_by_pack": {pack: retrieved[pack] for pack in sorted(available)},
        "cases": rows,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="path to the xpd binary to measure")
    parser.add_argument("--seed", type=int, default=20261122)
    args = parser.parse_args()
    questions, upcoming = load_questions()
    report = check(args.binary.resolve(), questions, args.seed)
    report["upcoming_cases"] = upcoming
    sys.stdout.write(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
