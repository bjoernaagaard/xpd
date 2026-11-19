A file that draws with `df.plot` and sets no backend gets matplotlib charts that xpd's plotly rules cannot check. Set `pd.options.plotting.backend = "plotly"` once after the import.

```diff lang="python"
 import pandas as pd
 
+pd.options.plotting.backend = "plotly"
+
 df.plot.bar(x="team", y="score")
```

## What the rule reports

The rule reports a file that draws with a plot method of the accessor and sets no plotting backend. The report is on the first import of pandas, once per file. xpd reports only a file where the setting changes no other line:

- The file calls `line`, `bar`, `barh`, `area`, `scatter`, `box` or `hist` on the accessor. `kde`, `density`, `pie` and `hexbin` raised `NotImplementedError` with plotly, so a file that names one is left alone.
- Every argument of those calls is `x`, `y` or `title`. A file with another keyword, such as `figsize` or `ax`, is left alone, and so is a file that calls `df.plot(...)` itself.
- Each call stands alone as a statement or is returned. A file that assigns the result is left alone, because the result is a matplotlib Axes there.
- The file imports neither matplotlib nor seaborn.

The rule also reports nothing when one of the following modules sets the backend:

- the `__init__.py` of a package that the file is in
- a module of the project that the file imports
- a module that one of those modules imports

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds the line `pd.options.plotting.backend = "plotly"` after `import pandas as pd`. df.plot then returns a plotly Figure in place of a matplotlib Axes. Any other first import of pandas has no fix.
