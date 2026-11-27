A default CVXPY solve requires a disciplined convex formulation. This rule
checks locally bound scalar variables and parameters, square and square root,
and direct constraint comparisons. A minimization objective must be convex
and a maximization objective must be concave. For an inequality the convex
expression must be on the lower side. Equality requires two affine expressions.

For example, `cp.sqrt(cp.square(x))` on a sign-unknown scalar can be written as
`cp.abs(x)`. The rule skips explicit `qcp=True`, `gp=True`, custom or
unknown solve modes, and curvature it cannot establish from local syntax.

[CVXPY disciplined convex programming](https://www.cvxpy.org/tutorial/dcp/)
