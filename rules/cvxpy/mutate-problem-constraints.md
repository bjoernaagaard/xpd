CVXPY's `Problem.constraints` returns a shallow copy of the list. Appending
to that copy leaves the next solve unaware of the added constraint.

```diff lang="python"
-problem.constraints.append(x >= 1)
+problem = cp.Problem(problem.objective, problem.constraints + [x >= 1])
```

## What the rule reports

The rule checks standalone list mutators such as `append` and `clear`,
and writes or deletions through an index or slice of `problem.constraints`.
A list saved to a variable can be edited and passed to a new problem.

## How to fix it

Assemble the constraints before constructing the problem, or build a new
problem with the updated list. A new problem has its own compilation cache.

[CVXPY guide to changing a problem](https://www.cvxpy.org/tutorial/intro/index.html#changing-the-problem)
