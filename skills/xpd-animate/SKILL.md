---
name: xpd-animate
description: Render and revise an editable Manim animation with xpd when the user wants a video, diagram, chart, formula, or concept explained in motion.
---

# Animate with xpd

Use this guide when the user wants a rendered explanation, diagram, chart, math animation, or
demonstration of a concept implemented in the project. Use the app's animation tools for motion
that runs in the app itself.

1. Read the conversation and the relevant code or data. Identify the behavior to illustrate and
   sketch the opening, the key change, and the outcome. Ask which concept the user means when
   several materially different concepts fit the prompt.
2. Check the local runtime with `xpd animate doctor --json` or the MCP tool `animate_status`.
   The runtime uses a pinned Manim Community image and Cairo on rootless Podman. If the report
   says the image is missing, provision it once with `xpd animate setup`. Guides and starter
   scenes are available offline.
3. Use a recipe or write a normal Manim `Scene` subclass. Keep the main scene and its inputs
   in the project. Pass every local asset or imported helper needed at render time. Make
   settings for the scene's important timing and data easy to edit.
4. Start a workflow from the project root. Inspect the preview video for timing and the contact
   sheet for layout. If a change is needed, edit the scene or its assets and submit a revision
   with a short note. Inspect each new preview.
5. Approve the revision you inspected. Approval renders that saved scene and its assets at final
   quality. Return the video and source bundle or render manifest.

## CLI

Start a run and save the run ID printed by the command:

```sh
xpd animate guide sequence > cache_miss.py
xpd animate start cache_miss.py CacheMiss --json
```

Use the returned ID in place of `RUN`. For example, after changing the pause in
`cache_miss.py`, submit the edit, inspect revision 2, and approve it:

```sh
xpd animate revise RUN --revision 1 --note "Hold the last frame longer" --json
xpd animate status RUN --json
xpd animate approve RUN --revision 2 --json
```

Use `xpd animate resume RUN` after an interruption. For a standalone preview, final,
or still, use `xpd animate render cache_miss.py CacheMiss --profile still`.

The `preview` profile uses Manim's `-qh` (1080p60), and `final` uses `-qk`
(4K60). The `still` profile uses `-qm -s` for the last frame at 720p.
Bundle a `manim.cfg` with `--asset` to set scene defaults. xpd reads the
bundled config beside the scene, falling back to the bundled project-root
config, and sets the managed output paths and preset.

The result gives the video or still, a contact sheet for video inspection, the editable sources,
the saved render settings, and the render log. To revise, recover the scene and inputs from the
source bundle. For example, change `CACHE_MISS_HOLD_SECONDS` in `cache_miss.py` from `1.0`
to `3.0`. Render a new preview and check the extra pause before rendering a new final.

Jobs are saved under `.xpd-animate/jobs/` in the project. xpd reuses a verified result for
identical inputs and settings. Add `--no-cache` to check a render from scratch.
Use `xpd api manim.Scene.play` for a Manim method signature or
`xpd find --package manim "animate a camera around a prism"` for deeper search.

## Local MCP

Call `animate_guide` with a recipe name to read its code. Then call `animate_render` with
the scene class, profile, and project-relative source path. An agent that only has MCP can
send `source` as an object with `entry` and a `files` map from project-relative names to
Python source text. Add project-relative file paths in `assets` for existing data or media.
Use `animate_artifact` with the returned job ID to inspect the contact sheet or get the
video, source bundle, manifest, or log. Use `animate_source` with the job ID to read
the main scene or a named helper saved by a prior session. Render tools run over local stdio.
For a revision with saved helpers or assets, pass `source` with the original `entry`, the
saved render ID as `from_job`, and only the changed text files in `files`.

## Editable recipes

| `xpd animate guide` argument | Scene class | Shows |
| --- | --- | --- |
| `sequence` | `CacheMiss` | A client request that misses the cache, then a second request that hits |
| `chart` | `SalesChart` | A monthly chart that reveals values and highlights March |
| `math` | `CompletingTheSquare` | One equation transformed into a square and two solutions |
| `geometry` | `MovingGeometry` | A right triangle that changes while its right angle stays fixed |
| `sorting` | `SortingTrace` | Insertion sort with the sorted prefix marked after each step |
| `three_d` | `CameraOrbit` | A camera move around a rectangular prism with Cairo |

For a custom scene, import Manim and use its public API directly. Use `Text` for labels
and `MathTex` for formulas. For a project-relative asset such as `scenes/assets/input.png`,
pass `--asset scenes/assets/input.png` from the project root and locate it relative to
`__file__` inside `scenes/scene.py`. The scene runs with the packages and fonts in the
pinned image. Use an installed font such as Noto Sans and Cairo-compatible objects and effects.
