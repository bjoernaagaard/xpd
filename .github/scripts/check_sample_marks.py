"""Check sample comments against xpd check samples --format json."""

# ruff: noqa: INP001

import json
import subprocess
import sys
import tokenize
from collections import Counter
from pathlib import Path
from typing import NamedTuple

SAMPLES = Path("samples")
MARK = "# xpd:"
README_EXAMPLE = SAMPLES / "bad.py"


class Finding(NamedTuple):
    path: Path
    line: int
    rule: str


def expected_findings() -> Counter[Finding]:
    expected: Counter[Finding] = Counter()
    for path in SAMPLES.rglob("*.py"):
        with path.open("rb") as source:
            for token in tokenize.tokenize(source.readline):
                if token.type == tokenize.COMMENT and token.string.startswith(MARK):
                    for rule in token.string.removeprefix(MARK).split(","):
                        expected[Finding(path, token.start[0], rule.strip())] += 1
    return expected


def reported_findings(command: list[str]) -> Counter[Finding]:
    result = subprocess.run(  # noqa: S603
        [*command, "check", str(SAMPLES), "--format", "json"],
        check=False,
        stdout=subprocess.PIPE,
        text=True,
    )
    if result.returncode not in (0, 1):
        sys.exit(f"xpd check samples failed with exit code {result.returncode}")

    reported: Counter[Finding] = Counter()
    for issue in json.loads(result.stdout)["issues"]:
        path = Path(issue["file"])
        if path != README_EXAMPLE:
            reported[Finding(path, issue["line"], issue["rule"])] += 1
    return reported


def main() -> int:
    command = sys.argv[1:] or ["cargo", "run", "--quiet", "--"]
    expected = expected_findings()
    reported = reported_findings(command)
    missing = reported - expected
    stale = expected - reported
    for (path, line, rule), count in sorted(missing.items()):
        sys.stderr.write(f"{path}:{line}: add {rule} to the {MARK} comment ({count} missing)\n")
    for (path, line, rule), count in sorted(stale.items()):
        marker = rule or "(empty rule)"
        sys.stderr.write(
            f"{path}:{line}: remove {marker} from the {MARK} comment ({count} extra)\n"
        )
    if missing or stale:
        return 1

    sys.stdout.write(f"Sample marks match {reported.total()} xpd diagnostics\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
