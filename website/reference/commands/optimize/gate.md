Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict. A design fix must edit a file that owns a cited checklist item.

Run `xpd optimize gate fix-id`.

## Example

```console
$ xpd optimize gate --help | sed -n '/^Usage:/p'
Usage: xpd optimize gate [OPTIONS] <FIX>
```
