`minimize` ignores an argument such as `jac=` when the chosen method does not use it, and warns. Delete the argument, or choose a method that uses it.

```diff lang="python"
-res = minimize(f, x0, method="BFGS", jac=grad, bounds=[(0, 1)] * n)
+res = minimize(f, x0, method="L-BFGS-B", jac=grad, bounds=[(0, 1)] * n)
```

## What the rule reports

The rule reports an argument of `optimize.minimize` that the chosen method ignores. On SciPy
1.18.1, minimize warns RuntimeWarning and goes on without the argument.

| Argument | Methods that ignore it |
| --- | --- |
| `jac=` | Nelder-Mead, Powell, COBYLA, COBYQA |
| `hess=`, `hessp=` | each method that uses no Hessian |
| `constraints=` | each method that can't handle constraints |
| `bounds=` | CG, BFGS, Newton-CG, and the trust-region methods other than trust-constr |

## How to fix it

xpd has no automatic fix, so make the change by hand.
