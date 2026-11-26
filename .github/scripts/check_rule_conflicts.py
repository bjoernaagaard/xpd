"""Find where the pinned Ruff works against an xpd rule, and fail on a pair without a stance.

An xpd rule wins over a Ruff rule. This script measures every place where the two meet, with
`ruff` and `xpd` from PATH and the Ruff rules and settings of `src/doctor/profile.toml`, without
its `unfixable` list. A rule that the profile turns off cannot work against an xpd rule.
Each pair of a Ruff rule and an xpd rule that it finds must have a stance in the `xpd` field of
the Ruff rule's record in `src/doctor/decisions.jsonl`. `xpd doctor` checks that the profile
acts on each stance.

```text
python3 .github/scripts/check_rule_conflicts.py [--xpd <path>] [--json <file>]
```

It measures five things.

```text
fix      one Ruff rule's fixes on samples/ change how often an xpd rule reports
format   ruff format on samples/ makes an xpd rule report more often
page     Ruff reports a rule on the code that an xpd rule page recommends, and not on its old code
example  xpd reports a rule on the code that a Ruff rule page recommends, and not on its bad one
broken   one xpd rule's fixes on samples/ leave an undefined name, a redefinition or a syntax error
```

The stances of the formatter are on the record of `line-length`, the width it wraps lines at. On
the code of a rule page it passes over the Ruff rules in `FRAGMENT`, which fire only because the
code is a fragment, such as a name the page never defines. A fix may leave none of `BROKEN`.
"""

# ruff: noqa: INP001, S603

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
from itertools import chain
from pathlib import Path
from typing import NamedTuple

SAMPLES = Path("samples")
PROFILE = Path("src/doctor/profile.toml")
DECISIONS = Path("src/doctor/decisions.jsonl")
XPD = ["xpd"]
FORMAT = "line-length"
FRAGMENT = {"F401", "F821", "I001", "E302", "E305", "W391", "W292"}
BROKEN = {"F811", "F821", "invalid-syntax"}
DIFF_BLOCK = re.compile(r"```diff[^\n]*\n(.*?)```", re.DOTALL)
CODE_BLOCK = re.compile(r"```(?:python|py|pycon)?\n(.*?)```", re.DOTALL)


class Pair(NamedTuple):
    """A Ruff rule that meets an xpd rule, with how and where."""

    ruff: str
    xpd: str
    kind: str
    where: str


def run(command: list[str], cwd: Path) -> str:
    """Return the standard output of `command`, whatever its exit code."""
    result = subprocess.run(command, cwd=cwd, check=False, capture_output=True, text=True)
    return result.stdout


def settings_toml() -> str:
    """Return a pyproject.toml with the profile's Ruff rules and settings, and strict xpd."""
    ruff = tomllib.loads(PROFILE.read_text())["ruff"]
    tables: dict[str, dict[str, object]] = defaultdict(dict)
    tables["lint"]["select"] = ruff["select"]
    tables["lint"]["ignore"] = ruff["ignore"]
    for table, key, value in leaves(ruff["settings"], ()):
        if key not in {"unfixable", "required-version"}:
            tables[table][key] = value
    lines = ['[tool.xpd]\nstyle = "strict"\n']
    for name in sorted(tables):
        lines.append(f"[tool.ruff.{name}]" if name else "[tool.ruff]")
        for key, value in tables[name].items():
            lines.append(f"{key} = {json.dumps(value)}")
        lines.append("")
    return "\n".join(lines)


def leaves(table: dict[str, object], path: tuple[str, ...]) -> list[tuple[str, str, object]]:
    """Return every value below `table` with the dotted name of its table and its key."""
    found = []
    for key, value in table.items():
        if isinstance(value, dict):
            found += leaves(value, (*path, key))
        else:
            found.append((".".join(path), key, value))
    return found


def xpd_issues(root: Path, targets: list[str]) -> list[dict[str, str]]:
    """Return the issues of `xpd check`, each with its file relative to `root`."""
    report = json.loads(run([*XPD, "check", *targets, "--format", "json"], root))
    issues = report["issues"] if isinstance(report, dict) else report
    for issue in issues:
        file = Path(issue["file"])
        issue["file"] = str(file.relative_to(root) if file.is_absolute() else file)
        issue["id"] = f"{issue['package']}/{issue['rule']}"
    return issues


def xpd_counts(root: Path, targets: list[str]) -> Counter[tuple[str, str]]:
    """Count the issues of `xpd check` by file and rule."""
    return Counter((issue["file"], issue["id"]) for issue in xpd_issues(root, targets))


def ruff_findings(root: Path, targets: list[str]) -> list[tuple[str, str, bool]]:
    """Return the file, the code and whether a fix exists for each finding of `ruff check`."""
    command = ["ruff", "check", *targets, "--no-fix", "--output-format", "json"]
    findings = []
    for finding in json.loads(run(command, root)):
        file = str(Path(finding["filename"]).relative_to(root))
        findings.append((file, finding["code"] or "invalid-syntax", bool(finding["fix"])))
    return findings


def project(root: Path, name: str, config: str, files: dict[str, str]) -> Path:
    """Make the project `name` below `root` with `files`."""
    folder = root / name
    shutil.rmtree(folder, ignore_errors=True)
    for relative, text in files.items():
        path = folder / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    (folder / "pyproject.toml").write_text(config)
    return folder


def changed(folder: Path, files: dict[str, str]) -> list[str]:
    """Return the files of `folder` whose text differs from `files`."""
    return [name for name, text in files.items() if (folder / name).read_text() != text]


def fix_pairs(root: Path, config: str, samples: dict[str, str]) -> list[Pair]:
    """Apply one Ruff rule's fixes at a time, and compare what xpd reports."""
    base = project(root, "base", config, samples)
    before = xpd_counts(base, ["samples"])
    fixable: dict[str, set[str]] = defaultdict(set)
    for file, code, has_fix in ruff_findings(base, ["samples"]):
        if has_fix:
            fixable[code].add(file)

    def one(code: str) -> list[Pair]:
        files = {name: samples[name] for name in sorted(fixable[code])}
        folder = project(root, f"fix-{code}", config, files)
        run(["ruff", "check", *files, "--select", code, "--fix", "--unsafe-fixes", "-q"], folder)
        touched = changed(folder, files)
        if not touched:
            return []
        after = xpd_counts(folder, touched)
        pairs = []
        for name, rule in sorted({*before, *after}):
            if name in touched and after[name, rule] != before[name, rule]:
                pairs.append(Pair(code, rule, "fix", name))
        return pairs

    with ThreadPoolExecutor() as pool:
        return list(chain.from_iterable(pool.map(one, sorted(fixable))))


def format_pairs(root: Path, config: str, samples: dict[str, str]) -> list[Pair]:
    """Format the samples, and list the xpd rules that then report more often."""
    before = xpd_counts(root / "base", ["samples"])
    folder = project(root, "format", config, samples)
    run(["ruff", "format", "samples", "-q"], folder)
    pairs = []
    for (name, rule), count in sorted(xpd_counts(folder, ["samples"]).items()):
        if count > before[name, rule]:
            pairs.append(Pair(FORMAT, rule, "format", name))
    return pairs


def diff_sides(doc: str) -> tuple[str, str]:
    """Return the code before and after of every diff block of a rule page."""
    before, after = [], []
    for block in DIFF_BLOCK.findall(doc):
        for line in block.splitlines():
            if not line.startswith("+"):
                before.append(line[1:])
            if not line.startswith("-"):
                after.append(line[1:])
        before.append("")
        after.append("")
    return "\n".join(before), "\n".join(after)


def codes(folder: Path) -> Counter[tuple[str, str]]:
    """Count the findings of Ruff in `folder` by file and code."""
    return Counter((file, code) for file, code, _ in ruff_findings(folder, ["."]))


def page_pairs(root: Path, config: str, rules: list[dict[str, str]]) -> list[Pair]:
    """Run Ruff on the code each xpd rule page starts from and on the code it recommends."""
    names, before, after = {}, {}, {}
    for rule in rules:
        old, new = diff_sides(rule["doc"])
        if new.strip():
            file = f"{rule['package']}__{rule['rule']}.py".replace("-", "_")
            names[file] = f"{rule['package']}/{rule['rule']}"
            before[file], after[file] = old, new
    seen = codes(project(root, "old", config, before))
    found = codes(project(root, "new", config, after))
    pairs = []
    for file, code in sorted(set(found) - set(seen)):
        if code not in FRAGMENT and code != "invalid-syntax":
            pairs.append(Pair(code, names[file], "page", names[file]))
    return pairs


def example_pairs(root: Path, config: str) -> list[Pair]:
    """Run xpd on the bad example and on the recommended code of every Ruff rule page."""
    bad, good = {}, {}
    off = tuple(tomllib.loads(PROFILE.read_text())["ruff"]["ignore"])
    for rule in json.loads(run(["ruff", "rule", "--all", "--output-format", "json"], root)):
        example = rule["explanation"].partition("## Example")[2]
        wrong, found, recommended = example.partition("Use instead")
        code = rule["code"] or ""
        if found and code and "Removed" not in rule["status"] and not code.startswith(off):
            bad[f"{code}.py"] = "\n".join(CODE_BLOCK.findall(wrong))
            section = recommended.partition("\n## ")[0]
            good[f"{code}.py"] = "\n".join(CODE_BLOCK.findall(section))
    before = xpd_counts(project(root, "bad", config, bad), ["."])
    after = xpd_counts(project(root, "good", config, good), ["."])
    pairs = []
    for (file, rule), count in sorted(after.items()):
        if count > before[file, rule]:
            pairs.append(Pair(Path(file).stem, rule, "example", Path(file).stem))
    return pairs


def broken(folder: Path, targets: list[str]) -> Counter[tuple[str, str]]:
    """Count the findings of Ruff that no fix may leave behind, by file and code."""
    found = ruff_findings(folder, targets)
    return Counter((file, code) for file, code, _ in found if code in BROKEN)


def broken_pairs(root: Path, config: str, samples: dict[str, str]) -> list[Pair]:
    """Apply one xpd rule's fixes at a time where all of them together leave broken code."""
    base = root / "base"
    before = broken(base, ["samples"])
    fixed = project(root, "fixed", config, samples)
    run([*XPD, "check", "samples", "--fix", "--unsafe-fixes"], fixed)
    suspects = set()
    for (name, code), count in broken(fixed, ["samples"]).items():
        if count > before[name, code]:
            suspects.add(name)
    if not suspects:
        return []
    fixable = set()
    for issue in xpd_issues(base, sorted(suspects)):
        if issue["fix"]:
            fixable.add((issue["file"], issue["rule"], issue["id"]))
    pairs = []
    for name, rule, rule_id in sorted(fixable):
        folder = project(root, "one", config, {name: samples[name]})
        run([*XPD, "check", name, "--rule", rule, "--fix", "--unsafe-fixes"], folder)
        for (file, code), count in sorted(broken(folder, [name]).items()):
            if count > before[file, code]:
                pairs.append(Pair(code, rule_id, "broken", name))
    return pairs


def stances() -> dict[tuple[str, str], str]:
    """Return the stance of every pair that decisions.jsonl records, by Ruff id and xpd rule."""
    recorded = {}
    for line in DECISIONS.read_text().splitlines():
        record = json.loads(line)
        if record["tool"] == "ruff":
            for rule, stance in record.get("xpd", {}).items():
                recorded[record["id"], rule] = stance
    return recorded


def failures(pairs: list[Pair], known: set[str]) -> tuple[list[str], int]:
    """Return a line for each pair without a stance and each stance that names no xpd rule."""
    recorded = stances()
    measured = {(pair.ruff, pair.xpd): pair for pair in reversed(pairs)}
    lines = []
    for key, pair in sorted(measured.items()):
        if key not in recorded:
            where = f"{pair.kind}, {pair.where}"
            lines.append(f"{pair.ruff} meets {pair.xpd} ({where}) and has no stance")
    for ruff, rule in sorted(recorded):
        if rule not in known:
            lines.append(f"{ruff} names {rule}, which is no xpd rule")
    return lines, len(set(recorded) - set(measured))


def main() -> None:
    """Measure the pairs, and exit 1 when one has no stance or a stance names no xpd rule."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", type=Path, help="write every measured pair to this file")
    parser.add_argument("--xpd", type=Path, help="the xpd to run, in place of the one on PATH")
    args = parser.parse_args()
    if args.xpd:
        XPD[:] = [str(args.xpd.resolve())]
    config = settings_toml()
    samples = {str(path): path.read_text() for path in sorted(SAMPLES.rglob("*.py"))}
    rules = json.loads(run([*XPD, "rules", "--builtin", "--format", "json"], Path.cwd()))
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch).resolve()
        pairs = fix_pairs(root, config, samples)
        pairs += format_pairs(root, config, samples)
        pairs += page_pairs(root, config, rules)
        pairs += example_pairs(root, config)
        pairs += broken_pairs(root, config, samples)
    if args.json:
        rows = [pair._asdict() for pair in pairs]
        args.json.write_text(json.dumps(rows, indent=1) + "\n")
    known = {f"{rule['package']}/{rule['rule']}" for rule in rules}
    lines, unmeasured = failures(pairs, known)
    for line in lines:
        sys.stdout.write(f"FAIL {line}\n")
    measured = len({(pair.ruff, pair.xpd) for pair in pairs})
    sys.stdout.write(f"{measured} pairs measured, {len(lines)} failures, ")
    sys.stdout.write(f"{unmeasured} recorded pairs that no sample shows\n")
    sys.exit(1 if lines else 0)


if __name__ == "__main__":
    main()
