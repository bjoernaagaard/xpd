This call draws random numbers from a new seed on every run, and `np.random.seed` does not reach it, so results change from run to run. Pass `seed`.

```diff lang="python"
-np.random.seed(0)
-bs = StationaryBootstrap(10, returns)
+bs = StationaryBootstrap(10, returns, seed=0)
 interval = bs.conf_int(np.mean, 1000)
```

## What the rule reports

The rule reports a call that draws random numbers without a seed.

| Call | Reported without |
| --- | --- |
| `IIDBootstrap`, `IndependentSamplesBootstrap`, `StationaryBootstrap`, `CircularBlockBootstrap`, `MovingBlockBootstrap`, `LocalBlockBootstrap` | `seed` |
| `SPA`, `StepM`, `MCS`, `RealityCheck` | `seed` |
| `forecast` with `method="simulation"` | `rng`, unless the file builds a distribution with a seed |
| `forecast` with `method="bootstrap"` | `random_state` |
| `simulate` on an arch model | a distribution with a seed anywhere in the file |

The rule leaves alone a bootstrap whose `state` or `generator` the file assigns. On arch 9.0.0,
8.0.0 and 5.0, `bs.generator = np.random.default_rng(0)` gave the same interval as `seed=0`. The
two documents that read the model's distribution leave alone a file that builds a distribution
with `seed=` anywhere, because a pattern cannot see which model gets it. `SPA`, `StepM`, `MCS`
and `RealityCheck` are read only when the file imports them from `arch.bootstrap`.

## How to fix it

xpd has no automatic fix. The seed is not in the file. Make the change by hand.

arch draws from generators of its own, which start from a new seed in every process. Each form
was run twice without a seed and twice with one, on arch 9.0.0 and on 8.0.0. The two runs
without a seed differed on both releases, and the table gives the numbers of 9.0.0.

| Form | Two runs without a seed | Two runs after `np.random.seed(0)` | Two runs with a seed | Seeded by |
| --- | --- | --- | --- | --- |
| `IIDBootstrap(x).conf_int(np.mean, 200)`, lower bound | -0.15601, -0.15855 | -0.1328, -0.15246 | -0.15667 twice | `seed=0` |
| `StationaryBootstrap(10, x)`, the same | -0.13476, -0.13188 | not run | -0.14428 twice | `seed=0` |
| `CircularBlockBootstrap(10, x)`, the same | -0.18054, -0.14953 | not run | -0.15445 twice | `seed=0` |
| `MovingBlockBootstrap(10, x)`, the same | -0.14193, -0.14765 | not run | -0.16665 twice | `seed=0` |
| `SPA(...)`, lower p-value | 0.7233, 0.6800 | not run | 0.6700 twice | `seed=0` |
| `MCS(...)`, first p-value | 0.250, 0.283 | not run | 0.297 twice | `seed=0` |
| `forecast(horizon=5, method="simulation")`, last step | 2.671, 2.601 | 2.676, 2.534 | 2.571 twice | `Normal(seed=0)` as the model's distribution, or a seeded `rng` |
| `forecast(horizon=5, method="bootstrap")`, last step | 2.587, 2.578 | 2.468, 2.737 | 2.595 twice | `random_state=np.random.RandomState(0)` |
| `arch_model(None).simulate(params, 100)`, last value | -1.24, 0.07 | -1.27, 0.30 | 0.097 twice | `Normal(seed=0)` as the model's distribution |

arch does not draw from NumPy's global generator, so `np.random.seed(0)` did not make any two
runs equal. Of the 291 reports on the two sets of code, 49 are in a file that calls
`np.random.seed`. One
report has wrong advice, on a placeholder bootstrap that a base class in arch's own source
builds and never draws from.

`seed` is a keyword of the bootstraps, the tests and the distributions since arch 5.0, so every
document carries `since: "5.0"`. The rule is info, because a seed does not move what the method
estimates.
