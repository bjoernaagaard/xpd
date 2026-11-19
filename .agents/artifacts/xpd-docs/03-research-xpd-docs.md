---
date: 2026-11-19T11:11:00Z
repos:
  - identifier: bjoernaagaard/xpd
    sha: 047efea0ee48d813e9112ec9092b99b640079893
topic: Public documentation and its build platform
type: research
tags: [research, documentation, mdbook]
status: complete
flow: design-discussion
---

# The rules page ships inside xpd and on the web

The Rust build reads the rule descriptions from the same Markdown file that mdBook publishes. A change to its headings can break a build, a link in a diagnostic, or the text a reader gets in their editor. The published quick start worked with xpd 0.38.0. The largest gaps we found were stale pack totals and missing command options.

## Readers enter through the package, repository or book

The package metadata points to the repository README and documentation. The README introduces installation, commands and integrations. The book gives each reader a route from a task to the relevant instructions. [Package metadata](../../../pyproject.toml), [README](../../../README.md), [Book contents](../../../docs/SUMMARY.md)

| Published page | What readers can do |
| --- | --- |
| [Get started](https://bjoernaagaard.github.io/xpd/) | Install xpd, read a finding and apply a fix |
| [Commands](https://bjoernaagaard.github.io/xpd/commands.html) | Look up a command, output format or exit status |
| [Claude Code](https://bjoernaagaard.github.io/xpd/claude-code.html) | Set up the hook, skill or plugin |
| [MCP server](https://bjoernaagaard.github.io/xpd/mcp.html) | Connect an agent and look up its tools and resources |
| [Code mode](https://bjoernaagaard.github.io/xpd/code-mode.html) | Follow a data workflow through checks and model fitting |
| [Language server](https://bjoernaagaard.github.io/xpd/lsp.html) | Set up diagnostics, rule help and fixes in an editor |
| [Rules](https://bjoernaagaard.github.io/xpd/rules.html) | Look up an individual rule or author one |

All seven chapter requests returned HTTP 200. The published HTML contained all 426 rule anchors emitted by the CLI and all 20 named cross-chapter targets found in project Markdown. The latter are listed below so a future site can be checked against them. External bookmarks cannot be enumerated from the repository.

| Page | Linked fragments |
| --- | --- |
| commands.html | api, check, equiv, find, graph, index, new, outline, profile, refit, run, step-code, test |
| claude-code.html | agent-plugin, agent-skill |
| code-mode.html | discovery-before-execution, the-gates, the-stages |
| index.html | install |
| mcp.html | settings |

## The catalog checks rule identities while people maintain descriptions and totals

`build.rs:157–244` matches pack headings and level-four rule headings to rule definitions. Missing, duplicate or unmatched sections fail the build. The catalog supplies the Markdown and published URL to consumers. The rule authoring command also writes sections in this format. [Build script](../../../build.rs), [Catalog](../../../src/catalog.rs), [Rule authoring](../../../src/author/project.rs), [Editor hover](../../../src/lsp.rs), [MCP resources](../../../python/xpd/mcp/catalog.py)

`xpd rules --builtin --format json` returned 427 records. Across 13 packs, 426 have book sections. The remaining record, `xpd/unused-suppression`, has neither a description section nor a URL in the catalog. The pack summary near the top of the rules page contains three stale counts.

| Pack | Rules page | xpd 0.38.0 |
| --- | ---: | ---: |
| pandas | 124 | 134 |
| NumPy | 59 | 63 |
| Python | 12 | 28 |

All 19 top-level commands have headings in the handwritten command reference. Coverage within commands varies. For example, the run section omits `--debug-query` and the outline section omits `--follow` and `--globs`. All 21 registered MCP tool names appear in the MCP table. Name coverage alone does not establish that every argument is documented. [Command reference](../../../docs/commands.md), [CLI](../../../src/cli.rs), [MCP reference](../../../docs/mcp.md), [MCP registration](../../../python/xpd/mcp/server.py)

## The first example reproduces five findings and two edits

We installed xpd with `uv tool install xpd` into an isolated tool directory and checked a copy of the repository sample. The install provided xpd 0.38.0. A reader following the literal sample path needs the checkout or a copy of the sample. The opening command also works with a path to the reader's own project. [Get started source](../../../docs/index.md), [Sample input](../../../samples/bad.py)

| Command on samples/bad.py | Observed result | Exit |
| --- | --- | ---: |
| uvx xpd check samples/bad.py | 5 findings | 1 |
| xpd check samples/bad.py --fix | 0 fixed, 5 findings | 1 |
| xpd check samples/bad.py --fix --unsafe-fixes | 2 fixed, 3 warnings remain | 0 |

The guide labels each field of a finding, explains the severity and shows the exact edits below. Captured output defaults to JSON. Running with `--format text` reproduced the human-readable summary, `xpd: fixed 2, 3 issues`.

```diff
-    df["a"][i] = row["a"] * 2
-df.dropna(inplace=True)
+    df.loc[i, "a"] = row["a"] * 2
+df = df.dropna()
```

## A site deployment and an xpd release follow separate paths

```text
docs/rules.md
├── Cargo build reads the rule sections and embeds them in xpd
└── mise run docs
    ├── mdbook-mermaid install . writes the diagram scripts
    └── mdbook build writes HTML, scripts and a search index to site/
        └── GitHub Actions uploads site/ and deploys it to Pages
```

The local command reported “HTML book written to” the checkout's site directory. The rules page used 892 KiB on disk and the search index used 4.7 MiB, measured with du. The Pages job watches docs, book.toml, mise.toml and its own workflow on main. It can also run manually. Changes confined to CLI code, rule YAML or the README do not trigger a Pages deploy. A Markdown edit can update the website while an installed xpd still serves its earlier embedded text. Six redirects preserve the previous chapter URLs ending in /index.html. [Book configuration](../../../book.toml), [Build task](../../../mise.toml), [Deployment](../../../.github/workflows/docs.yml), [Successful Pages run](https://github.com/bjoernaagaard/xpd/actions/runs/44429334924)

The CI workflow runs lefthook on pull requests. Its rule load check runs xpd against samples/good.py and its ranking check uses indexes/questions.json. The Rust build checks rule sections. The docs task renders the site. Neither the listed hooks nor that task compares command help with prose, tests the quick start transcript, or checks the published anchors. We inspected those checks and ran the docs build, CLI examples, HTTP and browser checks for this report. [Hooks](../../../lefthook.yml), [CI](../../../.github/workflows/ci.yml)

## Desktop and mobile readers can reach an individual rule by search

At 1280 by 800 pixels the sidebar measured 300 pixels and the main content 750. Pressing the slash key focused search. Searching for chained-assignment returned 30 hits. Down followed by Enter opened the rule at its named anchor. At 390 by 844 pixels the chapter menu began closed. Opening it showed seven chapters and in-page headings. All four diagram chapters rendered SVGs on mobile. The MCP sequence diagram fit the 350 pixel content area without horizontal page overflow.

Body copy uses Open Sans at 16 pixels. Sidebar links use 14 pixels with a 21 pixel line height. Code blocks use Source Code Pro at 14 pixels with 10 pixels of padding. Light-theme table cells use 3 pixels of vertical and 20 pixels of horizontal padding with a 1 pixel #f2f2f2 border. The sampled sidebar, main area, table and code block had no shadow.

| Theme | Page background | Text | Code background |
| --- | --- | --- | --- |
| Light | #ffffff | #000000 | #f6f7f6 |
| Rust | #e1e1db | #262625 | #f6f7f6 |
| Coal | #131516 | #98a3ad | #1d1f21 |
| Navy | #161923 | #bcbdd0 | #1d1f21 |
| Ayu | #0f141a | #c5c5c5 | #191f26 |

The MCP diagram and code sample rendered in every named theme. Auto used Light under the browser's default color preference. The check covered keyboard navigation and a mobile viewport. Physical phones, screen readers and contrast ratios remain unverified.

## The site generator and its diagram plugin have separate release schedules

Stable versions checked on 19 November 2026 are below. The alternatives were checked against official documentation. No migration was run.

| Tool | Project or installed version | Latest stable release checked |
| --- | --- | --- |
| mdBook | 0.5.0 | [0.5.4, 6 July](https://crates.io/crates/mdbook/0.5.4) |
| mdbook-mermaid | 0.17.1 | [0.17.1, 14 August](https://crates.io/crates/mdbook-mermaid/0.17.1) |
| Mermaid JavaScript | 11.6.0 in the plugin's generated asset | [12.3.0, 3 November](https://www.npmjs.com/package/mermaid/v/12.3.0) |
| Starlight | Alternative | [0.43.1, 13 November](https://www.npmjs.com/package/@astrojs/starlight/v/0.43.1) |
| Sphinx | Alternative | [9.2.0, 21 October](https://pypi.org/project/Sphinx/9.2.0/) |

The plugin supplies Mermaid's JavaScript, so installing the newest plugin still gives the site Mermaid 11.6.0. The Pages workflow pins configure-pages 6.0.0, checkout 7.0.1, upload-pages-artifact 5.0.0 and deploy-pages 5.0.1. Those match the stable tags checked. The workflow pins mise-action 5.2.6, and upstream offers 5.2.10. Local mise reported 2026.11.9, with 2026.11.10 available. The workflow sets no separate mise binary version. [Workflow pins](../../../.github/workflows/docs.yml), [mise-action release](https://github.com/jdx/mise-action/releases/tag/v5.2.10)

| Platform | Reader features | References, examples and checks |
| --- | --- | --- |
| mdBook | Built-in search, keyboard navigation, themes and highlighted code. The current site uses these and the Mermaid preprocessor. | Preprocessors can supply content. Built-in example testing targets Rust. xpd builds its Python examples as displayed code and maintains the CLI reference in Markdown. [Manual](https://rust-lang.github.io/mdBook/) |
| Starlight | Accessible navigation, Pagefind search and highlighted code. | Its plugin directory lists generators for Python, TypeDoc and OpenAPI, runnable code blocks, Mermaid integration and link validation. [Search](https://starlight.astro.build/guides/site-search/), [Plugins](https://starlight.astro.build/resources/plugins/) |
| Sphinx | HTML search and selectable themes. Accessibility depends in part on the chosen theme and content. | autodoc reads Python APIs, doctest runs Python examples, Graphviz supports diagram alt text, and linkcheck checks targets. [Extensions](https://www.sphinx-doc.org/en/master/usage/extensions/index.html), [Builders](https://www.sphinx-doc.org/en/master/usage/builders/index.html) |

Neither alternative's listed API generators directly consume xpd's Rust command definitions and YAML rule catalog. The choice of platform, how to maintain the references and which capabilities to adopt belong to the design discussion.

[Original research questions](02-research-questions-xpd-docs.md)
