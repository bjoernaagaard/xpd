# Python versions and tool coverage

Read this file when selecting syntax for a specific `requires-python` version or checking which tool enforces a rule from `python-best-practices`.

## Python 3.10 to 3.15 feature table

| Since | Use | Instead of |
| --- | --- | --- |
| 3.10 | `X \| None`, `match`, `zip(strict=True)`, `kw_only=True` and `slots=True` on dataclasses | `Optional[X]`, `Union`, long `isinstance` chains |
| 3.11 | `Self`, `tomllib`, `StrEnum`, `asyncio.TaskGroup`, `except*`, `err.add_note()` | class-bound `TypeVar`, `tomli`, `asyncio.gather` |
| 3.12 | `def first[T](xs: list[T]) -> T`, `type Pair = tuple[int, int]`, `@override`, `itertools.batched`, `Path.walk` | `TypeVar`, `TypeAlias`, `os.walk` |
| 3.13 | type parameter defaults, `@warnings.deprecated`, `copy.replace`, `ReadOnly`, `TypeIs` | `dataclasses.replace` for non-dataclasses, `TypeGuard` for narrowing |
| 3.14 | annotations evaluated on demand, template strings (`t"..."`), `compression.zstd`, `concurrent.interpreters` | quoted forward references, `from __future__ import annotations` |
| 3.15 | `lazy import`, `frozendict`, `sentinel`, `{**d for d in dicts}`, `[*xs for xs in lists]` | in-function imports for startup speed, `object()` sentinels, dict-merge loops |

## What each tool checks

| Rule area | Tool |
| --- | --- |
| Formatting, quotes, import order, unused names, mutable defaults (`B006`), bare `except`, named `lambda` (`E731`), in-function imports (`PLC0415`), private member access (`SLF001`) | `ruff format`, `ruff check` |
| Types, `None` handling, missing annotations | Project type checker (`ty`, `pyright`, `mypy`) |
| Underscore module names, `utils` or `config` modules, bare verb names, f-string keys, value-prefixed errors, runtime deprecation warnings | `xpd check` (`python/module-private-name`, `vague-module-name`, `generic-verb-name`, `interpolated-key`, `error-message-prefix`, `deprecated-without-decorator`) and `xpd discover` |
| Import-time work, complex or nested comprehensions, pass-through wrappers, multi-line call arguments | `xpd check` (`python/import-side-effect`, `comprehension-*`, `unpacked-comprehension`, `pass-through-function`, `multi-line-call-argument`) |
| `typing.cast`, `dict[str, Any]`, swapped plain IDs, frozen dataclasses over `NamedTuple`, untyped `json.loads`, redundant type conversions | `xpd check` (`python/typing-cast`, `dict-str-any`, `id-without-newtype`, `record-without-namedtuple`, `json-without-type`, `same-type-conversion`, `float-around-number`) |
| Stdlib replacements (`defaultdict`, `Counter`, `fromisoformat`, `perf_counter`, `tomllib`, `append`/`deque`, `orjson.loads(bytes)`, `__slots__`) | `xpd check` (`python/setdefault-without-defaultdict`, `counting-loop-without-counter`, `strptime-without-fromisoformat`, `elapsed-without-perf-counter`, `toml-without-tomllib`, `front-insert-in-loop`, `orjson-loads-decoded`, `slotted-base-without-slots`) |
| Vectorized pandas and NumPy loops, empty guards, `groupby` loops, mock imports, disabled or flaky tests | `xpd check` (`pandas/*`, `numpy/*`, `python/mock-import`, `python/test-*`) |
| Full multi-library static pack coverage and workflow rollups | `xpd check` and `xpd audit` |
