Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item.

Run `xpd untangle gate fix-id`.

## Example

```console
$ xpd untangle gate --help | sed -n '/^Usage:/p'
Usage: xpd untangle gate [OPTIONS] <FIX>
```
