"""Panel models that fit another model than the code reads as, or the same one much slower.

A line that xpd reports ends with a comment that names the rule. A form that raises on
linearmodels 7.1 sits in a function that is not a case and that nothing calls. Each case returns
what the code computes.
"""

import linearmodels as lm
import statsmodels.api as sm
from linearmodels.datasets import wage_panel
from linearmodels.panel import PanelOLS, PooledOLS

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def table():
    return wage_panel.load()


@case
def index_order():
    data = table()
    regressors = ["hours", "married"]
    swapped = data.set_index(["year", "nr"])  # xpd: time-before-entity-index
    by_year = PanelOLS(swapped.lwage, swapped[regressors], entity_effects=True).fit(cov_type="robust")
    right = data.set_index(["nr", "year"])
    by_worker = PanelOLS(right.lwage, right[regressors], entity_effects=True).fit(cov_type="robust")
    direct = PooledOLS.from_formula("lwage ~ 1 + hours", data.set_index(["year", "nr"])).fit(cov_type="robust")  # xpd: time-before-entity-index
    chained = data.rename(columns={"year": "date"}).set_index(["date", "nr"], drop=False).sort_index()  # xpd: time-before-entity-index
    by_date = PooledOLS(chained.lwage, chained[regressors]).fit(cov_type="robust")
    # A frame that no panel model reads may have any index.
    lookup = data.set_index(["year", "nr"])
    # A later statement that changes the index of the same name leaves the order open.
    turned = data.set_index(["year", "nr"])
    turned = turned.swaplevel().sort_index()
    after = PanelOLS(turned.lwage, turned[regressors], entity_effects=True).fit(cov_type="robust")
    both_times = data.assign(month=data.nr).set_index(["year", "month"])
    by_month = PooledOLS(both_times.lwage, both_times[regressors]).fit(cov_type="robust")
    columns = ["year", "nr"]
    from_a_name = data.set_index(columns)
    named = PooledOLS(from_a_name.lwage, from_a_name[regressors]).fit(cov_type="robust")
    return [r.params for r in (by_year, by_worker, direct, by_date, after, by_month, named)], lookup.shape


@case
def pooled_by_default():
    data = table().set_index(["nr", "year"])
    y, X = data.lwage, sm.add_constant(data[["exper", "hours"]])
    pooled = PanelOLS(y, X).fit(cov_type="robust")  # xpd: panel-ols-without-effects
    module = lm.PanelOLS(y, X, weights=data.hours + 1).fit(cov_type="robust")  # xpd: panel-ols-without-effects
    fixed = PanelOLS(y, X, entity_effects=True).fit(cov_type="robust")
    yearly = PanelOLS(y, X[["const", "hours"]], time_effects=True).fit(cov_type="robust")
    other = PanelOLS(y, X, other_effects=data.occupation).fit(cov_type="robust")
    switched_off = PanelOLS(y, X, entity_effects=False).fit(cov_type="robust")
    options = {"entity_effects": True}
    unpacked = PanelOLS(y, X, **options).fit(cov_type="robust")
    said = PooledOLS(y, X).fit(cov_type="robust")
    return [r.params for r in (pooled, module, fixed, yearly, other, switched_off, unpacked, said)]


@case
def dummy_columns(entities=60):
    data = table().set_index(["nr", "year"])
    data = data.loc[data.index.get_level_values(0).unique()[:entities]]
    model = PanelOLS(data.lwage, data[["hours", "married"]], entity_effects=True)
    slow = model.fit(use_lsdv=True, cov_type="robust")  # xpd: use-lsdv
    chained = PanelOLS(data.lwage, data[["hours"]], entity_effects=True).fit(cov_type="robust", use_lsdv=True)  # xpd: use-lsdv
    fast = model.fit(cov_type="robust")
    switched_off = model.fit(use_lsdv=False, cov_type="robust")
    flag = True
    from_a_name = model.fit(use_lsdv=flag, cov_type="robust")
    return [r.params for r in (slow, chained, fast, switched_off, from_a_name)]


def raises_on_7_1():
    data = table().set_index(["nr", "year"])
    y, X = data.lwage, data[["hours"]]
    PanelOLS(y, X, entity_effects=True, time_effects=True, other_effects=data.occupation)  # xpd: too-many-effects
    PanelOLS(y, X, entity_effects=True, time_effects=True)
    PanelOLS(y, X, entity_effects=True, other_effects=data.occupation)
    PanelOLS(y, X, entity_effects=True, time_effects=True, other_effects=None)
    lm.FirstDifferenceOLS(y, sm.add_constant(X))  # xpd: first-difference-constant
    lm.FirstDifferenceOLS(y, exog=sm.add_constant(X))  # xpd: first-difference-constant
    lm.FirstDifferenceOLS(y, X)
    lm.PooledOLS(y, sm.add_constant(X))
