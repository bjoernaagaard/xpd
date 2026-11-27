from sklearn.model_selection import train_test_split
from xgboost import XGBClassifier


def stop_on_test(X, y):
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=7)
    model = XGBClassifier(early_stopping_rounds=5, random_state=7)
    model.fit(X_train, y_train, eval_set=[(X_test, y_test)])  # xpd: early-stop-on-test
    return model.score(X_test, y_test)


def stop_on_development(X, y):
    X_dev, X_test, y_dev, y_test = train_test_split(X, y, random_state=7)
    X_train, X_val, y_train, y_val = train_test_split(X_dev, y_dev, random_state=7)
    model = XGBClassifier(early_stopping_rounds=5, random_state=7)
    model.fit(X_train, y_train, eval_set=[(X_val, y_val)])
    return model.score(X_test, y_test)


def no_stopping(X, y):
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=7)
    model = XGBClassifier(random_state=7)
    model.fit(X_train, y_train, eval_set=[(X_test, y_test)])
    return model.score(X_test, y_test)
