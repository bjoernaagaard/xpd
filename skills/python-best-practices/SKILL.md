---
name: python-best-practices
description: >-
  Write clear, typed, maintainable Python. Use whenever you write, change, refactor, debug,
  or explain Python code: modules, functions, classes, type hints, dataclasses, errors,
  comprehensions, imports, pyproject.toml, or scripts. Trigger on Python, .py files, uv,
  ruff, type checkers, pandas, or notebooks turned into modules, even without the words
  "best practices".
---

# Python best practices

Follow PEP 8, the Google Python Style Guide, and the typing documentation. This file holds the rules where they leave a choice open, where models drift, and what changed through Python 3.15. Where a project already has a convention, the project wins.

## 1. Before you hand back code

From the project root, run what the project has configured:

```bash
ruff format .           # or the project's formatter
ruff check .
ty check                # or the project's type checker: pyright, mypy
xpd check .             # when xpd is installed
xpd audit . --since HEAD  # optional read-only multi-workflow check
```

- Fix a finding rather than silencing it. To suppress one line, name the rule and reason: `# noqa: S603  fixed argument list`.
- Run the real command or app on real input and read its output. A clean lint run shows the code is tidy, not that it works.

## 2. Versions and dependencies

Read `requires-python` in `pyproject.toml` before using modern syntax. A new project declares the current release (`>=3.15`); a library declares the oldest release it tests. Check dependencies and the type checker separately before adopting new syntax. See [references/versions-and-tools.md](references/versions-and-tools.md) for the Python 3.10 to 3.15 feature table and tool-to-rule mapping.

## 3. Names and modules

- **Module-level names are public.** Do not prefix module-level functions, classes, or variables with `_` unless a framework requires it (such as Hamilton `_helper`). Export public names in `__all__` at the top of the module.
- **Class internals and locals may use `_`.** `def _load(self)` and local names inside functions are fine.
- **Importing a module does nothing.** Keep I/O, environment reads, library setup, and logging configuration out of module scope; run them inside `main()` or caller-invoked functions. No mutable module-level state; attach caches to objects or `@functools.cache`.
- **Name modules after what they hold.** Avoid `utils`, `helpers`, `common`, or `misc`.
- **A public name is a search query.** Use two to four words including a domain term (`load_orders`, not `load`). Keep one definition per name, one spelling per concept, and rename functions in the commit that changes what they do.
- **Write each name out whole.** Avoid building event names, settings keys, or module paths with f-strings, `getattr`, or `import_module`; use an explicit dict mapping.
- **Imports sit at the top**, absolute, one module per line, no wildcards. Justify any in-function import with a comment (or `lazy import` on Python 3.15+).
- **Casing:** `UPPER_CASE` constants, `CapWords` classes, `lower_case` everything else, no type suffixes (`user_list`), and no single-letter names outside tiny loops or comprehensions.

## 4. Functions and control flow

- **Return early** with no `else` after `return` or `raise`.
- **Booleans and options are keyword-only:** `def load(path, *, strict=False)`. Never use mutable defaults (`None` or tuple instead).
- **Keep comprehensions simple:** one `for`, at most one `if`, one line. Name a comprehension before passing it alongside other arguments (`frames = [...]`; `pd.concat(frames, ...)`). Build dicts or merged rows in an explicit loop instead of `{**a, **b}` inside comprehensions.
- **Build lists with `append`.** Use `reverse()` once after the loop or `collections.deque.appendleft` instead of `insert(0, item)`.
- **Use generator expressions** directly in consuming calls (`sum`, `any`, `"".join`), and generator functions for large or early-stopping streams.
- **One multi-line call per statement.** Name inner multi-line expressions on the line above instead of nesting them as call arguments.
- **No `lambda` bound to a name** (write `def`), and **no pass-through wrappers** that only forward arguments to one call. Split functions that work at multiple levels.

## 5. Types and records

- **Annotate every parameter and return value.** Inside functions, annotate only what the checker cannot infer (such as empty collections).
- **Data crossing a boundary gets a named model.** Parse files, requests, rows, or configs once into a `NamedTuple` (immutable default, copied with `_replace`), a `dataclass` (when mutable, inheriting, or needing `__post_init__`), or a `TypedDict` (when it must stay a dict). Never pass `dict[str, Any]` or raw `json.loads` across functions.
- **Trust typed values.** Skip redundant `isinstance`, `float()`, `int()`, `str()`, or `bool()` conversions on already-typed values. NumPy `np.float64` subclasses `float`, and pandas reductions (`mean`, `median`, `min`, `max`) return `nan` on empty Series without an empty guard. Use `to_numpy(dtype=float)` instead of `astype(float).to_numpy()`.
- **No `typing.cast` or `Any`** except at one narrow untyped library boundary returning a typed value. Every `# type: ignore` names its error code and reason.
- **Concrete types until a second implementation exists.** No `Protocol` or `ABC` for one implementation. Accept abstract collection inputs (`Iterable[str]`, `Mapping[str, int]`) and return concrete types (`list[str]`).
- **Prefer the standard library:** `tomllib`, `functools.cache`, `itertools.pairwise` and `batched`, `collections.defaultdict(list)`, `collections.Counter`, `date.fromisoformat`, `time.perf_counter()`, `StrEnum` or `Literal` for closed sets. Subclasses of slotted classes declare `__slots__`.
- **Keep records purposeful:** return a tuple for two or three local values, use `groupby().agg(...)` for group summaries, drop single-field wrappers, and turn `__init__` plus one-method classes into functions.

## 6. Errors

- **Check with `if` and `raise`** the most specific built-in or project exception with a message naming what failed and the offending value.
- **Catch narrowly.** No bare `except`; keep `except Exception` only at program or worker boundaries.
- **Handle once:** either re-raise with `from err` or log and stop, never both. Never return `None`, `-1`, or `""` to signal failure (`X | None` is only for normal absence).
- **No `assert` for runtime input or state checks**, and no `try` around imports unless declared as an optional extra.

## 7. Data, I/O, and boundaries

- Use `pathlib.Path`, `with` for every resource, and `encoding="utf-8"` below Python 3.15.
- Call `subprocess.run([...], check=True)` with a list and timeout; never `shell=True` with external input.
- Use timezone-aware `datetime.now(UTC)`, `timedelta` spans, unit-suffixed bare numbers (`timeout_s`), and `orjson.loads(raw)` directly on bytes.
- Read configuration and environment once in `main()` and pass values down. Use `secrets` for tokens; never run `pickle`, `eval`, or `yaml.load` on untrusted data.
- Log with `logging.getLogger(__name__)` and lazy `%` formatting; libraries never configure logging or print.

## 8. Comments and docstrings

- Comments explain why (order, constants, dependency bugs), never what the next line does. No commented-out code, banners, history logs, or ownerless TODOs.
- Write one-line imperative docstrings (`"""Return rows changed since the last sync."""`) using plain searchable words. Add detail only for exceptions, units, or side effects not visible in the signature.

## 9. Projects and proof

- Use `pyproject.toml` with `[project]`, `src/<package>/`, a lockfile, `uv add` (ask before adding dependencies), and `main()` behind `[project.scripts]`. Never weaken linter or type checker settings to pass.
- Prove every change by running the real app or command on real data after the final edit. For projects with tests, read [references/testing.md](references/testing.md) for E2E, Hypothesis property, differential (`xpd equiv`), and pytest 9 rules.

## References

- Python 3.10 to 3.15 syntax table and full tool-to-rule mapping (`ruff`, type checker, `xpd check`, `xpd discover`) -> [references/versions-and-tools.md](references/versions-and-tools.md)
- Proof, pytest 9, Hypothesis profiles, and banned test plugins -> [references/testing.md](references/testing.md)
- External specs: [PEP 8](https://peps.python.org/pep-0008/), [Google Python Style Guide](https://google.github.io/styleguide/pyguide.html), [typing docs](https://typing.python.org/en/latest/), and [xpd rules](https://bjoernaagaard.github.io/xpd/rules/)
