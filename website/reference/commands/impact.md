Use `xpd impact` to trace a proposed change from a column or transformation
to the outputs and callers that need checking. It reads Python source without
running it.

Name the pipeline with `--entry`. Use `--column` for an observed input or
produced column, or `--transformation` for a step named by [`xpd graph`](../graph/).
The report gives the producer and dependency path for each affected output.
It lists direct callers in the file with the source lines where they use those
outputs and a case to verify. Dynamic lookups and callers outside the file
appear as unresolved boundaries.

## Example

The dashboard reads the renamed `share` column, which depends on `net_amount`.
The audit reads only `order_id`. The dashboard can also select a column at
runtime, so the report flags that lookup.

```console
$ xpd impact samples/graph/impact.py --entry features --column net_amount --format text
samples/graph/impact.py:19: impact of column "net_amount" in features()
...
Output "share": totals [net_amount] (samples/graph/impact.py:7) -> shares [customer_share] (samples/graph/impact.py:12) -> publish_names [share] (samples/graph/impact.py:16)
samples/graph/impact.py:24: dashboard calls features(orders)
samples/graph/impact.py:27: uses share
...
samples/graph/impact.py:26: unresolved [dynamic-lookup] Resolve how result[[column]] uses the pipeline result.
```

## Inline rename

A rename can carry a dependency between two assignments in the same chain.
For this fixture, set `gross_amount` to 30 and change `discount` from 5 to 20
on one row. The dashboard's `share` goes from 50 to 20. The audit reads only
`order_id`, which stays the same. The report infers which inputs to vary and
which reads may change. Run the callers to check the actual values.

```console
$ xpd impact samples/graph/impact.py --entry inline_features --column net_amount --format text
samples/graph/impact.py:34: impact of column "net_amount" in inline_features()
samples/graph/impact.py:36: producer .assign(net_amount) [net_amount]
Output "renamed": .assign(net_amount) [net_amount] (samples/graph/impact.py:36) -> .rename [renamed] (samples/graph/impact.py:37)
Output "share": .assign(net_amount) [net_amount] (samples/graph/impact.py:36) -> .rename [renamed] (samples/graph/impact.py:37) -> .assign(share) [share] (samples/graph/impact.py:38)
samples/graph/impact.py:43: inline_dashboard calls inline_features(orders)
samples/graph/impact.py:43: uses share
  Verify: Inferred input candidates: "discount", "gross_amount". Change one of these columns to a different valid value in one row of orders, keeping all other input values fixed. Run inline_dashboard on both inputs and compare "share" at the listed reads, matching the same rows. Inferred: the caller can observe a change if the producer changes. Exact values and direction are unknown.
samples/graph/impact.py:34: unresolved [external-callers] Look for imports and callers outside this file.
```

## Renames across return branches

When both returns rename the same column on the same frame, the report traces
the renamed output and suggests varying the original input column.

```python title="branch_rename.py"
def publish(orders, alternate=False):
    if alternate:
        return orders.rename(columns={"net_amount": "share"})
    return orders.rename(columns={"net_amount": "share"})


def dashboard(orders):
    return publish(orders)[["share"]]
```

```console
$ xpd impact branch_rename.py --entry publish --transformation publish --format text
branch_rename.py:1: impact of transformation "publish" in publish()
branch_rename.py:3: producer publish [share]
Output "share": publish [share] (branch_rename.py:3)
branch_rename.py:8: dashboard calls publish(orders)
branch_rename.py:8: uses share
  Verify: Inferred input candidates: "net_amount". Change one of these columns to a different valid value in one row of orders, keeping all other input values fixed. Run dashboard on both inputs and compare "share" at the listed reads, matching the same rows. Inferred: the caller can observe a change if the producer changes. Exact values and direction are unknown.
branch_rename.py:1: unresolved [external-callers] Look for imports and callers outside this file.
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The report was generated. It may contain unresolved items. |
| 2 | The arguments were invalid or the source or query could not be analyzed. |
