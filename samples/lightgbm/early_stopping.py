from lightgbm import LGBMClassifier, early_stopping
from sklearn.model_selection import train_test_split


def stop_on_test(X, y):
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=7)
    model = LGBMClassifier(random_state=7)
    model.fit(X_train, y_train, eval_X=X_test, eval_y=y_test, callbacks=[early_stopping(5)])  # xpd: early-stop-on-test
    return model.score(X_test, y_test)


def stop_on_development(X, y):
    X_dev, X_test, y_dev, y_test = train_test_split(X, y, random_state=7)
    X_train, X_val, y_train, y_val = train_test_split(X_dev, y_dev, random_state=7)
    model = LGBMClassifier(random_state=7)
    model.fit(X_train, y_train, eval_X=X_val, eval_y=y_val, callbacks=[early_stopping(5)])
    return model.score(X_test, y_test)


def no_stopping(X, y):
    X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=7)
    model = LGBMClassifier(random_state=7)
    model.fit(X_train, y_train, eval_X=X_test, eval_y=y_test)
    return model.score(X_test, y_test)
