---
name: xpd-animate
description: Render and revise an editable Manim animation with xpd when the user wants a video, diagram, chart, formula, or concept explained in motion.
---

# Animate with xpd

Use `xpd animate` when the user wants a rendered explanation, diagram, chart, math animation, or concept demo. Use the app's own animation tools for live UI motion. Run `xpd animate -h` for full CLI flags.

## Workflow

1. Read the conversation and relevant code or data, sketch opening, key change, and outcome, and ask when multiple distinct concepts fit.
2. Compare sibling Manim primitives before coding: run `xpd animate catalog <family>` (or MCP `animate_catalog`) for families such as `indication`, `creation`, `transform`, `movement`, `composition`, `changing`, `graphing`, `rate_functions`, or `scene`. Write a short beat sheet naming visual intent, family checked, chosen primitive, and why it beat its siblings.
3. Check the local rootless Podman runtime with `xpd animate doctor --json` (or MCP `animate_status`). Run `xpd animate setup` once if the pinned Manim image is missing. Guides, catalog, and starter recipes work offline.
4. Start from a recipe (`xpd animate guide <recipe>`) or write a normal Manim `Scene` subclass in the project. Default to installed `Noto Sans`, `Noto Serif`, or `Noto Mono` fonts and Cairo-compatible objects. Pass local helpers, data, or `manim.cfg` via `--asset <path>` and locate them relative to `__file__`.
5. Iterate fast with `--profile draft` (`-ql`, 480p15), then render a workflow preview (`xpd animate start` or `--profile preview`, 1080p60) and inspect the contact sheet. Unchanged segments are cached per scene and profile so single-line edits reuse untouched beats.
6. Approve the inspected revision (`xpd animate approve`) to render final quality (`-qk`, 4K60). Return the video, beat sheet, and source bundle or manifest.

## CLI and profiles

```sh
xpd animate catalog indication
xpd animate guide sequence > cache_miss.py
xpd animate start cache_miss.py CacheMiss --json
xpd animate revise RUN --revision 1 --note "Hold last frame longer" --json
xpd animate approve RUN --revision 2 --json
```

Use `xpd animate status RUN --json`, `xpd animate resume RUN`, or standalone `xpd animate render <file> <Scene> --profile draft|preview|final|still` (`still` renders the last frame at 720p). Jobs live under `.xpd-animate/jobs/`; pass `--no-cache` to bypass verified cache reuse. Look up Manim signatures with `xpd api manim.Scene.play` or `xpd find --package manim "<query>"`.

## Local MCP

Use `animate_catalog`, `animate_guide`, `animate_status`, `animate_render` (accepting a project path or inline `source` `{entry, files, from_job}` plus `assets`), `animate_artifact` (`contact_sheet`, `video`, `source`, `manifest`, or `log`), and `animate_source` over local stdio.

## Starter recipes

| Recipe (`xpd animate guide`) | Scene class | Shows |
| --- | --- | --- |
| `sequence` | `CacheMiss` | Request cache miss followed by cache hit |
| `chart` | `SalesChart` | Monthly values revealed with March highlighted |
| `math` | `CompletingTheSquare` | Equation transformed into a square and solutions |
| `geometry` | `MovingGeometry` | Moving right triangle with fixed right angle |
| `sorting` | `SortingTrace` | Insertion sort with sorted prefix markers |
| `three_d` | `CameraOrbit` | Camera orbit around a rectangular prism in Cairo |
