"""Formulas that linearmodels reads in another way than statsmodels, next to the forms it means.

A line that xpd reports ends with a comment that names the rule. A form that raises on
linearmodels 7.1 sits in a function that is not a case and that nothing calls. Each case returns
what the code computes.
"""

import statsmodels.formula.api as smf
from linearmodels import IV2SLS, SUR, BetweenOLS, FamaMacBeth, FirstDifferenceOLS, PanelOLS, PooledOLS, RandomEffects
from linearmodels.datasets import mroz, wage_panel

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def panel():
    return wage_panel.load().set_index(["nr", "year"])


def women():
    return mroz.load().dropna().reset_index(drop=True)


@case
def no_intercept():
    data = panel()
    through_origin = PooledOLS.from_formula("lwage ~ exper", data).fit()  # xpd: formula-without-constant, panel-default-covariance
    with_intercept = PooledOLS.from_formula("lwage ~ 1 + exper", data).fit(cov_type="robust")
    meant = PooledOLS.from_formula("lwage ~ exper - 1", data).fit(cov_type="robust")
    also_meant = PooledOLS.from_formula("lwage ~ 0 + exper", data).fit(cov_type="robust")
    own_column = PooledOLS.from_formula("lwage ~ const + exper", data.assign(const=1.0)).fit(cov_type="robust")
    dummies = PooledOLS.from_formula("lwage ~ C(occupation) + exper", data).fit(cov_type="robust")
    random = RandomEffects.from_formula("lwage ~ exper", data).fit(cov_type="robust")  # xpd: formula-without-constant
    between = BetweenOLS.from_formula(formula="lwage ~ exper", data=data).fit()  # xpd: formula-without-constant
    fama = FamaMacBeth.from_formula("lwage ~ exper + hours", data).fit()  # xpd: formula-without-constant
    absorbed = PanelOLS.from_formula("lwage ~ exper + hours + EntityEffects", data).fit(cov_type="robust")
    other = PanelOLS.from_formula("lwage ~ hours", data, other_effects=data.occupation).fit(cov_type="robust")
    differenced = FirstDifferenceOLS.from_formula("lwage ~ hours", data).fit(cov_type="robust")
    # statsmodels adds the intercept itself, so its formula is left alone.
    patsy = smf.ols("lwage ~ exper", data=data.reset_index()).fit()
    text = "lwage ~ exper"
    from_a_name = PooledOLS.from_formula(text, data).fit(cov_type="robust")
    results = (through_origin, with_intercept, meant, also_meant, own_column, dummies, random, between, fama, absorbed, other, differenced, from_a_name)
    return [r.params for r in results], patsy.params


@case
def no_intercept_instruments():
    data = women()
    through_origin = IV2SLS.from_formula("lwage ~ exper + [educ ~ fatheduc + motheduc]", data).fit()  # xpd: formula-without-constant
    with_intercept = IV2SLS.from_formula("lwage ~ 1 + exper + [educ ~ fatheduc + motheduc]", data).fit()
    system = SUR.from_formula({"wage": "lwage ~ exper", "hours": "hours ~ 1 + educ"}, data).fit()  # xpd: formula-without-constant
    whole = SUR.from_formula({"wage": "lwage ~ 1 + exper", "hours": "hours ~ 1 + educ"}, data).fit()
    # Without a block in brackets the instrument models add the intercept themselves.
    added = IV2SLS.from_formula("lwage ~ exper", data).fit()
    return through_origin.params, with_intercept.params, system.params, whole.params, added.params


@case
def dropped_effects():
    data = panel()
    pooled = PooledOLS.from_formula("lwage ~ 1 + hours + EntityEffects", data).fit(cov_type="robust")  # xpd: effects-outside-panel-ols
    random = RandomEffects.from_formula("lwage ~ 1 + hours + TimeEffects", data).fit(cov_type="robust")  # xpd: effects-outside-panel-ols
    between = BetweenOLS.from_formula("lwage ~ 1 + hours + FixedEffects", data).fit()  # xpd: effects-outside-panel-ols
    fama = FamaMacBeth.from_formula("lwage ~ 1 + hours + EntityEffects", data).fit()  # xpd: effects-outside-panel-ols
    differenced = FirstDifferenceOLS.from_formula("lwage ~ hours + TimeEffects", data).fit(cov_type="robust")  # xpd: effects-outside-panel-ols
    fixed = PanelOLS.from_formula("lwage ~ 1 + hours + EntityEffects", data).fit(cov_type="robust")
    plain = PooledOLS.from_formula("lwage ~ 1 + hours", data).fit(cov_type="robust")
    return [r.params for r in (pooled, random, between, fama, differenced, fixed, plain)]


@case
def power_terms():
    data = women()
    interaction = IV2SLS.from_formula("lwage ~ 1 + exper + exper**2", data).fit()  # xpd: formula-power-term
    spaced = IV2SLS.from_formula("lwage ~ 1 + exper ** 2 + educ", data).fit()  # xpd: formula-power-term
    squared = IV2SLS.from_formula("lwage ~ 1 + exper + I(exper**2)", data).fit()
    crossed = IV2SLS.from_formula("lwage ~ 1 + (exper + educ)**2", data).fit()
    patsy = smf.ols("lwage ~ exper + exper**2", data=data).fit()
    return [list(r.params.index) for r in (interaction, spaced, squared, crossed)], patsy.params


@case
def pooled_by_default():
    data = panel()
    pooled = PanelOLS.from_formula("lwage ~ 1 + exper", data).fit(cov_type="robust")  # xpd: panel-ols-without-effects
    fixed = PanelOLS.from_formula("lwage ~ 1 + exper + EntityEffects", data).fit(cov_type="robust")
    yearly = PanelOLS.from_formula("lwage ~ 1 + hours + TimeEffects", data).fit(cov_type="robust")
    other = PanelOLS.from_formula("lwage ~ 1 + hours", data, other_effects=data.occupation).fit(cov_type="robust")
    said = PooledOLS.from_formula("lwage ~ 1 + exper", data).fit(cov_type="robust")
    return [r.params for r in (pooled, fixed, yearly, other, said)]


def raises_on_7_1():
    data = panel()
    FirstDifferenceOLS.from_formula("lwage ~ 1 + hours", data)  # xpd: first-difference-constant
    FirstDifferenceOLS.from_formula("lwage ~ hours + 1", data)  # xpd: first-difference-constant
    PanelOLS.from_formula("lwage ~ 1 + exper", data, entity_effects=True)  # xpd: effects-keyword-with-formula, panel-ols-without-effects
    PanelOLS.from_formula("lwage ~ 1 + exper + EntityEffects", data, time_effects=True)  # xpd: effects-keyword-with-formula
    PanelOLS.from_formula("lwage ~ 1 + hours + EntityEffects + TimeEffects", data, other_effects=data.occupation)  # xpd: too-many-effects
    PooledOLS.from_formula("lwage ~ 1 + exper", data, weights="hours")  # xpd: weights-column-name
    IV2SLS.from_formula("lwage ~ 1 + exper", women(), weights="hours")  # xpd: weights-column-name
    PooledOLS.from_formula("lwage ~ 1 + exper", data, weights=data.hours)
    try:
        FirstDifferenceOLS.from_formula("lwage ~ 1 + hours", data)
    except ValueError:
        pass
