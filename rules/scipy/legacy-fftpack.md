`scipy.fftpack` is legacy, and SciPy says new code should use `scipy.fft`. Import from `scipy.fft`, and check `rfft`, whose output layout differs.

```diff lang="python"
-from scipy import fftpack
-y = fftpack.dct(x)
+from scipy import fft
+y = fft.dct(x)
```

## What the rule reports

The rule reports an import of `scipy.fftpack`, whose module docstring marks it as legacy and
says new code should use `scipy.fft`. `scipy.fft` was no faster in the measurements, so the rule
claims only the preference.

## How to fix it

xpd has no automatic fix. Because `rfft` packs its output differently and the pseudo differential operators have no `scipy.fft` form. Make the change by hand.
