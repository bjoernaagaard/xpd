# Python review checklist

Read this checklist during step 3 of `python-review` to map observed issues to exact `xpd` and style rules.

## Correctness

- Mutable default argument or shared mutable class attribute (`B006`).
- Work at import time: I/O, environment reads, library configuration, or thread startup (`python/import-side-effect`).
- Loop closure capturing the loop variable or a generator consumed twice.
- Resource not closed on every path: file, socket, lock, process, or temp directory opened without `with`.
- Naive vs timezone-aware `datetime` mismatch or local time treated as UTC.
- `==` on floats, `is` on strings or numbers, or truthiness checks where `0` or `""` are valid values.
- Mutating a collection while iterating or order depending on a `set`.
- Unsafe input: `pickle`, `eval`, `yaml.load`, `shell=True` on untrusted data, unchecked user paths, or `random` for tokens.
- Async code calling blocking I/O, dropping task references, or swallowing `CancelledError`.

## Errors

- Bare `except`, `except Exception` hiding the cause, or `except ...: pass`.
- Error both logged and re-raised, or re-raised without `from err`.
- `None`, `-1`, or `""` returned to signal failure without caller handling.
- `assert` guarding runtime input or state.
- Error message omitting the offending value.

## Types and boundaries

- Public function without annotations or taking or returning `Any`.
- `typing.cast` (`python/typing-cast`) or `# type: ignore` instead of fixing the source type.
- `dict[str, Any]` across functions (`python/dict-str-any`), raw `json.loads` return (`python/json-without-type`), or swappable plain `str`/`int` IDs (`python/id-without-newtype`).
- Frozen dataclass replaceable by `NamedTuple` (`python/record-without-namedtuple`), single-caller local record, or single-field wrapper.
- Redundant checks or casts on typed values: `float()`, `int()`, `str()`, `bool()` (`python/same-type-conversion`, `python/float-around-number`), redundant `isinstance`, or empty-frame guards around pandas reductions (`pandas/empty-guard`).
- `Protocol` or `ABC` with one implementation, or an interface added solely for tests.
- Repeated bare strings where `StrEnum` or `Literal` fits.
- Missed stdlib idioms: `defaultdict(list)` (`python/setdefault-without-defaultdict`), `Counter` (`python/counting-loop-without-counter`), `date.fromisoformat` (`python/strptime-without-fromisoformat`), `time.perf_counter()` (`python/elapsed-without-perf-counter`), `tomllib` (`python/toml-without-tomllib`), `append`/`deque` (`python/front-insert-in-loop`), `orjson.loads(raw)` (`python/orjson-loads-decoded`), or missing `__slots__` on slotted subclass (`python/slotted-base-without-slots`).

## Module design

- Leading-underscore module-level names or missing `__all__` (`python/module-private-name`).
- Mutable module state, `utils`/`helpers`/`config` modules (`python/vague-module-name`), or wildcard imports.
- Error message starting with a runtime value (`python/error-message-prefix`) or runtime warning instead of `@deprecated` (`python/deprecated-without-decorator`).
- Bare-verb public names (`python/generic-verb-name`), duplicate public names across modules, or f-string keys/paths (`python/interpolated-key`) — check together with `xpd discover`.
- Unjustified import inside a function, single-method `__init__` class, namespace class, or third-party helper duplicating stdlib or existing dependencies.

## Readability and library habits

- Comprehensions with two `for` clauses (`python/comprehension-two-fors`), dict merges (`python/comprehension-merges-dicts`, `python/unpacked-comprehension`), multi-line bodies (`python/comprehension-multi-line`), or mixed call arguments (`python/comprehension-among-arguments`).
- Pass-through wrappers (`python/pass-through-function`), nested multi-line call arguments (`python/multi-line-call-argument`), or frames rebuilt from columns (`pandas/frame-rebuilt-from-columns`).
- Excessive pandas/NumPy round trips (`pandas/repeated-reset-index`, `pandas/numpy-round-trip`, `pandas/repeat-rows-by-index`) or `groupby` collection loops (`pandas/groupby-loop-collects`).
- Python loops replaceable by whole-array or column calls (`numpy/recurrence-loop`, `numpy/mask-per-label`, `numpy/random-draw-loop`, `pandas/column-loop`, `pandas/named-agg-lambda`, `pandas/filter-before-frame`).
- Multi-level functions, positional bools, deep nesting, named `lambda`, comments restating code, or outdated syntax for `requires-python`.
- Library-specific findings from `xpd check`, `xpd profile`, and `xpd equiv`.

## Proof

- Change lacking a runnable verification command.
- Mocked tests (`python/mock-import`) or private-detail assertions.
- Skipped or `xfail(strict=False)` tests (`python/test-skip`), retries (`python/test-rerun`), sleeps (`python/test-sleep`), or suppressed Hypothesis health checks (`python/test-suppressed-health-check`).
- Diff weakening a test, snapshot, or tool setting to make itself pass.
- Round trips, invariants, or rewrites in tested projects missing Hypothesis property tests or `xpd equiv`.
