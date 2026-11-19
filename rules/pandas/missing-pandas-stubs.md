The project has a type checker but not pandas-stubs, so calls to pandas are not type checked. Add pandas-stubs as a development dependency.

```sh
uv add --dev pandas-stubs
```

Add the package to the project environment, not to xpd itself.

## When xpd warns

Without pandas-stubs, a type checker doesn't check calls to pandas.

The warning appears when pandas-stubs is not installed and the project uses mypy, pyright, basedpyright, ty or pyrefly.
