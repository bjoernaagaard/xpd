pyarrow is not installed, so pandas stores text columns as Python strings and the `.str` methods run slower. Add pyarrow to the project, which made them 3 to 11 times faster on 1,000,000 rows.

```sh
uv add pyarrow
```

Add the package to the project environment, not to xpd itself.

## When xpd warns

xpd warns when the project's venv has pandas and does not have pyarrow.
