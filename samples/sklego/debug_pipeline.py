"""A DebugPipeline with the default log callback in a file that never sets up logging.

A line that xpd reports ends with a comment that names the rule. samples/sklego/fit.py holds the
same pipelines after logging.basicConfig, which xpd leaves alone.
"""

from sklearn.datasets import make_regression
from sklearn.linear_model import LinearRegression
from sklearn.preprocessing import StandardScaler
from sklego.pipeline import DebugPipeline, make_debug_pipeline

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def silent_logs():
    X, y = make_regression(n_samples=50, n_features=3, random_state=0)
    logged = DebugPipeline([("scale", StandardScaler()), ("model", LinearRegression())], log_callback="default")  # xpd: debug-pipeline-default-log
    made = make_debug_pipeline(StandardScaler(), LinearRegression(), log_callback="default")  # xpd: debug-pipeline-default-log
    shown = []
    printed = DebugPipeline([("scale", StandardScaler()), ("model", LinearRegression())], log_callback=lambda output, execution_time, **kwargs: shown.append(output[1]))
    quiet = DebugPipeline([("scale", StandardScaler()), ("model", LinearRegression())])
    return logged.fit(X, y).predict(X), made.fit(X, y).predict(X), printed.fit(X, y).predict(X), quiet.fit(X, y).predict(X), len(shown)
