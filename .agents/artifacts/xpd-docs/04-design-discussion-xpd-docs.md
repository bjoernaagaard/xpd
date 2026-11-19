---
type: design-discussion
flow: design-discussion
status: resolved
repos:
  - identifier: bjoernaagaard/xpd
    sha: 2f7d56539a80952d2bae4d8048fe559247006867
---

# How readers will learn xpd and look up a finding

I propose rewriting the public pages around two tasks. A new user should be able to install xpd and fix a finding in a file they can copy from the page. A returning user should be able to open a rule from an editor or search for a command option and go straight to its reference.

Today the quick start uses a sample from the checkout. The command reference has a heading for every command, but its outline section omits options including `--follow`. The rules page lists 124 pandas checks while xpd reports 134. Its explanations also ship inside xpd. [Research and supporting checks](03-research-xpd-docs.md)

## Guides teach tasks and references describe what the installed tools accept

Rewrite the README used on GitHub and PyPI, the quick start, the command and rule references, and the guides to Claude Code, MCP, code mode and the language server. Include rule authoring. Each guide should lead with the action, show copyable input and show the observed result. Use a diagram where the order of calls explains what happens.

For example, the chained assignment rule should open with the consequence and the fix. “Under pandas copy-on-write, assigning through a column can leave the frame unchanged. Assign to the frame with `.loc`.” The edit belongs beside that explanation.

```diff
- df["a"][i] = row["a"] * 2
+ df.loc[i, "a"] = row["a"] * 2
```

The reference should identify this as an unsafe automatic fix. The user chooses whether to apply it. The quick start should show the result of the safe command before introducing that choice.

## The site and installed help should use the same rule text

Proposed build inputs and consumers:

```text
Rule definitions and authored Markdown keyed by pack and rule ID
    Cargo embeds the definitions and Markdown
        CLI catalog, editor hover and MCP resources read the same entry
        Starlight renders one page per rule and a browse page for each pack
CLI command definitions and MCP registrations
    The documentation build derives options, defaults, tool signatures and counts
Authored guides and sample files
    Starlight renders the guides with verified examples
    Pagefind indexes the built pages for search
```

Keep the input to Cargo independent of generated site files. A contributor using xpd's rule authoring command should create the explanation where Cargo expects to find it. A missing or duplicate explanation should still fail the build. The current patterns are the heading checks in [build.rs](../../../build.rs), the rule writer in [project.rs](../../../src/author/project.rs) and the public entries in [catalog.rs](../../../src/catalog.rs). Handle the catalog's special unused-suppression diagnostic explicitly.

Use Astro and Node to build the Starlight site for GitHub Pages through the existing local command, `mise run docs`. Build the site on pull requests and include changes to its inputs in deployment triggers. Resolve stable tool versions together, pin the compatible set and record the resulting versions. Verify the six old chapter redirects, 426 published rule targets and 20 named cross-chapter links against the built site. Keep rules.html available so the browser can resolve each old fragment to its corresponding rule page. Check that each link reaches the correct rule ID.

Use the current sample-based checks as a starting point. Run the guide commands on a temporary copy, compare the edits, inspect output from the CLI, editor and MCP for the same rule, and follow an old rule link. Check keyboard search, navigation at phone width and diagram readability in each supported theme. [Existing checks](../../../lefthook.yml)

## Resolved design questions

The user chose research followed by a design discussion and requested a complete rewrite alongside the platform upgrade.

### Starlight will publish the site and Pagefind will provide search

The user chose Starlight for searchable guides and reference pages and accepted the Astro and Node build. Updating mdBook would have kept the existing Rust publishing tools. Sphinx offered Python example checks, but its Python reference generator would still need an exporter for xpd's Rust commands and YAML rules. [Platform comparison](03-research-xpd-docs.md)

### Each rule has its own page and links back to its pack

A reader who opens `pandas/chained-assignment` from an editor, search result or shared link lands on the rule's explanation, example and fix. The page links to the pandas pack so the reader can browse related rules. New catalog links point directly to each rule page. Links from earlier xpd releases, such as `rules.html#chained-assignment`, must reach that same page.

A page per pack or a single catalog page would put readers inside a longer list. A dedicated page keeps one finding in view and gives it an address readers can share.
