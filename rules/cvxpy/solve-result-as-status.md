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

Direct truth tests of a solve call in an `if`, `elif`, `while`, or
`assert`, including negated and parenthesized tests. Numeric comparisons
with the objective are valid.

## How to fix it

Solve once and check the status before reading values from the solution.
Include `cp.OPTIMAL_INACCURATE` only if the application can use a solution
with lower accuracy.

[CVXPY guide to infeasible and unbounded problems](https://www.cvxpy.org/tutorial/intro/index.html#infeasible-and-unbounded-problems)
