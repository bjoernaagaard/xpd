Run `xpd harden` to answer one question. Does the code fail loudly, deterministically and honestly?

| Group | Finding kinds |
| --- | --- |
| Silent failure | `swallowed-exception`, `rule-issue` |
| Fixed waits and retries | `sleep-or-retry` |
| Unreliable tests | `disabled-test`, `flaky-test` |

Each module with a finding gets six checklist items, including test files. Run `xpd harden --questions` to read the questions and the facts that inform them. Set `goal` to `design` for a checklist fix.

At each gate, xpd repeats the first check after the checks pass. The exit status and the SHA-256 digest of stdout must agree with the first execution.

## Recorded run

1. **Start**. Record the current commit, paths and project checks. Start from a clean tracked tree. Run `xpd harden start . --check "your-project-check"`.
2. **List**. Read the findings and checklist with the bare command. Use JSON to get stable finding IDs and checklist item IDs. Run `xpd harden . --format json`.
3. **Plan**. Record an ordered list of fixes and any findings or checklist answers that have a reason. Name a finding ID or set a supported goal for each fix. A design goal names checklist items in `questions`. Under `answered`, give each item a verdict of `pass`, `na` or `accepted` and a reason. Run `xpd harden plan plan.json`.
4. **Approve**. Record approval for a fix that changes what callers or users see. Run `xpd harden approve fix-id`.
5. **Gate**. Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item. Run `xpd harden gate fix-id`.
6. **Drop**. Return to the run’s current commit and record a reason for dropping the fix. Run `xpd harden drop fix-id --reason "reason"`.
7. **Status**. Show the current run, its recorded answers and the next stage. Run `xpd harden status`.
8. **Finish**. Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure. Run `xpd harden finish`.

## Example

Replace a swallowed exception with a failure the caller can handle.

```python title="loader.py"
def load_count(raw):
    try:
        return int(raw)
    except ValueError:
        pass
```

```console
$ git init -q && git add loader.py && git -c user.name=xpd -c user.email=xpd@example.com -c commit.gpgsign=false commit -qm start
$ xpd harden start loader.py --check "python3 -m py_compile loader.py" --format json | jq -c '{findings: .run.was, next: .next.stage}'
{"findings":1,"next":"plan"}
$ xpd harden loader.py --format json | jq -c '{kinds: [.findings[].kind], items: (.checklist | length)}'
{"kinds":["swallowed-exception"],"items":6}
$ printf '%s\n' '{"fixes":[{"id":"propagate","title":"Propagate invalid input","goal":"design","questions":["loader.py/fails-loudly"]}]}' | xpd harden plan - --format json | jq -c .next.stage
"fix"
$ sed -i 's/        pass/        raise/' loader.py && git -c user.name=xpd -c user.email=xpd@example.com -c commit.gpgsign=false commit -qam "Propagate invalid input"
$ xpd harden gate propagate --format json | jq -c '.run.answered["loader.py/fails-loudly"].verdict'
"fixed"
$ xpd harden finish --format json | jq -c .run.finished
xpd: 1 findings at the start, 0 now: 1 fixed, 0 added, 0 left without a reason; lines 5 -> 5
true
```

## Exit status

A successful list or recorded stage exits 0. A rejected gate or a finish blocked by findings or checklist items exits 1. Invalid input exits 2.
