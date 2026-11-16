"""A file that seeds NumPy, which fixed a pymoo run up to 0.6.1.5 and does not since 0.6.1.6.

A line that xpd reports ends with a comment that names the rule. This file also reads the
history that it saves, so xpd says nothing about save_history here.
"""

import numpy as np
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.optimize import minimize
from pymoo.problems import get_problem

np.random.seed(1)  # xpd: numpy-seed-ignored, legacy-random
problem = get_problem("zdt1")
res = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), save_history=True)
best_per_generation = [entry.opt.get("F").min() for entry in res.history]
