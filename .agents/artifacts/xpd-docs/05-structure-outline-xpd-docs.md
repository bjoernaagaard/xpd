---
type: structure-outline
flow: design-discussion
repos:
  - identifier: bjoernaagaard/xpd
    sha: 7a5a287239e9122802a455a753ff0ba2293e5342
---

# Rewrite the xpd documentation and publish it with Starlight

A new reader should be able to copy a small Python file, run xpd and see exactly what a fix changes. Someone following a finding from an editor should reach a dedicated rule page with the same explanation that xpd ships. Starlight will publish the guides, reference pages and pack indexes. Pagefind will search them. [Settled design](04-design-discussion-xpd-docs.md)

## Implementation overview

- [x] ✅ 1. Build a working Starlight quick start.
- [x] ✅ 2. Give every rule a page and share its explanation with installed tools.
- [x] ✅ 3. Generate command and MCP references from the running tools.
- [x] ✅ 4. Rewrite the public guides and package README.
- [x] ✅ 5. Verify existing links and switch GitHub Pages to Starlight.

Develop and verify the phases together before the production switch. The deployed book continues serving its existing URLs until the complete replacement passes the final checks.

## ✅ 1. Build a working Starlight quick start

**Change outline.** Add the Astro and Starlight site configuration, Node package manifest and lockfile. Configure the GitHub Pages base path `/xpd/`, static output and Pagefind. Check current official releases at implementation time. Pin the newest mutually compatible stable versions of Node, Astro, Starlight and the diagram integration. Record the versions actually resolved. Keep the candidate build separately invocable until the existing Pages workflow can publish the whole site.

Rewrite the quick start around a copyable example drawn from [samples/bad.py](../../../samples/bad.py). Give the reader a command, an explanation of its output, and a diff. Distinguish the default safe fixes from edits requiring explicit consent.

**Automated verification.** Install from the lockfile and run the site's production build. Serve the static output under its deployment prefix. Check that the start page, its assets and the Pagefind index load. Run the published commands on a temporary copy of the example. Compare the displayed lines, severities, exit statuses and edits with the observed output. Use an explicit text format when capturing a terminal transcript.

**Manual verification.** Follow the quick start from an empty directory. Find the page again using keyboard search, then read the sample and diff at phone width.

**Progress on 19 November 2026.** The site lives in `website/`, and `mise run docs:starlight` builds it into `website/dist/`. The `docs-preview` daemon in `pitchfork.toml` builds the site the same way and serves it at `http://localhost:4321/xpd/`. The existing `mise run docs` still builds the mdBook site, and the Pages workflow is unchanged.

| Tool | Resolved version | Where it is pinned |
| --- | --- | --- |
| Node | 26.13.1, the current LTS release | `mise.toml` |
| Astro | 7.4.13 | `website/package.json` and `package-lock.json` |
| Starlight | 0.43.1 | the same |
| Astro Markdown processor, Sätteri | 0.4.10 | the same, with smart punctuation off as in mdBook |
| astro-mermaid, the integration that Starlight's plugin list names | 2.2.0 | the same |
| Mermaid | 12.3.0 | the same |
| Expressive Code line numbers | 0.44.4 | the same, matching Starlight's Expressive Code |
| Pagefind | 1.6.0 | the lockfile, through Starlight |

- [x] `npm ci` installs from the lockfile and `npm run build` finishes. Astro reports that the `i18n` collection and a `404` entry are missing. Starlight's own 0.43.1 starter reports the same two warnings.
- [x] Served under `/xpd/`, the start page, its 10 linked assets and `pagefind/pagefind-entry.json` return 200.
- [x] On a temporary copy of the sample taken from the page's copy button, the three output blocks match xpd 0.38.0 line for line with `--format text`. The exit statuses are 1, 1 and 0, and the safe fix leaves the file unchanged. The unsafe fixes write the two lines that the page's diff shows.

The agent also ran the manual steps in headless Chrome for Testing 158. Ctrl+K opened search, and "unsafe fixes" returned the page and three of its sections. Starlight binds Ctrl+K, while mdBook used the slash key. At 390 pixels wide, no code block or page scrolled sideways, and the diagram rendered in light and dark themes. A tap on the search button and a search for "chained assignment" opened the page at "Check a sample file". The user accepted these check results on 19 November 2026.

- [x] Follow the quick start from an empty directory.
- [x] Find the page again using keyboard search.
- [x] Read the sample and diff at phone width.

## ✅ 2. Give every rule a page and share its explanation with installed tools

**Change outline.** First capture the old catalog's IDs, URLs and fragment targets from the baseline commit. Move authored explanations into Markdown keyed by pack and rule ID. Rewrite each entry in plain language. Lead with the consequence and action, then show a small example and its fix. Where xpd offers no automatic fix, describe what the reader can change. Include the special diagnostic `xpd/unused-suppression`, whose baseline record lacks a page.

```text
rules/ and Markdown keyed by pack and rule ID
    build.rs validates and embeds entries in xpd
        CLI, editor hover and MCP read the catalog
        Starlight generates a page for each rule and an index for each pack
```

Update [build.rs](../../../build.rs), [the catalog](../../../src/catalog.rs), [the pack loader](../../../src/pack.rs) and [the rule author](../../../src/author/project.rs) together. Preserve missing, duplicate and orphan entry checks. Build xpd before exporting the site data so Cargo can build from a clean checkout. Have the site derive counts and fix metadata from the catalog. Keep public URL changes together with the Pages switch so released tools always link to published pages.

**Automated verification.** Run `cargo build` and `cargo run --quiet -- rules --builtin --format json`. Compare the full ID sets across catalog records, pages and pack indexes, and compare pack counts directly to the records. Check one page per ID and a link back to the owning pack. Run the documented `xpd new rule` flow in a disposable project and check where its explanation is written. Use a real MCP client and an LSP hover request to read the same rule as the CLI.

**Manual verification.** Open pandas/chained-assignment from the candidate catalog. Read its consequence, example and unsafe fix together, then follow the link to the pandas pack. Check a rule with no automatic fix as well.

**Progress on 19 November 2026.** Each rule's text now lives in `rules/<pack>/<id>.md` beside its YAML, and `rules/<pack>/pack.md` introduces the pack and lists each id once under a group heading. xpd's own rule has its text in `rules/xpd/`. `rules/README.md` introduces the rules overview. `build.rs` embeds the Markdown and no longer reads `docs/rules.md`. `website/scripts/rules.mjs` runs `cargo run -- rules --builtin --format json` before `astro build` and writes the overview, 14 pack pages and 427 rule pages into `website/src/content/docs/rules/`, which git ignores. The catalog URLs still point to `rules.html#<id>`, the mdBook site still builds them, and the Pages workflow is unchanged. `xpd/unused-suppression` has a page and text but no catalog URL until phase 5. The baseline IDs, URLs and fragments from commit 7538988 are in [legacy-rule-targets.json](legacy-rule-targets.json).

Every rule entry starts with a new lead that states the consequence and the action, followed by a diff of the example and its fix, then "What the rule reports" and "How to fix it". The longer notes kept their measured wording, with the irregular fix sentences rewritten and the four package-check pages rewritten.

- [x] `cargo build` and `cargo run --quiet -- rules --builtin --format json` give 427 records, all with `doc`, and 426 with the old URL. Every field but `doc` equals the baseline.
- [x] The ID sets of the catalog, the built rule pages and the pack index tables are equal, at 427 each. Each pack page's count equals its records, for example pandas 134, NumPy 63 and Python 28. Each rule page has the rule ID as its title and links back to its pack, and 11,304 internal links resolve.
- [x] The build fails with a message when a rule has no Markdown, when a Markdown file names no rule, when `pack.md` leaves an id out or lists it twice, and when two packs share an id.
- [x] `xpd new rule team/no-print ... --with-test --yes` in an empty project writes `rules/team/no-print.md` beside `rules/team/no-print.yml` and no `docs/rules.md`. `xpd check` then reports the rule.
- [x] A FastMCP 4.2 client reading `xpd://rules/pandas/chained-assignment`, `pandas/merge-without-validate` and `xpd/unused-suppression` from `python -m xpd.mcp` gets text equal to the CLI `doc`. An LSP hover on line 6 of `samples/bad.py` returns the CLI `doc` followed by the `rules.html#chained-assignment` link.
- [x] `xpd find --write-vectors` embedded the 426 changed records, and `xpd find --check` passes 53 of 53 questions. `cargo fmt --check`, clippy with `-D warnings`, the rule load check on `samples/good.py`, `mise run docs` and `mise run docs:starlight` pass. The mdBook `rules.html` still has all 426 baseline anchors.

The agent also ran the manual steps in headless Chrome for Testing 158 against the `docs-preview` daemon. From the rules overview, the pandas link and then the chained-assignment link opened the rule, which showed its lead, the diff and the unsafe fix command, and its pack link returned to the pandas pack. `pandas/merge-without-validate` showed "none, edit by hand" and the sentence on making the change by hand. Ctrl+K searches for `chained-assignment`, `pandas/chained-assignment`, `chained assignment` and `merge validate` each listed the rule page first. At 390 pixels wide the rule and pack pages did not scroll sideways in light and dark themes. The user accepted these results on 19 November 2026.

- [x] Open pandas/chained-assignment from the candidate catalog and follow the link to the pandas pack.
- [x] Check a rule with no automatic fix.

## ✅ 3. Generate command and MCP references from the running tools

**Change outline.** Use the CLI command definitions behind [src/cli.rs](../../../src/cli.rs) and the registrations in [the MCP server](../../../python/xpd/mcp/server.py) for names, nested commands, flags, arguments, defaults and tool signatures. Keep explanations of when to use a command, worked examples and exit behavior in authored prose. Cover MCP prompts and resources alongside its tools. Make generated fields part of the docs build so an option change appears on the next build.

**Automated verification.** Walk the built command tree and compare its public options with the rendered reference, including subcommands. Run `cargo run --quiet -- outline --help` and `cargo run --quiet -- run --help`. Verify that the appropriate pages include `--follow`, `--globs` and `--debug-query`. Start xpd's MCP server and compare its live tool, resource, resource template and prompt listings with the reference. Use the current inventories during verification. The research baseline was 19 top-level commands and 21 registered tools.

**Manual verification.** Search for a flag and a tool name. From each result, check that a reader can identify the owning command or tool and see how to supply the argument.

**Progress on 19 November 2026.** A hidden command, `xpd commands`, prints the clap command tree as JSON through clap's public getters, with each visible command, argument, default, possible value, conflict and required group. clap-markdown 0.1.5 writes one Markdown file and clap_mangen 0.3.4 writes man pages, so neither gives fields that the site can combine with authored text. `website/scripts/mcp_listing.py` starts `python -m xpd.mcp` over stdio, as `xpd mcp` does, once as is and once with `--code-mode`, and lists its tools, resources, resource templates, prompts, instructions and `--help`. `website/scripts/reference.mjs` runs both after `rules.mjs` and writes 23 command pages under `commands/` and the MCP overview, 21 tool pages, resources and prompts under `mcp/`, which git ignores. Each option and argument is a heading with its syntax, default, values and conflicts. Each tool shows its call, its arguments with every nested field by dotted path, and its result fields.

When to use each command and tool, worked examples, exit status and tool errors are written by hand in `website/reference/`, one file for each command and tool. The build fails when a command or tool has no file, when a file names none, when a command page has no "Exit status" section or a tool page no "Errors" section, and when a page names a flag that its command does not take. The 31 arguments without help text got it in the clap definitions, and 47 MCP inputs got descriptions, so `--help` and the server show the same text as the site. The build installs the server's dependencies from `pyproject.toml` with uv into `website/.mcp-venv`. `xpd index` has its page at `commands/xpd-index/`, because Astro reads a page named `index` as its folder's page. The mdBook site and the Pages workflow are unchanged.

| Tool | Resolved version | Where it is pinned |
| --- | --- | --- |
| uv | 0.13.9, the newest release | `mise.toml` |
| Python for the listing | 3.14.10. uv offers 3.15 here only as a free-threaded build | `website/scripts/reference.mjs` |
| FastMCP | 4.2.2 | `pyproject.toml`, `~=4.2.0` |

- [x] `mise run docs:starlight` builds 492 pages with the two warnings of phase 1. `cargo fmt --check`, clippy with `-D warnings`, the rule load check on `samples/good.py`, `xpd find --check` with 53 of 53 questions, `ruff format --check` and `ruff check` pass. `mise run docs` still writes the mdBook site.
- [x] `xpd --help` lists 19 commands, and `xpd new` has 4 of its own. For all 23, a script parsed `--help` and found every one of 125 options and arguments as a heading of its page, with every default and possible value.
- [x] `cargo run --quiet -- outline --help` and `cargo run --quiet -- run --help` list `--follow` and `--globs`, and `run` lists `--debug-query`. The outline page has headings for `--follow` and `--globs`, and the run page for all three.
- [x] `fastmcp list` on the live server gave 21 tools, 10 resources and 5 prompts, and a client listed 2 resource templates and 3 code-mode tools. Every tool page has each of 363 arguments and nested fields, and the resources and prompts pages have every URI, template, prompt and argument.
- [x] The 27 `$ xpd` lines in the authored pages give their shown output when run from the repository root, with pandas 3.1.1 and scikit-learn 1.9.1 for `profile`, `equiv` and `refit`. The tool examples are results of live calls to all 21 tools. 42,720 internal links on 492 pages resolve, fragments included.
- [x] A wrong flag in a page and a page that names no command each stop the build with a message.

The agent also ran the manual steps in headless Chrome for Testing 158 against the `docs-preview` daemon. Ctrl+K searches for `--follow`, `debug-query`, `globs`, `discovery_gate`, `update_snapshots` and `unsafe_fixes` showed results titled by their command or tool, such as "xpd outline" with a "--follow" section and "check tool" with an "unsafe_fixes" section. Two Tabs and Enter opened `xpd run`, the `discovery_gate` tool and the `test` tool, and a click on the "--follow" section opened `commands/outline/#--follow`. A search for `--follow` lists the plotly pack page first, because Pagefind drops the dashes and the pack text uses "follow". At 390 pixels wide, 8 reference pages did not scroll sideways in light and dark themes, and the MCP sequence diagram rendered in both. The user authorized verified browser and integration checks as sign-off for phases 3 through 5, including publication and live verification.

- [x] Search for a flag and a tool name, and from each result identify the owning command or tool and how to supply the argument.

## ✅ 4. Rewrite the public guides and package README

**Change outline.** Use the seven chapters in [the existing book](../../../docs/SUMMARY.md) as the coverage inventory. Rewrite the README used on GitHub and PyPI, installation, commands, rules and rule authoring, Claude Code, MCP setup, code mode and language server guides. Link guides to generated facts rather than copying counts or option tables. In each workflow, lead with what the reader can do, explain prerequisites and show the result. Use a diagram for interactions and a diff for changes to code. Keep examples readable wherever the package README is rendered.

**Automated verification.** Run the documented local commands on copies of the examples. For the full bad.py baseline, run check, then safe fixes, then explicitly enabled unsafe fixes. Compare with the observed five findings, zero safe edits and two unsafe edits. Drive each integration through its normal entry point, verify the displayed response or output, and report any example requiring unavailable external access separately. Build the complete site and check internal links, images and diagrams.

**Manual verification.** Start at the README and follow installation through the first fix without a checkout. In the site, navigate to rule authoring, connect an editor and locate an MCP tool. Check keyboard focus, search results, code readability, diagram labels and horizontal scrolling on desktop and at phone width in each supported theme.

**Progress on 19 November 2026.** Five guides live in `website/src/content/docs/guides/`, at `guides/claude-code/`, `guides/mcp/`, `guides/code-mode/`, `guides/editors/` and `guides/write-a-rule/`, and the sidebar lists them after Get started. They cover the hook, the skills and the agent plugin, MCP setup and settings, code mode, the language server, and rule authoring for a project and for xpd's packs. They link to the generated command, MCP and rule pages for options, counts and tool fields. The README leads with installation and the copyable `bad.py`, and it links each guide and rule by its new address. Get started links to the guides. Phase 5 removed the old book chapters in `docs/`.

`samples/pipeline/discovery.json` named `sklearn.conftest.fetch_species_distributions`, which the index now gives as `sklearn.datasets.fetch_species_distributions`, so the gate `discovery-is-recorded` failed. The record was written again from `find` and `api`, as the code mode guide tells readers to do.

- [x] With xpd 0.38.0 from `uvx` in an empty folder, the README's `bad.py` gave 5 findings and exit 1, `--fix` left the file unchanged with exit 1, and `--fix --unsafe-fixes` wrote the two lines of the README's diff with exit 0. Every `$ xpd` line of the README, the Claude Code guide and the rule authoring guide gave its shown output from this checkout's build.
- [x] Claude Code 2.1.328 in a scratch project with the guide's `.claude/settings.json` ran `xpd hook` after an Edit, which exited 2, and its transcript holds the hook error with all five rules. `claude mcp add xpd -- uvx xpd mcp` and `claude mcp list` showed `✔ Connected`. `fastmcp list` and `fastmcp call` ran as the MCP guide shows, `check` returned the guide's issue fields, and the server over HTTP answered `initialize` with 200. `npx skills add bjoernaagaard/xpd` installed the three skills, and in a scratch `CODEX_HOME`, Codex 0.170.0 installed the plugin and listed its `xpd` server.
- [x] Following the code mode guide's setup in a new uv environment, `run_pipeline.py` from the page passed all 15 gates in 7.5 seconds with Python 3.15.2, scikit-learn 1.9.1 and pandas 3.1.1. With `leaky.py` the run stopped at `check` on `sklearn/tune-on-test` at line 74, and `predict.py` gave the shown rows, right for 85 of 86.
- [x] A script that speaks LSP over stdio, as an editor does, got 5 diagnostics for `bad.py`, the unsafe quick fix of line 6, the hover with the rule page link and `MethodNotFound` for completion. The Neovim, Helix, Zed and VS Code configurations were not run, because this machine has none of the editors, and the bas proxy refuses downloads from github.com other than git.
- [x] `mise run docs` built 497 pages. 48,206 internal links and sources in the built site resolve, 6,734 of them with fragments. The built site has no images, and every Mermaid diagram rendered in the browser.

The agent also ran the manual steps in headless Chrome for Testing 158 against the `docs-preview` daemon. At 1280 and 390 pixels wide, in the light and dark themes, Get started, the five guides, a rule page and the MCP overview had no sideways scroll and no code block that scrolls sideways, and each diagram rendered with its labels inside the content column. Ctrl+K searches for `xpd hook`, `unsafeFixes`, `write a rule` and `code mode gates` listed the hook command, the check command, the rule authoring guide and the gates tool, and two Tabs and Enter opened each one. Tab moved a visible outline from "Skip to content" through the header to the sidebar.

- [x] Start at the README and follow installation through the first fix without a checkout.
- [x] In the site, navigate to rule authoring, connect an editor and locate an MCP tool.
- [x] Check keyboard focus, search results, code readability, diagram labels and horizontal scrolling on desktop and at phone width in each theme.

## ✅ 5. Verify existing links and switch GitHub Pages to Starlight

**Change outline.** Make the existing `mise run docs` build the complete Starlight site into `site/`. Update [Pages publishing](../../../.github/workflows/docs.yml) to build on pull requests and deploy verified main builds. Include the site sources, dependency locks, rule text, CLI and MCP definitions, and other build inputs in its triggers. Update compatible workflow dependencies. Retire mdBook configuration and assets once their consumers have moved. Publish the new site before releasing binaries that emit the new URLs.

Keep the legacy chapter routes, including the old rules page. Its fragment handling must take an old link such as `rules.html#chained-assignment` to the specific new rule page. Validate the destination's rule ID as well as its HTTP status.

**Automated verification.** From a clean checkout run `mise run docs`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo run --quiet -- check samples/good.py --format text` and `jactionlint .github/workflows/docs.yml`. Run the existing CI hooks. Against the built site under `/xpd/`, follow the baseline's six chapter aliases, 20 named cross-chapter targets and all 426 published rule targets, including fragment handling in a browser. Check new catalog links as well. Confirm that a pull request builds without deploying and that the production workflow serves the verified output.

**Manual verification.** On the published site, open an old editor link, read the rule, browse its pack and search for a command. Compare the result with the candidate build and verify the quick start remains reachable from the package README.

**Progress on 19 November 2026.** `mise run docs` builds the Starlight site into `site/`, and the `docs:starlight` task, mdBook, mdbook-mermaid, `book.toml`, `docs/` and the generated Mermaid scripts are gone. The `docs-preview` daemon serves the same build. xpd links each rule and package check to `https://bjoernaagaard.github.io/xpd/rules/<pack>/<id>/`, in SARIF and GitHub output, LSP diagnostics and hover, and the catalog, and `xpd/unused-suppression` now has its address too.

`website/src/legacy.mjs` maps each old address to its new page. `rules.html`, `commands.html`, `claude-code.html`, `mcp.html`, `code-mode.html`, `lsp.html`, `claude-code/index.html`, `code-mode/index.html` and `lsp/index.html` are small pages whose script reads the fragment and opens its new page. An unknown fragment opens the chapter's new page, and without JavaScript the page refreshes to it. The old `commands/`, `mcp/` and `rules/` folder addresses are pages of the new site. Each old heading of a chapter has an entry, and the rule entries come from the generated rule pages, so a new rule gets one with no edit.

`.github/workflows/docs.yml` builds on pull requests and on pushes to main that touch the site, the rules, the Rust and Python sources, the indexes, the skills, the lockfiles, `mise.toml` or the workflow, and only main deploys. mise-action moved from 5.2.6 to 5.2.10, and the other actions were already on their newest tags. The Release workflow now ignores `website/**` in place of `docs/**` and `book.toml`. The changes went out as [pull request 14](https://github.com/bjoernaagaard/xpd/pull/14), with commits 11f81cc and 6b96864 on main.

- [x] From a clean clone of the branch, `mise run docs` built 497 pages and 506 HTML files, and `cargo fmt --check`, clippy with `-D warnings`, `cargo run --quiet -- check samples/good.py --format text` and `jactionlint .github/workflows/docs.yml` exited 0. The lefthook pre-commit hooks passed on the commit, and `xpd find --check` passed 53 of 53 after `--write-vectors` embedded the one rule whose text changed. 48,206 internal links of the clean build resolve.
- [x] `xpd rules --builtin --format json` gives 427 records, each with the address of its rule page.
- [x] In headless Chrome for Testing 158 against the site under `/xpd/`, locally and then live, all 426 baseline rule links of `rules.html#<id>` opened the page whose heading is the rule's pack and id. The 427 new catalog links, the 20 named cross-chapter targets with their headings, the 6 chapter aliases and the 7 chapter pages opened their new pages. On the live site, the first pass timed out once while one rule page loaded, and the same link passed 3 of 3 runs again.
- [x] On pull request 14, the Documentation build passed in 3 minutes 49 seconds, its deploy job was skipped, and the CI lefthook job passed. On main, run 44544972720 built and deployed at 14:38 UTC, and the Pages API reports the `workflow` build type. Five live pages, including `rules.html` and `guides/mcp/`, are byte for byte equal to the clean build.

The agent also ran the manual steps in headless Chrome against the live site. `rules.html#chained-assignment` opened `rules/pandas/chained-assignment/`, with the lead, the diff and the unsafe fix, and its pack link opened the pandas pack, which has 134 rules and fixes for 81. Ctrl+K, `xpd outline`, two Tabs and Enter opened `commands/outline/`. Every site link in the README on main answers 200, with its heading where it names one, and its Get started link is the quick start. The live guides matched the local build at both widths in both themes.

- [x] On the published site, open an old editor link, read the rule, browse its pack and search for a command.
- [x] Compare the result with the candidate build, and verify the quick start remains reachable from the package README.

## Open questions

None. Resolve exact dependency versions against the official releases during implementation. The research records the baseline and the settled design records the platform and page structure. [Research](03-research-xpd-docs.md)
