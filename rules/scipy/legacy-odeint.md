SciPy's docs say new code should use `solve_ivp` instead of `odeint`. Switch to `solve_ivp`, and note that it takes `t` before `y`.

```diff lang="python"
-ys = odeint(lambda y, t: -k * y, y0, t)
+ys = solve_ivp(lambda t, y: -k * y, (t[0], t[-1]), y0, t_eval=t).y.T
```

## What the rule reports

The rule reports `integrate.odeint`, because SciPy's docs ask new code to use `solve_ivp`.
Neither solver was faster on every problem measured, so the message makes no claim about speed.
The message also tells you what changes when you switch:

- `solve_ivp` raises ValueError when `t_eval` repeats a time, which `odeint` accepts.
- The values differ in the last digits.
- Keep the default method of `solve_ivp` for a large system, because `method='LSODA'` stores a
  full Jacobian and raised MemoryError at 1,000,000 equations.

## How to fix it

xpd has no automatic fix. Because solve_ivp calls `fun(t, y)` where odeint calls `func(y, t)`, and returns an object. Make the change by hand.
