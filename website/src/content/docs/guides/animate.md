---
title: Animate a concept with Manim
description: Set up xpd's Manim renderer, preview a scene, export a 4K MP4, and revise it from the CLI or local MCP.
---

Run `xpd animate` to render and revise an editable Manim video from a built-in starter recipe or your own scene using rootless Podman on x86-64 Linux. If you use the xpd agent plugin, ask the agent in plain language to animate any concept in your project.

```mermaid
flowchart LR
    A["xpd animate doctor and setup"] --> B["xpd animate guide <recipe> > scene.py"]
    B --> C["xpd animate render --profile preview"]
    C --> D["Inspect video and contact sheet"]
    D -->|Edit scene.py| C
    D -->|Approved| E["xpd animate render --profile final"]
```

## Set up the renderer once

Container rendering requires x86-64 Linux and rootless Podman with cgroups v2. Starter recipes and guides work offline.

```sh
xpd animate doctor
xpd animate setup
xpd animate doctor --json
```

## Render and revise a scene

Generate an editable starter scene, render a 1080p preview with a contact sheet, tweak timing or layout, and render the 4K final video. Jobs are cached under `.xpd-animate/jobs/<id>/` (pass `--no-cache` to force a fresh render).

```sh
xpd animate guide sequence > cache_miss.py
xpd animate render cache_miss.py CacheMiss --profile preview --json
xpd animate render cache_miss.py CacheMiss --profile final --json
```

| Profile | Manim preset | Output | Resolution |
| --- | --- | --- | --- |
| `preview` | `-qh` | MP4 and contact sheet | 1920 by 1080 at 60 fps |
| `final` | `-qk` | MP4 and contact sheet | 3840 by 2160 at 60 fps |
| `still` | `-qm` | PNG of the last frame | 1280 by 720 |

Pass `--asset <path>` (relative to the project root) for any helper module, data file, or `manim.cfg`, and resolve assets inside Python via `Path(__file__).parent`.

```sh
xpd animate render scenes/chart.py SalesChart --profile preview \
  --asset scenes/data/monthly.csv --asset manim.cfg --json
```

For scenes you trust and run without Podman, install [Manim Community](https://docs.manim.community/en/stable/installation.html) locally and run `manim render --renderer cairo --format mp4 -qh --seed 0 cache_miss.py CacheMiss`.

## Render from local MCP

Start `xpd mcp --root .` and call the five `animate_` tools over stdio.

| MCP tool | Purpose |
| --- | --- |
| `animate_status` | Check Podman and image readiness |
| `animate_guide` | Return the animation guide or one of the six starter recipes |
| `animate_render` | Render a file path, inline `source.files` map, or `from_job` revision |
| `animate_artifact` | Retrieve `contact_sheet` (default), `video`, `source`, `manifest`, or `log` by `job_id` |
| `animate_source` | Read saved scene files from a completed `job_id` before editing |

Clients without filesystem access can submit inline source text directly to `animate_render`.

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
