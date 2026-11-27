Run `xpd untangle` to answer one question. Do pieces that change together live together, with no dead weight?

| Group | Finding kinds |
| --- | --- |
| Hidden links | `copy`, `near-copy`, `changes-together`, `shared-state` |
| Needless links | `volatile-import`, `import-cycle`, `private-reach`, `vague-module` |
| Surplus | `pass-through`, `default-argument`, `unreferenced`, `constant-parameter` |
| Special cases | `one-value-branch`, `one-caller-parameter` |

A module with a finding gets six checklist items. Run `xpd untangle --questions` to read the questions and the facts that inform them. The supported fix goals are `shorter` or `design`.

A `shorter` fix needs a lower line count. For a data script, run `xpd equiv` on the old and new versions. Record the result in the fix commit.

## Recorded run

1. **Start**. Record the current commit, paths and project checks. Start from a clean tracked tree. Run `xpd untangle start . --check "your-project-check"`.
2. **List**. Read the findings and checklist with the bare command. Use JSON to get stable finding IDs and checklist item IDs. Run `xpd untangle . --format json`.
3. **Plan**. Record an ordered list of fixes and any findings or checklist answers that have a reason. Name a finding ID or set a supported goal for each fix. A design goal names checklist items in `questions`. Under `answered`, give each item a verdict of `pass`, `na` or `accepted` and a reason. Run `xpd untangle plan plan.json`.
4. **Approve**. Record approval for a fix that changes what callers or users see. Run `xpd untangle approve fix-id`.
5. **Gate**. Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item. Run `xpd untangle gate fix-id`.
6. **Drop**. Return to the run’s current commit and record a reason for dropping the fix. Run `xpd untangle drop fix-id --reason "reason"`.
7. **Status**. Show the current run, its recorded answers and the next stage. Run `xpd untangle status`.
8. **Finish**. Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure. Run `xpd untangle finish`.

## Example

```console
$ xpd untangle --questions --format json | jq -c '{facts: (.facts | length), questions: (.questions | length)}'
{"facts":2,"questions":6}
```

## Exit status

A successful list or recorded stage exits 0. A rejected gate or a finish blocked by findings or checklist items exits 1. Invalid input exits 2.
