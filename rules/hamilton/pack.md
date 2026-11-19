# Hamilton

The Hamilton rule finds a node that changes an input that other nodes share. It runs on files
that import `hamilton`, the module of [Apache Hamilton](https://hamilton.apache.org/), which PyPI
ships as `apache-hamilton` and `sf-hamilton`. Hamilton builds a graph of steps from plain
functions, so the name of a function is its output and the names of its parameters are its inputs. The results
below are from sf-hamilton 1.90.0 with pandas 3.1.1.

```python
def raw() -> pd.DataFrame: ...                      # node raw
def clipped(raw: pd.DataFrame) -> pd.DataFrame: ... # node clipped, with the input raw
def total_uk(raw: pd.DataFrame) -> float: ...       # node total_uk, with the same raw
```

xpd does not check a node module that uses no Hamilton decorator, because it has no `hamilton`
import.

## Shared inputs

Copy an input before a node changes it, because Hamilton hands the same object to every node that takes the input.

- node-mutates-input
