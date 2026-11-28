Run `xpd audit` for a single read-only report across `xpd check`, static ledger workflows, optional `--since` regression deltas, and recorded ledger debt.

Without `--workflows`, `xpd audit` runs `xpd check` alongside the four general-purpose static workflows: `untangle`, `harden`, `optimize`, and `discover`. Pass `--workflows all` to include every registered static workflow, or pass a comma-separated list of workflow names. `xpd audit` never executes project code, never runs runtime adapters, and never writes to the ledger store.

Pass `--since <commit>` to compare both linter issues and workflow findings against an earlier commit and report newly introduced and resolved counts. Add `--fail-on-regression` in CI or local gates to exit 1 when `--since` detects net new issues or findings, or when `xpd check` reports errors without `--since`.

When a worktree has recorded ledger runs in its Duroxide store, `xpd audit` also summarizes accepted findings, accepted checklist exceptions, and open fixes for each audited workflow.

## Example

```python title="example.py"
def load_count(raw):
    try:
        return int(raw)
    except ValueError:
        pass
```

```console
$ xpd audit example.py --workflows harden --format json | jq -c '{schema: .schema_version, harden: .workflows[0].findings}'
{"schema":1,"harden":1}
```

## Exit status

A completed audit exits 0. When `--fail-on-regression` is set and a regression or `check` error is present, `xpd audit` exits 1. Invalid arguments or unreadable inputs exit 2.
