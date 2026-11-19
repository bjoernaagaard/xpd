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
- [ ] 2. Give every rule a page and share its explanation with installed tools.
- [ ] 3. Generate command and MCP references from the running tools.
- [ ] 4. Rewrite the public guides and package README.
- [ ] 5. Verify existing links and switch GitHub Pages to Starlight.

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

## 2. Give every rule a page and share its explanation with installed tools

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

The agent also ran the manual steps in headless Chrome for Testing 158 against the `docs-preview` daemon. From the rules overview, the pandas link and then the chained-assignment link opened the rule, which showed its lead, the diff and the unsafe fix command, and its pack link returned to the pandas pack. `pandas/merge-without-validate` showed "none, edit by hand" and the sentence on making the change by hand. Ctrl+K searches for `chained-assignment`, `pandas/chained-assignment`, `chained assignment` and `merge validate` each listed the rule page first. At 390 pixels wide the rule and pack pages did not scroll sideways in light and dark themes. These results await the user's confirmation.

- [ ] Open pandas/chained-assignment from the candidate catalog and follow the link to the pandas pack.
- [ ] Check a rule with no automatic fix.

## 3. Generate command and MCP references from the running tools

**Change outline.** Use the CLI command definitions behind [src/cli.rs](../../../src/cli.rs) and the registrations in [the MCP server](../../../python/xpd/mcp/server.py) for names, nested commands, flags, arguments, defaults and tool signatures. Keep explanations of when to use a command, worked examples and exit behavior in authored prose. Cover MCP prompts and resources alongside its tools. Make generated fields part of the docs build so an option change appears on the next build.

**Automated verification.** Walk the built command tree and compare its public options with the rendered reference, including subcommands. Run `cargo run --quiet -- outline --help` and `cargo run --quiet -- run --help`. Verify that the appropriate pages include `--follow`, `--globs` and `--debug-query`. Start xpd's MCP server and compare its live tool, resource, resource template and prompt listings with the reference. Use the current inventories during verification. The research baseline was 19 top-level commands and 21 registered tools.

**Manual verification.** Search for a flag and a tool name. From each result, check that a reader can identify the owning command or tool and see how to supply the argument.

## 4. Rewrite the public guides and package README

**Change outline.** Use the seven chapters in [the existing book](../../../docs/SUMMARY.md) as the coverage inventory. Rewrite the README used on GitHub and PyPI, installation, commands, rules and rule authoring, Claude Code, MCP setup, code mode and language server guides. Link guides to generated facts rather than copying counts or option tables. In each workflow, lead with what the reader can do, explain prerequisites and show the result. Use a diagram for interactions and a diff for changes to code. Keep examples readable wherever the package README is rendered.

**Automated verification.** Run the documented local commands on copies of the examples. For the full bad.py baseline, run check, then safe fixes, then explicitly enabled unsafe fixes. Compare with the observed five findings, zero safe edits and two unsafe edits. Drive each integration through its normal entry point, verify the displayed response or output, and report any example requiring unavailable external access separately. Build the complete site and check internal links, images and diagrams.

**Manual verification.** Start at the README and follow installation through the first fix without a checkout. In the site, navigate to rule authoring, connect an editor and locate an MCP tool. Check keyboard focus, search results, code readability, diagram labels and horizontal scrolling on desktop and at phone width in each supported theme.

## 5. Verify existing links and switch GitHub Pages to Starlight

**Change outline.** Make the existing `mise run docs` build the complete Starlight site into `site/`. Update [Pages publishing](../../../.github/workflows/docs.yml) to build on pull requests and deploy verified main builds. Include the site sources, dependency locks, rule text, CLI and MCP definitions, and other build inputs in its triggers. Update compatible workflow dependencies. Retire mdBook configuration and assets once their consumers have moved. Publish the new site before releasing binaries that emit the new URLs.

Keep the legacy chapter routes, including the old rules page. Its fragment handling must take an old link such as `rules.html#chained-assignment` to the specific new rule page. Validate the destination's rule ID as well as its HTTP status.

**Automated verification.** From a clean checkout run `mise run docs`, `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo run --quiet -- check samples/good.py --format text` and `jactionlint .github/workflows/docs.yml`. Run the existing CI hooks. Against the built site under `/xpd/`, follow the baseline's six chapter aliases, 20 named cross-chapter targets and all 426 published rule targets, including fragment handling in a browser. Check new catalog links as well. Confirm that a pull request builds without deploying and that the production workflow serves the verified output.

**Manual verification.** On the published site, open an old editor link, read the rule, browse its pack and search for a command. Compare the result with the candidate build and verify the quick start remains reachable from the package README.

## Open questions

None. Resolve exact dependency versions against the official releases during implementation. The research records the baseline and the settled design records the platform and page structure. [Research](03-research-xpd-docs.md)
