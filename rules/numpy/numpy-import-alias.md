NumPy's docs import it as `np`, and other code and the fixes of this pack expect that name. Write `import numpy as np`.

```diff lang="python"
-import numpy as N
+import numpy as np
```

## What the rule reports

The rule reports `import numpy as <name>` with a name other than `np` or `xp`, and `from numpy import *`, which replaces Python's `sum`, `min`, `max`, `round`, `abs`, `all` and `any`.

## How to fix it

xpd has no automatic fix. Because every use of the old name changes with the import. Make the change by hand.
