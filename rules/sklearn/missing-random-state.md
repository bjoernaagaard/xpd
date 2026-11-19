Without `random_state`, a split or a model takes a new seed on every run, so the score changes from run to run. Pass `random_state`.

```diff lang="python"
-X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)
-model = RandomForestClassifier()
+X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2, random_state=0)
+model = RandomForestClassifier(random_state=0)
```

## What the rule reports

The rule reports `train_test_split(X, y)`, `RandomForestClassifier()` and `KFold(shuffle=True)`
without `random_state`.

The rule leaves alone a file that calls `np.random.seed`, or a seed helper such as `set_seed` or
`seed_everything` that the file doesn't define and doesn't import from TensorFlow, PyTorch or
`random`. It also leaves alone a call with `shuffle=False` or `**`, a `train_test_split` whose
result is thrown away, and a constructor that only `isinstance` reads. Of 527 held-back reports,
52 were labelled, and each reader called 1 wrong. Recall was 79 of 80.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Any fixed number makes the run repeatable, and 0 is only the example.

The rule is info, because a seed doesn't move the expected score. A seed can still change which
model wins a comparison, as the worse of two models whose true accuracy differs a little won more
often without a seed:

| Split | Worse model won, of 300 trials |
| --- | --- |
| each model on its own split, without a seed | 25.0 percent |
| both models on one shared split | 14.3 percent |

The classes that the rule names gave another output in five fresh runs on scikit-learn 1.9.1:

| Kind | Classes |
| --- | --- |
| ensembles | the forests, extra trees, bagging, gradient boosting, `AdaBoostRegressor`, `IsolationForest`, `RandomTreesEmbedding` |
| trees and other models | the decision trees, the MLPs, the SGD models |
| clustering | `KMeans`, `MiniBatchKMeans`, `SpectralClustering`, `AffinityPropagation` |
| decomposition and embedding | `TruncatedSVD`, `NMF`, `LatentDirichletAllocation`, `FastICA`, `MiniBatchDictionaryLearning`, `MDS`, `SpectralEmbedding`, `Nystroem`, `RBFSampler`, the random projections |
| splitters | `ShuffleSplit`, `StratifiedShuffleSplit`, `RepeatedKFold`, `RepeatedStratifiedKFold`, and `KFold`, `StratifiedKFold`, `GroupKFold` and `StratifiedGroupKFold` with `shuffle=True` |
| classes with a setting | `TSNE` with `init="random"`, `SVC(probability=True)`, a HistGradientBoosting model with `early_stopping=True`, a Gaussian mixture with more than one component |

A HistGradientBoosting model without `random_state` is reported with a message that names 10,000
rows, because above that size it stops early on a validation part drawn at random.
`LogisticRegression`, `PCA`, `Perceptron`, `AdaBoostClassifier` and `TSNE` with its default `init`
gave the same output and are not listed.
