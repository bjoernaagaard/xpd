arch 8.0 or 9.0 removed this form, so the import or the call raises. Use the replacement that the message names.

```diff lang="python"
-from arch.__future__ import reindexing
-
-bs = IIDBootstrap(x, random_state=np.random.RandomState(0))
-bs.seed(1234)
-dist = Normal(random_state=np.random.RandomState(0))
-fig = res.hedgehog_plot(type="mean")
+bs = IIDBootstrap(x, seed=1234)
+dist = Normal(seed=np.random.default_rng(0))
+fig = res.hedgehog_plot(plot_type="mean")
```

## What the rule reports

The rule reports a form that arch 8.0 or 9.0 removed. The table after the example lists every form, with
what it did on the release before the removal and what it does now.

The rule leaves alone an import inside a `try` block that catches the error, code behind a
version check, and code inside `pytest.raises`. The pack has no rule for deprecated API, because
the source of arch 9.0.0 raises no `DeprecationWarning` and no `FutureWarning`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The fix is safe for these forms, because arch 7.2.0 ran the old and the new form and gave equal values or the same object. `seed=` replaces `random_state=` on a distribution or on one of the five bootstraps that 7.2.0 has, and names the one positional argument of a distribution, `bs.state` replaces `bs.get_state()`, `bs.state = s` replaces the statement `bs.set_state(s)`, and `bs.generator` replaces `bs.random_state` on those bootstraps. The import of `cov_nw` from `arch.utility` moves to `arch.utility.cov`, and the other names of the same import stay in a line of their own. The fix is also safe where `plot_type=` replaces `type=` on `hedgehog_plot` of a result that the file gets from an arch model, which drew the same figure with either keyword on 8.0.0. The fix is unsafe for the same bootstrap fixes on a `LocalBlockBootstrap`, which came in 9.0 and never took the old forms, and the deleted `arch.__future__` import. Deleting `import arch.__future__.reindexing` also stops that line from binding `arch`. `seed()` on an object, a distribution with more than one argument, `hedgehog_plot` of an object the file does not tie to arch, the read only attributes and the `nested` and `studentize` pair have no fix.

| Form | Before | Removed in | Now |
| --- | --- | --- | --- |
| `random_state=` on `IIDBootstrap`, `IndependentSamplesBootstrap`, `StationaryBootstrap`, `CircularBlockBootstrap`, `MovingBlockBootstrap` or `LocalBlockBootstrap` | 7.2.0: FutureWarning | 8.0.0 | TypeError, because `random_state` is read as data |
| `random_state=` or a positional argument on `Normal`, `StudentsT`, `SkewStudent` or `GeneralizedError` | 7.2.0: FutureWarning | 8.0.0 | TypeError |
| `bs.seed(1)`, `bs.get_state()`, `bs.set_state(s)` and `bs.random_state` on a bootstrap | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `seed` on `SPA`, `StepM`, `MCS` or `RealityCheck` | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `from arch.utility import cov_nw` | 7.2.0: runs | 8.0.0 | ImportError |
| `from arch.__future__ import reindexing` | 8.0.0: runs and does nothing | 9.0.0 | ModuleNotFoundError |
| `res.hedgehog_plot(type="mean")` | 8.0.0: FutureWarning | 9.0.0 | TypeError |
| an assignment to `block_size` of a block bootstrap, or to `bootstrap` of a multiple comparison test | 8.0.0: runs | 9.0.0 | AttributeError |
| `nested=True` with `studentize=False` on `SPA`, `StepM` or `RealityCheck` | 8.0.0: runs | 9.0.0 | ValueError |

The bootstraps take their data as keyword arguments, so 8.0.0 and 9.0.0 read `random_state` as
one more series, and they raise because a `RandomState` is not an array. `seed` takes the same
`RandomState`, a `Generator` or an integer. It exists since arch 5.0, so on 4.19
`IIDBootstrap(x, seed=0)` raised TypeError. `IIDBootstrap(x, seed=np.random.RandomState(0))` drew
the same samples on 6.1.0, 7.2.0, 8.0.0 and 9.0.0. The `state` property reads and sets the
state of the generator, and `generator` gives the generator itself.

Other objects also have the names `seed`, `get_state`, `set_state`, `random_state`, `block_size`
and `bootstrap`, such as a NumPy `RandomState`. The rule reports them only on a variable that the
file assigns from one of the arch classes and from nothing else. It reports `hedgehog_plot` on
any object, because no other library that was looked at has a method with the name. It fixes the
call only on a result that the file gets from `fit` or `fix` of an arch model.
