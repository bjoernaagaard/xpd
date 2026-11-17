"""A file that seeds NumPy. scikit-learn then draws from that seed, so xpd reports no missing
random_state here. The NumPy rule legacy-random reports the global seed and the draw from it."""

import numpy as np
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import train_test_split

np.random.seed(0)  # xpd: legacy-random
X = np.random.normal(size=(100, 3))  # xpd: legacy-random
y = (X[:, 0] > 0).astype(int)
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)
model = RandomForestClassifier(n_estimators=10).fit(X_train, y_train)
