SciPy marks `signal.stft`, `spectrogram` and similar functions as legacy. Use a `ShortTimeFFT` object and its method of the same purpose.

```diff lang="python"
-w, h = signal.sosfreqz(sos, worN=512)
+w, h = signal.freqz_sos(sos, worN=512)
```

## What the rule reports

The rule reports `signal.stft`, `istft`, `spectrogram` and `check_COLA`, which SciPy marks as
legacy in favor of the `ShortTimeFFT` class. It also reports `signal.sosfreqz`, a legacy alias
of `freqz_sos`, but not on SciPy older than 1.15.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `freqz_sos` for `sosfreqz`, whose body only calls freqz_sos. The other forms have no fix.
