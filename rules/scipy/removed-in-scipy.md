These names were removed from SciPy and raise AttributeError on SciPy 1.18.1. Use the replacement that the message names.

```diff lang="python"
 from scipy import integrate, signal, stats
-from scipy.misc import comb
-area = integrate.trapz(y, x)
-window = signal.hann(64)
-p = stats.binom_test(k, n)
-angle = scipy.pi * frac
+from scipy.special import comb
+area = integrate.trapezoid(y, x)
+window = signal.windows.hann(64)
+p = stats.binomtest(k, n).pvalue
+angle = np.pi * frac
```

## What the rule reports

The rule reports SciPy names that raise on SciPy 1.18.1. Its message says what to write
instead, or says that SciPy has no public replacement.

| Old names | On SciPy 1.18.1 |
| --- | --- |
| Removed functions for integration, windows, wavelets, Legendre polynomials, spherical Bessel functions, statistics and linear algebra | The name is gone. |
| `interp2d` | A call raises `NotImplementedError`. |
| Everything in `scipy.misc` | The module is empty and warns when imported. |
| NumPy functions and constants on the `scipy` namespace, such as `scipy.pi` | The name is gone. |

The rule leaves alone code under a SciPy version check, a `hasattr` check, or a `try` that
catches `ImportError` or `AttributeError`. Such code exists to support old SciPy.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `trapezoid`, `cumulative_trapezoid` and `simpson` for `trapz`, `cumtrapz` and `simps`, in the module the code already names or in an import as `trapezoid as trapz`, `scipy.signal.windows.<name>` for the 19 windows that `scipy.signal` held, such as `hann`, and `np.<name>` for the 438 names that were the NumPy object on the `scipy` namespace, such as `scipy.pi`. On SciPy 1.13.1, 1.12.0 and 1.3.3 each new form is the same object as the old one or gives a result with an equal pickle. Unsafe, because the results differ on a SciPy that runs both forms: `pinv` for `pinv2`, whose third positional argument was another tolerance, `median_abs_deviation` for `median_absolute_deviation`, which had another default scale, `gibrat` for `gilbrat`, which is another class, `stats.mstats.find_repeats`, which returns a plain tuple, `impulse`, `lsim` and `step` for `impulse2`, `lsim2` and `step2`, `betainc` and `betaincinv` for `btdtr` and `btdtri`, each also in an import as `new as old`, `np.kron`, `np.tri`, `np.tril` and `np.triu` for the `scipy.linalg` names, `np.<name>` for `sqrt`, `log` and the six other `numpy.lib.scimath` functions on the `scipy` namespace, and `scipy.datasets` for `face`, `ascent` and `electrocardiogram` on `scipy.misc`, which raises ImportError without the pooch package. Unsafe, because no SciPy that runs here holds the old form: `eye` for `speye`, `stats.mstats.kendalltau_seasonal`, `expm` for `expm2` and `expm3`, `stats.chi2.sf` for `stats.chisqprob`, `scipy.special.betainc` and `erfc` for `scipy.stats.betai` and `erfc`, `np.linalg.slogdet`, `sparse.csgraph.connected_components` for `sparse.cs_graph_components`, `scipy.special` for `comb`, `factorial` and the other names on `scipy.misc` or the `scipy` namespace, six windows such as `taylor`, and 46 NumPy names such as `np.round`. The other forms have no fix, because the name has no replacement that takes the same arguments, the message names two, or the new module is one that the code does not name, as in `from scipy import misc` or an import from the old module.
