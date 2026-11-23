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

`xpd check --fix --unsafe-fixes` rewrites `from scipy.fftpack import ...` to `from scipy.fft import ...` when every imported name has a compatible `scipy.fft` equivalent (excluding `rfft`, `irfft`, `rfftfreq`, and pseudo-differential operators). Other imports remain manual.
