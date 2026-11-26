A module named `utils`, `helpers`, `common`, `config` or `types` does not say what it holds. Name it after its contents, and split it when the contents have no single name.

```diff lang="text"
 orders/
-├── utils.py        # parse_date, round_price, retry
+├── dates.py        # parse_date
+├── prices.py       # round_price
 └── report.py
```

## What the rule reports

The rule reports a module named `util`, `utils`, `helper`, `helpers`, `common`, `misc`, `config`,
`types`, `constants` or `handlers`, and a package with one of the same names.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The name says nothing about what the module holds, so unrelated code collects in it. Every
other module then imports it. A search for `config.py` or `types.py` also lands on one such file
in every package, and the path is the only thing that tells them apart. xpd reports the file on
line 1.
