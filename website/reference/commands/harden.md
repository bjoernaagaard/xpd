Run `xpd harden` to find silent failures, fixed waits, retries, and unreliable tests, and verify every fix against a reliability checklist and determinism gate. See the [hardening guide](../../guides/harden/) for a full walkthrough.

| Group | Finding kinds |
| --- | --- |
| Silent failure and reliability errors | `swallowed-exception`, `rule-issue` |
| Fixed waits and retries | `sleep-or-retry` |
| Disabled or unreliable tests | `disabled-test`, `flaky-test` |

Each module with a finding, including test files, gets six checklist items. Run `xpd harden --questions` to read the questions and the facts that inform them. Set `goal` to `design` for a checklist fix.

At each gate, xpd repeats the first check after the checks pass. The exit status and the SHA-256 digest of stdout must agree with the first execution.

## Recorded run

1. **Start**: record the current commit, paths, and project checks from a clean tracked tree with `xpd harden start . --check "your-project-check"`.
2. **List**: read stable finding IDs and checklist item IDs with `xpd harden . --format json`.
3. **Plan**: record ordered fixes, accepted findings, and checklist verdicts (`pass`, `na`, or `accepted`) with reasons using `xpd harden plan plan.json`.
4. **Approve**: record approval for an interface change with `xpd harden approve fix-id`.
5. **Gate**: commit one fix and run `xpd harden gate fix-id`. A `design` fix must edit a file that owns a cited checklist item.
6. **Drop**: return to the run's current commit and record a reason with `xpd harden drop fix-id --reason "reason"`.
7. **Status**: show the current run, its recorded answers, and the next stage with `xpd harden status`.
8. **Finish**: compare against the starting commit with `xpd harden finish`. An open fix, a new finding without an accepted reason, or an unanswered checklist item causes exit 1.

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
