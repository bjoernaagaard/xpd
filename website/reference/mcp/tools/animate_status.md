Check whether this host has the local Manim runtime and image. Use
`xpd animate setup` on the renderer host when the status reports a missing image.

## Example

```json
{}
```

```json
{"ready":"...","version":"0.22.1","reason":"..."}
```

## Errors

A runtime that is unavailable normally reports `ready: false` with a reason.
An unsupported host can return a tool error.
