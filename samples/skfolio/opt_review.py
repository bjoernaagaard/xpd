from sklearn.model_selection import train_test_split
from skfolio.optimization import MeanRisk


def leaky(returns):
    model = MeanRisk().fit(returns)
    train, test = train_test_split(returns, shuffle=False)
    return model.predict(test)


def fit_on_training(returns):
    train, test = train_test_split(returns, shuffle=False)
    model = MeanRisk().fit(train)
    return model.predict(test)


def deploy(returns):
    model = MeanRisk()
    model.fit(returns)
    return model
