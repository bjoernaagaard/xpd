# Testing and proof rules

Read this file when proving a Python change or writing or configuring project tests.

## Proving a change

- Run the real command or application on real data and read its output.
- For a bug, run the same case before and after the fix.
- Re-run proof after the final edit; proof counts only for the code it ran on.
- Write a test file only when the project or user asks for one, and never write a test that replaces the code under test with a mock.

| Test kind | What it proves | Write it for |
| --- | --- | --- |
| End to end | A full user flow works in the running program | Flows a user would notice breaking; end with a checkable, rerunnable artifact |
| Property (Hypothesis) | A statement holds across thousands of generated inputs | Round trips (`decode(encode(x)) == x`), invariants, idempotent actions, fast-vs-slow parity |
| Differential | New code produces the same result as old code | Rewrites or migrations; `xpd equiv old.py new.py` compares scripts assigning `result` |

Watch every new test fail once on broken code.

## Pytest and Hypothesis configuration

- Run pytest from `tests/` beside `src/` with `--import-mode=importlib`, `-ra`, and on pytest 9 `strict = true`.
- Add `pytest-randomly` to catch order-dependent tests and `pytest-timeout` with a configured `timeout` value so hangs fail.
- Never add `pytest-rerunfailures`, `flaky`, `pytest-mock`, or `pytest-socket`, and never use `--reruns` or `xfail(strict=False)`.
- For slow property or differential tests, configure a Hypothesis profile with `deadline=None`. Never suppress `HealthCheck.function_scoped_fixture` or `HealthCheck.differing_executors`.
- Avoid `pytest-cov` and `pytest-benchmark` for end-to-end verification; coverage misses subprocess commands and benchmarks only compare saved runs.
- Investigate intermittent failures at the root cause; never skip, rerun, or add sleeps. If no cause can be found, delete the test and record why.
- Never weaken tests, snapshots, or tool settings to pass your own change, and never add interfaces or hooks purely for test mocking.
- Stop and show the user if a check still fails after three fix rounds.
