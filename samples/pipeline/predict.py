"""Loads the saved penguin model and predicts the held out rows, as a service would.

The contract that train.py wrote says all it needs: where the data and the model are, the
input columns with their dtypes, the versions, the split, and the code files that the model
imports when it loads. The saved Pipeline holds add_features by its module and name, so
features.py has to be beside this script. It leaves the class probabilities in `result`, so
`xpd equiv train.py predict.py` compares them with those of the fitted model.
"""

import json
from pathlib import Path

import joblib
import pandas as pd
import sklearn
from sklearn.model_selection import train_test_split

CONTRACT = Path("model/contract.json")

contract = json.loads(CONTRACT.read_text())
trained_with = contract["model"]["versions"]["scikit-learn"]
if trained_with != sklearn.__version__:
    msg = f"The model was trained with scikit-learn {trained_with}, not {sklearn.__version__}."
    raise RuntimeError(msg)

missing = [file for file in contract["model"]["code"] if not Path(file).is_file()]
if missing:
    msg = f"The model imports {missing}, and no such file is here."
    raise FileNotFoundError(msg)

penguins = pd.read_csv(contract["data"]["file"])
expected = {column["column"]: column["dtype"] for column in contract["inputs"]}
X = penguins[list(expected)]
found = {name: str(dtype) for name, dtype in X.dtypes.items()}
if found != expected:
    msg = f"The input columns are {found}, and the contract says {expected}."
    raise TypeError(msg)

split = contract["split"]
target = penguins[contract["data"]["target"]]
X_train, X_test = train_test_split(
    X,
    test_size=split["test_size"],
    random_state=split["seed"],
    stratify=target if split["stratified"] else None,
)

model = joblib.load(contract["model"]["file"])
result = pd.DataFrame(model.predict_proba(X_test), index=X_test.index, columns=model.classes_)
