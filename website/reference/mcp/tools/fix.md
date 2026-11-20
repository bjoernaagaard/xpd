Use `fix` to write xpd's fixes, as `xpd check --fix` does. It writes the safe fixes,
then lints what is left and returns the same result as [`check`](../check/). With `unsafe`, it
first counts the unsafe fixes and asks the user to confirm them, because an unsafe fix may change
what the code does.

```text
fix(paths, unsafe=true)
  lint paths and count the issues whose fix is "unsafe"
  if the count is above zero
    ask the user "Apply 2 unsafe fixes in bad.py? An unsafe fix may change what the code does."
    if the client has to collect the answer first
      return the question and write no file
    if the user declines
      write the safe fixes only and return what is left
  write the safe and the unsafe fixes and return what is left
```

A client on a protocol older than 2026-07-28 gets the question during the call. A client on
the 2026-07-28 protocol, or one that runs `fix` as a background task, gets the question as the
result, and calls `fix` again with the answer.

## Example

On `samples/bad.py`, whose two fixes are both unsafe, the user declined the question.
The tool wrote no fix and returned the 5 issues that are left.

```json
{"paths":["samples/bad.py"],"unsafe":true}
```

```json
{"fixed":0,"fixable":0,"issues":"..."}
```

## Errors

A path outside the project root fails before xpd runs. A client that cannot ask its user gets an error for an unsafe fix, and the server
writes no file.
