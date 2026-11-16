"""Covariance choices in fit that give other standard errors than the code says, or that raise.

A line that xpd reports ends with a comment that names the rule. A form that raises on
linearmodels 7.1 sits in a function that is not a case and that nothing calls. Each case returns
what the code computes.
"""

import warnings

import linearmodels
import numpy as np
import statsmodels.api as sm
from linearmodels import IV2SLS, IVGMM, IVGMMCUE, SUR, AbsorbingLS, BetweenOLS, FamaMacBeth, IVSystemGMM, PanelOLS, PooledOLS, RandomEffects, TradedFactorModel
from linearmodels.datasets import french, mroz, wage_panel

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def panel():
    data = wage_panel.load().set_index(["nr", "year"])
    return data.lwage, sm.add_constant(data[["exper", "hours"]])


def women():
    data = mroz.load().dropna().reset_index(drop=True)
    return data.assign(const=1.0)


def four(data):
    return data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc", "motheduc"]]


@case
def clustered_without_clusters():
    y, X = panel()
    model = PanelOLS(y, X, entity_effects=True)
    robust_in_fact = model.fit(cov_type="clustered")  # xpd: clustered-without-clusters
    by_entity = model.fit(cov_type="clustered", cluster_entity=True)
    by_time = model.fit(cov_type="clustered", cluster_time=True)
    chained = PooledOLS(y, X).fit(cov_type="clustered")  # xpd: clustered-without-clusters
    random = RandomEffects(y, X).fit(cov_type="clustered", debiased=True)  # xpd: clustered-without-clusters
    options = {"cov_type": "clustered", "cluster_entity": True}
    unpacked = model.fit(**options)
    name = "clustered"
    from_a_name = model.fit(cov_type=name)
    between = BetweenOLS(y, X).fit(cov_type="clustered")
    data = women()
    two_stage = IV2SLS(*four(data)).fit(cov_type="clustered")  # xpd: clustered-without-clusters
    grouped = IV2SLS(*four(data)).fit(cov_type="clustered", clusters=data.kidslt6)
    absorbed = AbsorbingLS(data.lwage, data[["const", "exper"]], absorb=data[["kidslt6"]].astype("category")).fit(cov_type="clustered")  # xpd: clustered-without-clusters
    # A statsmodels result takes its own names and is left alone.
    ols = sm.OLS(data.lwage, data[["const", "exper"]]).fit(cov_type="cluster", cov_kwds={"groups": data.kidslt6})
    results = (robust_in_fact, by_entity, by_time, chained, random, unpacked, from_a_name, between, two_stage, grouped, absorbed)
    return [r.std_errors for r in results], ols.bse


@case
def default_covariance():
    y, X = panel()
    default = PanelOLS(y, X, entity_effects=True).fit()  # xpd: panel-default-covariance
    model = RandomEffects(y, X)
    also_default = model.fit(debiased=True)  # xpd: panel-default-covariance
    stated = PanelOLS(y, X, entity_effects=True).fit(cov_type="unadjusted")
    clustered = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
    between = BetweenOLS(y, X).fit()
    fama = FamaMacBeth(y, X).fit()
    data = women()
    cross_section = IV2SLS(*four(data)).fit()
    return [r.std_errors for r in (default, also_default, stated, clustered, between, fama, cross_section)]


@case
def ignored_kernel_options():
    y, X = panel()
    model = FamaMacBeth(y, X)
    ignored = model.fit(bandwidth=3)  # xpd: cov-option-without-cov-type
    also_ignored = model.fit(cov_type="robust", kernel="parzen")  # xpd: cov-option-without-cov-type
    read = model.fit(cov_type="kernel", bandwidth=3)
    by_position = model.fit("kernel", True, 3)
    return [r.std_errors for r in (ignored, also_ignored, read, by_position)]


@case
def default_bandwidth():
    data = women()
    y, exog, endog, instruments = four(data)
    equations = {"wage": {"dependent": y, "exog": exog, "endog": endog, "instruments": instruments}, "hours": {"dependent": data.hours, "exog": data[["const", "age"]]}}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        changed = IVGMM(y, exog, endog, instruments, weight_type="kernel").fit()  # xpd: kernel-default-bandwidth
        changed_cue = IVGMMCUE(y, exog, endog, instruments, weight_type="kernel").fit()  # xpd: kernel-default-bandwidth
        changed_system = IVSystemGMM(equations, weight_type="kernel").fit()  # xpd: kernel-default-bandwidth
        changed_cov = IVGMM(y, exog, endog, instruments).fit(cov_type="kernel")  # xpd: kernel-default-bandwidth
        from_formula = IVGMM.from_formula("lwage ~ 1 + exper + [educ ~ fatheduc + motheduc]", data, weight_type="kernel").fit()  # xpd: kernel-default-bandwidth
    selected = IVGMM(y, exog, endog, instruments, weight_type="kernel", optimal_bw=True).fit()
    fixed = IVGMM(y, exog, endog, instruments, weight_type="kernel", bandwidth=4).fit()
    fixed_cov = IVGMM(y, exog, endog, instruments).fit(cov_type="kernel", bandwidth=4)
    robust = IVGMM(y, exog, endog, instruments, weight_type="robust").fit()
    two_stage = IV2SLS(y, exog, endog, instruments).fit(cov_type="kernel")
    system_cov = IVSystemGMM(equations).fit(cov_type="kernel")
    results = (changed, changed_cue, changed_system, changed_cov, from_formula, selected, fixed, fixed_cov, robust, two_stage, system_cov)
    return [np.asarray(r.std_errors) for r in results]


def raises_on_7_1():
    y, X = panel()
    data = women()
    model = PanelOLS(y, X, entity_effects=True)
    model.fit(cluster_entity=True)  # xpd: cov-option-without-cov-type, panel-default-covariance
    model.fit(cov_type="robust", cluster_entity=True)  # xpd: cov-option-without-cov-type
    model.fit(cov_type="unadjusted", clusters=data.kidslt6)  # xpd: cov-option-without-cov-type
    model.fit(bandwidth=3)  # xpd: cov-option-without-cov-type, panel-default-covariance
    model.fit(cov_type="clustered", kernel="parzen")  # xpd: clustered-without-clusters, cov-option-without-cov-type
    model.fit(cov_type="kernel", bandwidth=3)
    IV2SLS(*four(data)).fit(clusters=data.kidslt6)  # xpd: cov-option-without-cov-type
    IV2SLS(*four(data)).fit(cov_type="kernel", kernel="parzen")
    linearmodels.IVGMM(*four(data)).fit(cov_type="clustered")  # xpd: clustered-without-clusters
    IVGMM(*four(data)).fit(cov_type="clustered", clusters=data.kidslt6)
    factors = french.load()
    traded = TradedFactorModel(factors[["S1V1", "S1V3"]], factors[["MktRF", "SMB"]])
    traded.fit(bandwidth=3)  # xpd: cov-option-without-cov-type
    traded.fit(cov_type="kernel", bandwidth=3)

    model.fit(cov_type="HC1")  # xpd: unknown-cov-type
    model.fit(cov_type="cluster", cluster_entity=True)  # xpd: unknown-cov-type
    model.fit(cov_type="hac")  # xpd: unknown-cov-type
    model.fit(cov_type="Robust")
    model.fit(cov_type="driscoll-kraay")
    BetweenOLS(y, X).fit(cov_type="kernel")  # xpd: unknown-cov-type
    FamaMacBeth(y, X).fit(cov_type="clustered")  # xpd: unknown-cov-type
    IV2SLS(*four(data)).fit(cov_type="HAC")  # xpd: unknown-cov-type
    IV2SLS(*four(data)).fit(cov_type="nonrobust")  # xpd: unknown-cov-type
    IV2SLS(*four(data)).fit(cov_type="homoskedastic")
    SUR.multivariate_ls(data[["lwage", "hours"]], data[["const", "exper"]]).fit(cov_type="hac")
    SUR.multivariate_ls(data[["lwage", "hours"]], data[["const", "exper"]]).fit(cov_type="HC0")  # xpd: unknown-cov-type
    traded.fit(cov_type="unadjusted")  # xpd: unknown-cov-type
    traded.fit(cov_type="robust")
    sm.OLS(data.lwage, data[["const", "exper"]]).fit(cov_type="HC1")
    name = "HC1"
    model.fit(cov_type=name)
    # A name that is also assigned from statsmodels is not known to be a linearmodels model.
    either = PanelOLS(y, X, entity_effects=True)
    either = sm.OLS(y, X)
    either.fit(cov_type="HC1")
    try:
        model.fit(cov_type="HC1")
    except ValueError:
        pass
