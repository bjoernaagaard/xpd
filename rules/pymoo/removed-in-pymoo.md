pymoo removed this module, name or option between 0.5.0 and 0.6.2, so the code raises. Use the replacement that the message names.

```diff lang="python"
-from pymoo.factory import get_problem, get_sampling, get_crossover, get_mutation
-from pymoo.algorithms.nsga2 import NSGA2
-from pymoo.performance_indicator.hv import Hypervolume
+from pymoo.problems import get_problem
+from pymoo.algorithms.moo.nsga2 import NSGA2
+from pymoo.indicators.hv import HV
+from pymoo.operators.crossover.sbx import SBX
+from pymoo.operators.mutation.pm import PM
+from pymoo.operators.sampling.rnd import FloatRandomSampling
+from pymoo.termination.default import DefaultMultiObjectiveTermination
 
 algorithm = NSGA2(
     pop_size=100,
-    sampling=get_sampling("real_random"),
-    crossover=get_crossover("real_sbx", prob=0.9, eta=15),
-    mutation=get_mutation("real_pm", eta=20),
+    sampling=FloatRandomSampling(),
+    crossover=SBX(prob=0.9, eta=15),
+    mutation=PM(eta=20),
 )
-res = minimize(get_problem("zdt1"), algorithm, ("f_tol", 0.0025), seed=1)
-hv = Hypervolume(ref_point=np.array([1.1, 1.1])).calc(res.F)
+res = minimize(get_problem("zdt1"), algorithm, DefaultMultiObjectiveTermination(ftol=0.0025), seed=1)
+hv = HV(ref_point=np.array([1.1, 1.1]))(res.F)
```

## What the rule reports

The rule reports a module, a name or an option that pymoo removed between 0.5.0 and 0.6.2, and the
message names what took its place where there is one.

| Form | Examples |
| --- | --- |
| an import of one of the 130 removed modules | `pymoo.model`, the algorithm modules of 0.4 such as `pymoo.algorithms.nsga2` and `pymoo.algorithms.so_genetic_algorithm`, the long operator modules such as `pymoo.operators.crossover.simulated_binary_crossover`, `pymoo.performance_indicator`, `pymoo.decision_making`, `pymoo.util.termination`, `pymoo.operators.mixed_variable_operator`, `pymoo.interface`, `pymoo.util.function_loader` and `pymoo.factory` |
| a function of `pymoo.factory` that built an object from a name | `get_sampling("real_random")`, and the message names the class to use |
| one of 49 names that a module of 0.6.2 no longer has | `Display` in `pymoo.util.display`, `starmap_parallelized_eval` in `pymoo.core.problem` |
| one of the ten termination names that 0.6.0 dropped, given to `get_termination` or in a tuple given to `minimize` | `"f_tol"`, `"x_tol"` |
| `calc` on an indicator | `Hypervolume(...).calc(F)` |

An import inside a `try` block that catches the error, or behind a version check, is left alone.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

With the unsafe fix, the new module for 89 removed modules when every imported name is still in it, such as `pymoo.core.problem` for `pymoo.model.problem` and `pymoo.algorithms.moo.nsga2` for `pymoo.algorithms.nsga2`, the new import for `get_problem`, `get_termination` or `get_reference_directions` imported alone from `pymoo.factory`, `do` for `calc` on an indicator, and the new module for a name that moved, such as `pymoo.parallelization.dask` for `DaskParallelization` from `pymoo.core.problem`, when the statement is on one line and imports only names that moved there. `DES` gets the new module from pymoo 0.6.2 on, the first release that has it. No release from 0.4.2.2 to 0.6.2 had an old and a new form as one object. The new modules of 0.5.0 and the new homes of the moved names came after the old ones went, the `pymoo.factory` functions of 0.6.0 are wrappers that warn and call the new ones, the sorting functions of 0.6.1.6 are other objects than their new copies, and no release has both `calc` and `do` on an indicator. The other forms have no fix.
