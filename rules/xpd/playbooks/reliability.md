# Harden Python code and tests with xpd harden

Use the reliability checklist to find silent failures and verify that a check gives the same result on the same commit.

## Asks

- Harden flaky Python tests and swallowed exceptions with xpd harden
- Find unreliable tests, fixed waits and silent failures
- Check that a Python project gives the same result on each run

## Code

```python
def parse_port(value: str) -> int:
    try:
        return int(value)
    except ValueError as error:
        raise ValueError(f"Invalid port: {value!r}") from error
```

## Commands

1. **Start**. Record the current commit, paths and project checks. Start from a clean tracked tree. Run `xpd harden start . --check "your-project-check"`.
2. **List**. Read the findings and checklist with the bare command. Use JSON to get stable finding IDs and checklist item IDs. Run `xpd harden . --format json`.
3. **Plan**. Record an ordered list of fixes and any findings or checklist answers that have a reason. Name a finding ID or set a supported goal for each fix. A design goal names checklist items in `questions`. Under `answered`, give each item a verdict of `pass`, `na` or `accepted` and a reason. Run `xpd harden plan plan.json`.
4. **Approve**. Record approval for a fix that changes what callers or users see. Run `xpd harden approve fix-id`.
5. **Gate**. Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item. Run `xpd harden gate fix-id`.
6. **Drop**. Return to the run’s current commit and record a reason for dropping the fix. Run `xpd harden drop fix-id --reason "reason"`.
7. **Status**. Show the current run, its recorded answers and the next stage. Run `xpd harden status`.
8. **Finish**. Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure. Run `xpd harden finish`.

## Rules

- python/test-rerun
- python/test-sleep
- python/test-suppressed-health-check
- python/test-skip
