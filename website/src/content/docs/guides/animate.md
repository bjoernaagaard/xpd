---
title: Animate a concept with Manim
description: Set up xpd's renderer, preview a scene, export an MP4, and revise it from the CLI or local MCP.
---

Use `xpd animate` to make an editable Manim video from a diagram, chart, equation,
or concept in your project. If you use the xpd agent plugin, ask in plain language.

> Animate how our cache handles a miss. Show the database call, then show the next request hitting the cache. Give me a preview first.

## Set up the renderer once

Rendering requires x86-64 Linux and rootless Podman with cgroups v2. Setup
downloads the pinned Manim Community image and checks it. Use `doctor` to see
whether the renderer is ready. The guide and starter scenes work offline.

If you need xpd on your PATH, install it with `uv tool install xpd`. Use
`uv tool upgrade xpd` if you already installed it with uv.

```sh
xpd animate doctor
xpd animate setup
xpd animate doctor --json
```

## Render your first video

Run from your project root. The `sequence` recipe writes an editable Python
file containing the Manim scene class `CacheMiss`.

```sh
xpd animate guide sequence > cache_miss.py
xpd animate render cache_miss.py CacheMiss --profile preview --json
xpd animate render cache_miss.py CacheMiss --profile final --json
```

Open the preview's `video` and `contact_sheet` paths before rendering the final.
The contact sheet samples the start, middle, and end. Play the preview to check
timing. Use `still` to render just the last frame.

| Profile | Output | Size and frame rate |
| --- | --- | --- |
| `preview` | MP4 and contact sheet | 854 × 480 at 15 fps |
| `final` | MP4 and contact sheet | 1280 × 720 at 30 fps |
| `still` | PNG of the last frame | 1280 × 720 |

Jobs live under `.xpd-animate/jobs/<id>/`. The JSON result gives you the media
paths, editable source archive, render settings, and log. To revise this
example, change `CACHE_MISS_HOLD_SECONDS` from `1.0` to `3.0` in
`cache_miss.py` and render another preview. For an exact repeat, xpd can reuse
the verified result. Add `--no-cache` to render again from scratch.

## Run a trusted scene directly

For a scene and imports you explicitly trust, use
[a local Manim installation](https://docs.manim.community/en/stable/installation.html).
Direct Manim runs the scene with your account's permissions.

```sh
manim checkhealth
xpd animate guide sequence > cache_miss.py
manim render --renderer cairo --format mp4 --resolution 854,480 --fps 15 --seed 0 cache_miss.py CacheMiss
```

## Use local MCP

Start the installed xpd server with your project root.

```sh
xpd mcp --root .
```

Call `animate_status` to check readiness and `animate_guide` with
`{"recipe":"sequence"}` to get the scene code. If your project already has
`cache_miss.py`, pass its path to `animate_render`.

```json
{"source":"cache_miss.py","scene":"CacheMiss","profile":"preview"}
```

For a client without filesystem access, use a `source` object with an `entry`
path and a `files` map from project-relative paths to source text. Put the
recipe text in that map. This minimal scene shows the shape of the request.

```json
{
  "scene": "CircleDemo",
  "profile": "preview",
  "source": {
    "entry": "circle.py",
    "files": {
      "circle.py": "from manim import Circle, Create, Scene\n\nclass CircleDemo(Scene):\n    def construct(self):\n        self.play(Create(Circle()))\n"
    }
  }
}
```

Pass existing project assets in `assets`, for example
`["scenes/data/monthly.csv"]`.

Use the returned render ID as `job_id` in `animate_artifact`. The default
artifact is the contact sheet. Choose `video`, `source`, `manifest`, or `log`
to retrieve those files. For a later revision, call `animate_source` with that
ID to read the saved scene. Submit the original `entry`, the ID as `from_job`,
and the changed text files in `files`. The renderer restores the saved inputs
and applies your edits. Rendering and artifact retrieval use local stdio.

## Render your own scene

Run `xpd animate guide` for all six recipes, or supply any Cairo-compatible
Manim scene that uses the packages in the pinned image. Add each local helper,
image, or data file with `--asset`, using paths relative to the project root.

```sh
xpd animate render scenes/chart.py SalesChart --profile preview \
  --asset scenes/data/monthly.csv --json
```

Inside the scene, resolve an asset relative to the scene file with
`Path(__file__).parent`. If a render fails, read the log path in the error.
Check that the requested scene class exists and that each required helper or
asset was passed. If setup is the issue, `xpd animate doctor` reports the
missing runtime or image.
