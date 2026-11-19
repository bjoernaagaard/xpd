orjson reads UTF-8 bytes directly, so decoding them to a str first only adds work. Pass the bytes to `orjson.loads`.

```diff lang="python"
 import orjson
 
 
 def read_quote(body: bytes) -> dict[str, object]:
-    return orjson.loads(body.decode("utf-8"))
+    return orjson.loads(body)
```

## What the rule reports

The rule reports `orjson.loads(raw.decode())`, and the same call with UTF-8 as the encoding,
written as `"utf-8"`, `"utf8"` or `"UTF-8"`, given by position or as `encoding=`. It leaves alone
a decode with another encoding, a decode with an `errors` argument, and `json.loads`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `orjson.loads(raw)`.

xpd applies the replacement with `--fix --unsafe-fixes`.

[orjson](https://github.com/ijl/orjson#deserialize) reads UTF-8 bytes directly, and its
documentation says to pass bytes "rather than creating an unnecessary `str` object". Given a
`str`, it first turns the text back into UTF-8, so the decode before the call does the same work
twice. On Python 3.15.2 on one machine, reading a document of 161 KB with Chinese text took 83.1
microseconds from the bytes and 182.9 from `raw.decode()`, 2.20 times as long. An ASCII
document of 62 KB took 37.1 and 38.8 microseconds. Both gave equal results. `json.loads` from
the standard library decodes bytes itself at the same cost, so the rule leaves it alone.

The fix is unsafe, because bytes that are not valid UTF-8 then raise `orjson.JSONDecodeError`
in place of `UnicodeDecodeError`. Catch `ValueError`, which both of them are, where the code
handles bad input.
