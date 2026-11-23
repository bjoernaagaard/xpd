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

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The report was generated. It may contain unresolved items. |
| 2 | The arguments were invalid or the source or query could not be analyzed. |
