Use `xpd map` to plan a native Polars reimplementation of a named pandas
pipeline. It reads the Python source without importing or running it.

The plan lists observed column reads, writes and dependencies by stage.
It proposes native expressions, windows and lazy scans where they fit.
It flags unresolved calls, dynamic column names, index behavior and
observable writes that need attention before the rewrite.

Supply `--contract` to declare ordered input and output schemas,
positional row identity and the output materialization boundary. The
[comparison contract](../equiv/) also tells `xpd equiv` how to compare
the native Polars result with the original pandas result on the same
data. `map` checks observed column names against the declared schemas.
`equiv` verifies the actual returned columns, order, types and values.
A complete plan still needs that comparison.

## Example

```console
$ xpd map samples/pipeline/features.py --entry add_features --from pandas --to polars --format text
samples/pipeline/features.py:28: add_features (pandas to polars, partial)
Observed input columns: bill_depth_mm, bill_length_mm, body_mass_g, flipper_length_mm
...
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The plan has no unresolved items. |
| 1 | The plan is partial and lists the evidence still needed. |
| 2 | The arguments, source file or contract could not be read or analyzed. |
