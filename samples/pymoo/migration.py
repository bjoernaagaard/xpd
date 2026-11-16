"""Imports and names that older pymoo releases had, and the forms that replace them on pymoo 0.6.2.

A line that xpd reports ends with a comment that names the rule. A removed form raises on pymoo
0.6.2, so it sits in a function that is not a case and that nothing calls. Each case returns what
the code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.core.problem import ElementwiseProblem
from pymoo.indicators.gd import GD
from pymoo.indicators.hv import HV
from pymoo.indicators.igd import IGD
from pymoo.optimize import minimize
from pymoo.problems import get_problem
from pymoo.termination import get_termination

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def factory_functions():
    from pymoo.factory import get_problem  # xpd: removed-in-pymoo
    from pymoo.factory import get_termination  # xpd: removed-in-pymoo
    from pymoo.factory import get_reference_directions  # xpd: removed-in-pymoo
    from pymoo.factory import get_problem as problem_by_name, get_termination as termination_by_name  # xpd: removed-in-pymoo, removed-in-pymoo
    from pymoo.factory import get_sampling, get_crossover, get_mutation  # xpd: removed-in-pymoo, removed-in-pymoo, removed-in-pymoo
    from pymoo.factory import get_selection, get_algorithm  # xpd: removed-in-pymoo, removed-in-pymoo
    from pymoo.factory import get_performance_indicator, get_decision_making  # xpd: removed-in-pymoo, removed-in-pymoo
    from pymoo.factory import get_decomposition, get_visualization  # xpd: removed-in-pymoo, removed-in-pymoo
    from pymoo.factory import get_crossover_options  # xpd: removed-in-pymoo
    import pymoo.factory  # xpd: removed-in-pymoo

    return get_problem, get_termination, get_reference_directions, problem_by_name, termination_by_name, get_sampling, get_crossover, get_mutation, get_selection, get_algorithm, get_performance_indicator, get_decision_making, get_decomposition, get_visualization, get_crossover_options, pymoo.factory


def removed_modules():
    from pymoo.model.problem import Problem  # xpd: removed-in-pymoo
    from pymoo.model.callback import Callback  # xpd: removed-in-pymoo
    from pymoo.algorithms.nsga2 import NSGA2 as OldNSGA2  # xpd: removed-in-pymoo
    from pymoo.algorithms.so_genetic_algorithm import GA  # xpd: removed-in-pymoo
    from pymoo.operators.crossover.simulated_binary_crossover import SimulatedBinaryCrossover  # xpd: removed-in-pymoo
    from pymoo.performance_indicator.hv import Hypervolume  # xpd: removed-in-pymoo
    from pymoo.decision_making.pseudo_weights import PseudoWeights  # xpd: removed-in-pymoo
    from pymoo.algorithms.soo.nonconvex.nelder_mead import NelderMead  # xpd: removed-in-pymoo
    from pymoo.util.termination.max_gen import MaximumGenerationTermination  # xpd: removed-in-pymoo
    from pymoo.util.function_loader import is_compiled  # xpd: removed-in-pymoo
    from pymoo.util.nds.find_non_dominated import find_non_dominated  # xpd: removed-in-pymoo

    return Problem, Callback, OldNSGA2, GA, SimulatedBinaryCrossover, Hypervolume, PseudoWeights, NelderMead, MaximumGenerationTermination, is_compiled, find_non_dominated


def removed_modules_without_a_new_home():
    import pymoo.model.problem  # xpd: removed-in-pymoo
    from pymoo.model.problem import Problem, MetaProblem  # xpd: removed-in-pymoo
    from pymoo.configuration import Configuration  # xpd: removed-in-pymoo
    from pymoo.util.termination.default import MultiObjectiveDefaultTermination  # xpd: removed-in-pymoo
    from pymoo.operators.mixed_variable_operator import MixedVariableSampling  # xpd: removed-in-pymoo
    from pymoo.operators.integer_from_float_operator import IntegerFromFloatMutation  # xpd: removed-in-pymoo
    from pymoo.interface import sample  # xpd: removed-in-pymoo
    from pymoo.indicators.hv.exact import ExactHypervolume  # xpd: removed-in-pymoo

    return pymoo.model.problem, Problem, MetaProblem, Configuration, MultiObjectiveDefaultTermination, MixedVariableSampling, IntegerFromFloatMutation, sample, ExactHypervolume


def removed_names():
    from pymoo.util.display import Display  # xpd: removed-in-pymoo
    from pymoo.core.problem import ElementwiseProblem as Elementwise, starmap_parallelized_eval  # xpd: removed-in-pymoo
    from pymoo.operators.mutation.bitflip import BinaryBitflipMutation  # xpd: removed-in-pymoo
    from pymoo.core.problem import DaskParallelization  # xpd: removed-in-pymoo
    from pymoo.algorithms.moo.nsga2 import calc_crowding_distance  # xpd: removed-in-pymoo

    return Display, Elementwise, starmap_parallelized_eval, BinaryBitflipMutation, DaskParallelization, calc_crowding_distance


def removed_termination_names():
    by_name = get_termination("f_tol", tol=0.01)  # xpd: removed-in-pymoo
    design = get_termination("x_tol")  # xpd: removed-in-pymoo
    res = minimize(get_problem("zdt1"), NSGA2(pop_size=20), ("f_tol", 0.01), seed=1)  # xpd: removed-in-pymoo
    keyword = minimize(get_problem("zdt1"), NSGA2(pop_size=20), termination=("default",), seed=1)  # xpd: removed-in-pymoo
    return by_name, design, res, keyword


def calc_on_an_indicator():
    pf = get_problem("zdt1").pareto_front()
    direct = IGD(pf).calc(pf[::10])  # xpd: removed-in-pymoo
    ind = HV(ref_point=np.array([1.1, 1.1]))
    held = ind.calc(pf)  # xpd: removed-in-pymoo
    return direct, held


def starmap_from_the_old_module():
    from pymoo.core.problem import StarmapParallelization  # xpd: deprecated-in-pymoo
    from pymoo.core.problem import Problem, StarmapParallelization as Starmap  # xpd: deprecated-in-pymoo

    return StarmapParallelization, Problem, Starmap


@case
def imports_that_still_work():
    from pymoo.algorithms.soo.nonconvex.ga import GA
    from pymoo.config import Config
    from pymoo.core.callback import Callback
    from pymoo.core.problem import Problem
    from pymoo.functions import is_compiled
    from pymoo.parallelization import StarmapParallelization
    from pymoo.termination.default import DefaultMultiObjectiveTermination
    from pymoo.util.display.output import Output
    from pymoo.util.ref_dirs import get_reference_directions

    try:
        from pymoo.factory import get_sampling
    except ImportError:
        get_sampling = None
    names = [GA, Config, Callback, Problem, StarmapParallelization, DefaultMultiObjectiveTermination, Output, get_reference_directions]
    return [n.__name__ for n in names], get_sampling, bool(is_compiled())


@case
def termination_names_of_pymoo_06():
    by_name = get_termination("n_gen", 5)
    res = minimize(get_problem("zdt1"), NSGA2(pop_size=20), ("n_gen", 5), seed=1)
    settings = {"x_tol": 1e-8, "default": True}
    return type(by_name).__name__, res.F, settings.get("x_tol"), ("f_tol", 0.01)


@case
def indicators_called():
    pf = get_problem("zdt1").pareto_front()
    direct = IGD(pf)(pf[::10])
    ind = GD(pf)
    held = ind.do(pf[::10])

    class Price:
        def calc(self, amount):
            return amount * 2

    price = Price()
    return direct, held, price.calc(3)


@case
def feasible_key():
    res = minimize(get_problem("bnh"), NSGA2(pop_size=20), ("n_gen", 5), seed=1)
    old = res.pop.get("feasible")  # xpd: deprecated-in-pymoo
    pair = res.pop.get("F", "feasible")[1]  # xpd: deprecated-in-pymoo
    pop = res.pop
    by_name = pop.get("feasible")  # xpd: deprecated-in-pymoo
    new = res.pop.get("FEAS")
    settings = {"feasible": True}
    return old, pair, by_name, new, settings.get("feasible", False), settings.get("feasible")


class Sphere(ElementwiseProblem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum()
