"""Calls to minimize and to the algorithms that do something else than the code says, and the forms that work.

A line that xpd reports ends with a comment that names the rule. A form that raises on pymoo
0.6.2 sits in a function that is not a case and that nothing calls. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import pickle

from pymoo.algorithms.moo.moead import MOEAD
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.core.callback import Callback
from pymoo.decomposition.pbi import PBI
from pymoo.operators.mutation.bitflip import BitflipMutation
from pymoo.operators.mutation.pm import PM, PolynomialMutation
from pymoo.optimize import minimize
from pymoo.problems import get_problem
from pymoo.termination import get_termination
from pymoo.termination.max_gen import MaximumGenerationTermination
from pymoo.util.ref_dirs import get_reference_directions

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


@case
def termination_given_to_setup():
    problem = get_problem("zdt1")
    old = NSGA2(pop_size=20)
    old.setup(problem, ("n_gen", 7), seed=1)  # xpd: setup-positional-termination
    by_name = NSGA2(pop_size=20)
    by_name.setup(problem, get_termination("n_gen", 7), seed=1)  # xpd: setup-positional-termination
    new = NSGA2(pop_size=20)
    new.setup(problem, termination=("n_gen", 7), seed=1)
    while new.has_next():
        new.next()
    box = Box()
    box.setup(problem, ("n_gen", 7))
    return type(old.termination).__name__, type(by_name.termination).__name__, new.n_gen, box.args


class Box:
    def setup(self, *args):
        self.args = len(args)


@case
def algorithm_read_after_the_run():
    problem = get_problem("zdt1")
    algorithm = NSGA2(pop_size=20, callback=Best())
    res = minimize(problem, algorithm, ("n_gen", 10), seed=1)
    left = algorithm.pop  # xpd: algorithm-read-after-minimize
    records = len(algorithm.callback.data["best"])  # xpd: algorithm-read-after-minimize
    if algorithm.n_gen is None:  # xpd: algorithm-read-after-minimize
        left = algorithm.opt  # xpd: algorithm-read-after-minimize
    ran = len(res.algorithm.pop), len(res.algorithm.callback.data["best"]), res.algorithm.n_gen
    size = algorithm.pop_size
    return left, records, ran, size, [algorithm.n_gen for algorithm in [res.algorithm]]


@case
def algorithm_run_in_place_or_built_again():
    problem = get_problem("zdt1")
    in_place = NSGA2(pop_size=20)
    kept = minimize(problem, in_place, ("n_gen", 10), seed=1, copy_algorithm=False)
    again = NSGA2(pop_size=20)
    first = minimize(problem, again, ("n_gen", 5), seed=1)
    again = NSGA2(pop_size=30)
    stepped = NSGA2(pop_size=20)
    stepped.setup(problem, termination=("n_gen", 5), seed=1)
    done = minimize(problem, stepped, ("n_gen", 5), seed=1)
    return len(in_place.pop), kept.algorithm is in_place, first.F, again.pop, stepped.pop, done.F


@case
def algorithm_bound_again_by_a_walrus():
    walrus = NSGA2(pop_size=20)
    res = minimize(get_problem("zdt1"), walrus, ("n_gen", 5), seed=1)
    # The walrus gives the name the algorithm that ran.
    generations = (walrus := res.algorithm).n_gen
    return generations, walrus.n_gen


def notify(algorithm):
    return algorithm.pop


@case
def second_run_without_a_copy():
    algorithm = NSGA2(pop_size=20)
    first = minimize(get_problem("zdt1"), algorithm, ("n_gen", 10), seed=1, copy_algorithm=False)
    second = minimize(get_problem("zdt2"), algorithm, ("n_gen", 30), seed=2, copy_algorithm=False)  # xpd: reused-algorithm-without-copy
    looped = NSGA2(pop_size=20)
    fronts = []
    for name in ("zdt1", "zdt2"):
        fronts.append(minimize(get_problem(name), looped, ("n_gen", 5), seed=1, copy_algorithm=False).F)  # xpd: reused-algorithm-without-copy
    return first.F, second.F, second.algorithm.n_gen, fronts


@case
def runs_with_a_new_algorithm():
    algorithm = NSGA2(pop_size=20)
    first = minimize(get_problem("zdt1"), algorithm, ("n_gen", 10), seed=1, copy_algorithm=False)
    algorithm = NSGA2(pop_size=20)
    second = minimize(get_problem("zdt2"), algorithm, ("n_gen", 10), seed=2, copy_algorithm=False)
    copied = NSGA2(pop_size=20)
    third = minimize(get_problem("zdt1"), copied, ("n_gen", 5), seed=1)
    fourth = minimize(get_problem("zdt2"), copied, ("n_gen", 5), seed=1)
    fronts = []
    for name in ("zdt1", "zdt2"):
        fresh = NSGA2(pop_size=20)
        fronts.append(minimize(get_problem(name), fresh, ("n_gen", 5), seed=1, copy_algorithm=False).F)
    # A loop over the algorithms runs another object in each pass.
    for each in [NSGA2(pop_size=20), NSGA2(pop_size=30)]:
        fronts.append(minimize(get_problem("zdt1"), each, ("n_gen", 5), seed=1, copy_algorithm=False).F)
    return first.F, second.F, third.F, fourth.F, fronts


@case
def stopping_rule_under_another_name():
    problem = get_problem("zdt1")
    stepper = NSGA2(pop_size=20)
    stepper.setup(problem, seed=1, n_gen=7)
    asked = minimize(problem, NSGA2(pop_size=20), termination=("n_gen", 7), seed=1)
    return type(stepper.termination).__name__, asked.algorithm.n_gen


def run_that_does_not_stop_where_asked():
    problem = get_problem("zdt1")
    long = minimize(problem, NSGA2(pop_size=20), n_gen=7, seed=1)  # xpd: minimize-ignored-keyword
    longer = minimize(problem, NSGA2(pop_size=20), seed=1, generations=7)  # xpd: minimize-ignored-keyword
    twice = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), n_gen=7, seed=1)  # xpd: minimize-ignored-keyword
    return long, longer, twice


@case
def seed_under_another_name():
    problem = get_problem("zdt1")
    a = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), random_state=1)  # xpd: minimize-ignored-keyword
    b = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1)
    return a.F.shape[1], b.F


def continue_a_finished_run():
    with open("checkpoint.pkl", "rb") as f:
        checkpoint = pickle.load(f)
    checkpoint.has_terminated = False  # xpd: has-terminated-assignment
    checkpoint.termination = MaximumGenerationTermination(20)
    return minimize(get_problem("zdt1"), checkpoint, seed=1, copy_algorithm=False)


class Job:
    def __init__(self):
        self.has_terminated = False


@case
def another_object_with_the_attribute():
    # A job is not an algorithm that the file loads, so its attribute is its own.
    job = Job()
    job.has_terminated = False
    return job.has_terminated


@case
def moead_arguments():
    ref_dirs = get_reference_directions("das-dennis", 3, n_partitions=6)
    algorithm = MOEAD(ref_dirs, n_neighbors=10, decomposition=PBI(), prob_neighbor_mating=0.7)
    res = minimize(get_problem("dtlz2"), algorithm, ("n_gen", 5), seed=1)
    return res.F, algorithm.pop_size, NSGA2(pop_size=50).pop_size


def moead_arguments_that_raise():
    ref_dirs = get_reference_directions("das-dennis", 3, n_partitions=6)
    sized = MOEAD(ref_dirs, pop_size=50)  # xpd: moead-arguments
    named = MOEAD(ref_dirs, decomposition="pbi")  # xpd: moead-arguments
    chosen = MOEAD(ref_dirs, decomposition="auto")  # xpd: moead-arguments
    return sized, named, chosen


@case
def mutation_rate():
    problem = get_problem("zdt1")
    per_individual = PM(prob=1 / problem.n_var, eta=20)  # xpd: mutation-prob-per-individual
    low = PolynomialMutation(prob=0.05, eta=20)  # xpd: mutation-prob-per-individual
    bits = BitflipMutation(prob=1.0 / 30)  # xpd: mutation-prob-per-individual
    per_variable = PM(prob_var=1 / problem.n_var, eta=20)
    both = PM(prob=0.1, prob_var=0.1, eta=20)
    every = PM(prob=1.0, eta=20)
    rate = 1 / problem.n_var
    named = PM(prob=rate, eta=20)
    res = minimize(problem, NSGA2(pop_size=20, mutation=per_variable), ("n_gen", 5), seed=1)
    return [type(m).__name__ for m in (per_individual, low, bits, both, every, named)], res.F
