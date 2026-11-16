"""A run that keeps a copy of the algorithm for every generation and never looks at the copies.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.core.callback import Callback
from pymoo.optimize import minimize
from pymoo.problems import get_problem

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


class Best(Callback):
    def __init__(self):
        super().__init__()
        self.data["best"] = []

    def notify(self, algorithm):
        self.data["best"].append(algorithm.pop.get("F").min())


problem = get_problem("zdt1")
kept = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1, save_history=True)  # xpd: unread-history
off = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1, save_history=False)
recorded = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1, callback=Best())


def run_for_a_caller():
    return minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1, save_history=True)


@case
def copies_kept_and_values_recorded():
    return kept.F, off.F, recorded.algorithm.callback.data["best"], run_for_a_caller().F
