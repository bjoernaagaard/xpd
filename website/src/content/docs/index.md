---
title: Get started
description: Install xpd, check a small pandas file, and choose which fixes to apply.
---

Run `uvx xpd check .` in any Python project to find slow or broken library calls without importing or running your code. `xpd` parses each file once, detects imported libraries, and runs only the matching rule packs plus the Python pack.

```mermaid
flowchart LR
    A["xpd check ."] --> B["Parse each .py file once"]
    B --> C["Detect imported libraries"]
    C --> D["Run matching library packs and Python pack"]
    D --> E["Report issues and apply optional fixes"]
```

## Install

`xpd` requires Python 3.13 or newer. Install it once with [uv](https://docs.astral.sh/uv/getting-started/installation/), or replace `xpd` with `uvx xpd` in any command to run without installing.

```sh
uv tool install --python '>=3.13' xpd
```

## Check a sample file

Save `bad.py` in an empty folder. You do not need pandas installed because `xpd` never executes your script.

```python title="bad.py" showLineNumbers
import pandas as pd

df = pd.DataFrame({"a": [1, 2], "k": [1, 2]})
other = df.copy()
for i, row in df.iterrows():
    df["a"][i] = row["a"] * 2
df.dropna(inplace=True)
m = pd.merge(df, other, on="k")
out = pd.DataFrame()
for f in ["x.csv"]:
    out = pd.concat([out, pd.read_csv(f)])
```

Run `xpd check bad.py`. Terminal runs print text, and piped runs print JSON (pass `--format text` in scripts when you want text).

```console
$ xpd check bad.py --format text
bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 more with --fix --unsafe-fixes
```

## Read diagnostics and apply fixes

Each diagnostic line names the location, severity, rule identifier, and replacement guidance.

| Severity | Meaning | Exit code |
| --- | --- | --- |
| `error` | Code produces a wrong result or raises an exception | `1` |
| `warning` | Code is slow or fragile | `0` |
| `info` | A shorter or cleaner idiom exists | `0` |

Safe fixes (`--fix`) preserve runtime behavior. Unsafe fixes (`--fix --unsafe-fixes`) can change behavior, so review the diff after applying them.

```console
$ xpd check bad.py --fix --unsafe-fixes --format text
...
xpd: fixed 2, 3 issues
```

```diff lang="python"
 for i, row in df.iterrows():
-    df["a"][i] = row["a"] * 2
-df.dropna(inplace=True)
+    df.loc[i, "a"] = row["a"] * 2
+df = df.dropna()
```

Line 6 now mutates `df` in place under pandas copy-on-write, and line 7 returns a new DataFrame instead of mutating shared references. No errors remain, so `xpd check` exits with `0`.

## Where to go next

| Goal | Page |
| --- | --- |
| Browse every rule pack and fix level | [Rules](rules/) |
| Wire xpd into Claude Code or Codex | [Claude Code and other agents](guides/claude-code/) |
| Expose visual tools, gates, and animations over MCP | [Connect an agent with MCP](guides/mcp/) |
| Validate a scikit-learn pipeline end to end | [Train a model with gates](guides/gates/) |
| Get live diagnostics and quick fixes in your editor | [Use xpd in an editor](guides/editors/) |
| Audit charts against Tufte's principles | [Review a chart against Tufte's book](guides/chart-review/) |
| Prepare features and targets | [Record a data preparation review](guides/data-prep/) |
| Render editable Manim concept animations | [Animate a concept with Manim](guides/animate/) |
| Author custom project rules or a library pack | [Write a rule](guides/write-a-rule/) |
| Look up flags and exit codes for any command | [Commands](commands/) |
