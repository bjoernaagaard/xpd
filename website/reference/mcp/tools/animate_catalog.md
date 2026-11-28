Browse the bundled Manim Community 0.22.1 module catalog offline to compare sibling primitives side by side before writing scene code. Omit `family` to list every module family, or pass a family name such as `indication`, `creation`, `transform`, `movement`, `graphing`, or `rate_functions`. See [`xpd animate catalog`](../../../commands/animate/catalog/) and the [animation guide](../../../guides/animate/).

## Example

```json
{"family":"indication"}
```

```json
{"version":"0.22.1","query":"indication","modules":["manim.animation.indication"],"symbols":["..."]}
```

## Errors

An unknown family name fails with the CLI error listing valid module families.
