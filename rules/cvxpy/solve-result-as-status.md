CVXPY's `Problem.solve()` returns the objective value. An optimal objective
of zero is false in Python. An infinite objective for an infeasible or
unbounded problem is true.

```diff lang="python"
-if problem.solve():
+problem.solve()
+if problem.status == cp.OPTIMAL:
     use_solution()
```

## What the rule reports

The rule checks direct truth tests in `if`, `elif`, `while`, `assert`,
conditional expressions, and calls to the built-in `bool`. It follows
parentheses and negation. Numeric comparisons and direct uses as assert
messages are outside these truth tests.

## How to fix it

Solve once and check the status before reading values from the solution.
Include `cp.OPTIMAL_INACCURATE` only if the application can use a solution
with lower accuracy.

[CVXPY guide to infeasible and unbounded problems](https://www.cvxpy.org/tutorial/intro/index.html#infeasible-and-unbounded-problems)
