SciPy lists these optimize functions as legacy and not recommended for new scripts. Use the call that the message names, which returns an `OptimizeResult`.

```diff lang="python"
-xopt = optimize.fmin_bfgs(f, x0, fprime=grad, disp=False)
+xopt = optimize.minimize(f, x0, jac=grad, method="BFGS").x
```

## What the rule reports

The rule reports the functions that `scipy.optimize` lists as legacy and not recommended for new
scripts. Its message names the call that replaces each one:

| Legacy function | Write instead |
| --- | --- |
| `fmin`, `fmin_powell`, `fmin_cg`, `fmin_bfgs`, `fmin_ncg`, `fmin_l_bfgs_b`, `fmin_tnc`, `fmin_cobyla`, `fmin_slsqp` | `minimize(fun, x0, method=...)` with `'Nelder-Mead'`, `'Powell'`, `'CG'`, `'BFGS'`, `'Newton-CG'`, `'L-BFGS-B'`, `'TNC'`, `'COBYLA'` or `'SLSQP'` |
| `fminbound` | `minimize_scalar(fun, bounds=(a, b), method='bounded')` |
| `brent`, `golden` | `minimize_scalar(fun, method='brent')` or `method='golden'` |
| `leastsq` | `least_squares(fun, x0)` |
| `fsolve` | `root(fun, x0)` |
| `broyden1`, `broyden2`, `anderson`, `excitingmixing`, `linearmixing`, `diagbroyden` | `root(fun, x0, method=...)` with the same name |
| `newton_krylov` | `root(fun, x0, method='krylov')` |

## How to fix it

xpd has no automatic fix. Because the newer functions take other argument names and return an OptimizeResult. Make the change by hand.
