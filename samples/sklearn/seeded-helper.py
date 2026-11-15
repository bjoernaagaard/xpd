"""A file that calls a seed helper it imports and does not define. Such a helper seeds NumPy, so
xpd reports nothing here."""

from sklearn.datasets import make_classification
from sklearn.ensemble import RandomForestClassifier
from sklearn.model_selection import train_test_split


def fit_after_seed_helper():
    from experiment.utils import set_seed

    set_seed(0)
    X, y = make_classification(random_state=0)
    X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)
    return RandomForestClassifier(n_estimators=10).fit(X_train, y_train).score(X_test, y_test)
