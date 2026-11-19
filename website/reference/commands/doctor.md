Run `xpd doctor` at the root of a Python project to see which Ruff, ty and pytest settings to change and why. Doctor reads the project's configuration and Python files. It prints edits for you to review and apply. Ruff and ty run their own checks.

Doctor ships a catalog reviewed against Ruff 0.17.4, ty 0.0.106 and pytest 9.2.0. Doctor consults Ruff and ty in the project's environment or on `PATH`. It gets the installed pytest version from the project's virtual environment. Advice for a missing tool is marked “unverified locally”.

## Recommended Ruff rules

Doctor recommends seven rule codes. It suggests adding missing codes to `lint.extend-select`, which preserves your existing selection. An active code needs no edit. Deliberate exclusions can appear under “Project choices doctor keeps”.

| Code | What Ruff checks | What to do about a finding |
| --- | --- | --- |
| `PTH118` | Paths joined with `os.path.join` | Use a `Path` join where the caller accepts a path object. Preserve string or bytes contracts. |
| `PERF101` | A list created just to iterate over it | Iterate over the original iterable when a snapshot is unnecessary. |
| `PERF102` | A dictionary loop that discards the key or value | Iterate over the keys or values you actually use. |
| `PGH003` | A blanket `type: ignore` | Name the diagnostic you intend to suppress. |
| `PGH004` | A blanket `noqa` | Name the Ruff rule you intend to suppress. |
| `RUF017` | Lists combined with repeated addition | Flatten the inputs in one pass to avoid copying earlier items repeatedly. |
| `RUF100` | A `noqa` that no longer suppresses a finding | Remove the unused suppression. |

## Recommended ty settings

Doctor recommends three diagnostic levels under `[tool.ty.rules]` in `pyproject.toml`. Use the file and table named in your report if your project configures ty elsewhere.

| Rule | Level | Why enable it |
| --- | --- | --- |
| `blanket-ignore-comment` | `error` | A suppression without a diagnostic name can hide a new error on the same line. |
| `disjoint-cast` | `error` | Casting between types that cannot overlap makes a promise the value cannot satisfy. |
| `missing-type-argument` | `warn` | A bare generic such as `list` leaves the element type unspecified. |

## Recommended pytest settings

Doctor recommends strict mode, `-ra` for the test summary, and `--import-mode=importlib`. It names the TOML file and table where the settings belong. If it finds an INI or CFG file that pytest may use, it names the file in a note.

## When the advice depends on your project

Conditional advice names the decision you need to make and the evidence that raised it. For example, seeing scalar division is a reason to consider a ty warning. It does not establish that a particular divisor is zero.

| Project decision | Ruff rules or settings Doctor considers |
| --- | --- |
| Enforce typed API boundaries | `ANN401`, plus `allow-star-arg-any` and `ignore-fully-untyped` in the annotation settings. |
| Control when imports run | `PLC0415` for local imports, `TID254` and `TID255` for lazy imports, and `ban-lazy` for an explicit eager-import policy. |
| Keep annotations needed at runtime | `runtime-evaluated-base-classes` and `runtime-evaluated-decorators`, including Pydantic's `BaseModel` and `validate_call`. |
| Migrate numerical code | `NPY201` for NumPy 2 migrations and `PD011` for pandas code using `.values`. |
| Simplify loops or avoid unnecessary work | `PERF402` and `PERF403` for loops that copy items, `FURB192` for sorting to find an extreme, `RUF015` for selecting the first item, and `RUF065` for logging. |
| Maintain suppressions and preview policy | `RUF101` for obsolete rule codes, `RUF102` for unknown codes, and `lint.explicit-preview-rules` to opt into preview rules individually. |
| Diagnose an unexpected Ruff result | Doctor can suggest `ruff check --show-settings PATH` or `ruff check --show-files PATH` to inspect effective settings or included files. |

| Project decision | Conditional ty advice |
| --- | --- |
| Check arithmetic, conditional bindings or optional values | Consider `warn` for `division-by-zero`, `possibly-unresolved-reference` or `truthiness-test-of-none-union`, according to the code. |
| Protect typed boundaries | Consider `error` for `unsound-assignment`, `unsound-return` or `unsound-yield`. For decorators your project owns, consider `warn` for `dynamic-function-decorator-return`. |
| Check package metadata | Consider `missing-direct-dependency = "warn"` for packaged projects with complete dependency metadata that accept preview checks. |
| Change how ty analyzes the project | Consider disabling `respect-type-ignore-comments` when adopting ty-specific suppressions, enabling `strict-generic-narrowing` for generic container checks, or setting `return-type-inference` to `"always"` during gradual migration. |
| Run trusted standalone scripts with inline dependencies | Consider `project-manager.script = "uv"` with uv 0.12.3 or newer. |

The catalog also records which upstream defaults to leave in place and which settings inherit another decision. Each record identifies its release, reason and upstream source. [Browse the decision catalog](https://github.com/bjoernaagaard/xpd/blob/main/src/doctor/decisions.jsonl).

## Example from a small project

In a directory with no checker configuration, put this function in `sample.py`:

```python
def ratio(total: float, units: int) -> float:
    return total / units
```

Run `xpd doctor --format text`. With Ruff 0.17.4 and ty 0.0.107 installed, Doctor suggested adding three missing Ruff codes to the new project's configuration:

```toml
[tool.ruff.lint]
extend-select = ["PTH118", "PGH003", "PGH004"]
```

The other four recommended Ruff codes were already enabled. Doctor also suggested the three recommended ty levels above. Separately, it pointed to `return total / units` on line 2 as evidence for the conditional `division-by-zero = "warn"`. Review whether that check suits your code before adding it.

For a machine-readable report, run:

```sh
xpd doctor --format json | jq '.suggestions[] | {tool, id, file, table, key, value, condition, evidence, status}'
```

Read `tools` for version information, `suggestions` for proposed edits and inspection commands, `policies` for deliberate choices Doctor keeps, and `notes` for limits it encountered.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Doctor printed its report, including any suggestions. |
| 2 | Doctor could not run. |
