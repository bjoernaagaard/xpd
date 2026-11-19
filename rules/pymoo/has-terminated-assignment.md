Since pymoo 0.6.0, `has_terminated` is not read, so setting it does not restart a loaded algorithm that had finished. Give the algorithm a new termination instead.

```diff lang="python"
 (checkpoint,) = np.load("checkpoint.npy", allow_pickle=True).flatten()
-checkpoint.has_terminated = False
-res = minimize(problem, checkpoint, ("n_gen", 20), seed=1, copy_algorithm=False)
+checkpoint.termination = MaximumGenerationTermination(20)
+res = minimize(problem, checkpoint, copy_algorithm=False)
```

## What the rule reports

The rule reports `has_terminated = False` on a name that the file assigns from `np.load`, `pickle.load` or `dill.load`, also as one name of a tuple. Another object with an attribute of the same name is left alone. The checkpoint recipe of pymoo 0.4 and 0.5 set the attribute to let a loaded algorithm run on, and since 0.6.0 pymoo no longer reads it. An `NSGA2` that had finished its 5 generations ran 0 more generations after `has_terminated = False`, and 15 more after it got a new termination.

## How to fix it

xpd has no automatic fix, so make the change by hand.
