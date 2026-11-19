---
type: design-discussion
flow: design-discussion
status: open
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
        Site build reads the built-in catalog and renders the rule reference
CLI command definitions and MCP registrations
    Site build derives options, defaults, tool signatures and counts
Authored guides and sample files
    Site build combines instructions with verified examples
```

Keep the input to Cargo independent of generated site files. A contributor using xpd's rule authoring command should create the explanation where Cargo expects to find it. A missing or duplicate explanation should still fail the build. The current patterns are the heading checks in [build.rs](../../../build.rs), the rule writer in [project.rs](../../../src/author/project.rs) and the public entries in [catalog.rs](../../../src/catalog.rs). Handle the catalog's special unused-suppression diagnostic explicitly.

Keep GitHub Pages and the existing local build command. Build the site on pull requests and include changes to its inputs in deployment triggers. Resolve stable tool versions together, pin the compatible set and record the resulting versions. Verify the six old chapter redirects, 426 published rule targets and 20 named cross-chapter links against the built site. URL fragments stay in the browser, so preserving an old rule link requires keeping its anchor or resolving the fragment on the old page.

Use the current sample-based checks as a starting point. Run the guide commands on a temporary copy, compare the edits, inspect output from the CLI, editor and MCP for the same rule, and follow an old rule link. Check keyboard search, navigation at phone width and diagram readability in each supported theme. [Existing checks](../../../lefthook.yml)

## Design questions

### Which publishing platform should maintainers work with?

| Option | What it offers this project | Cost |
| --- | --- | --- |
| Starlight, recommended | Pagefind search, navigation for a larger reference, highlighted examples and integrations for diagrams and link checks | Adds an Astro and Node build. Existing URLs and embedded rule text need explicit handling. |
| Updated mdBook | Keeps the working Rust-based build, keyboard search and Mermaid pages | Building a more detailed catalog and checking xpd examples remain project work. |
| Sphinx | Python example checking, cross-references and Python API documentation | The command definitions are Rust and the rules are YAML. They still need an exporter. |

Starlight is my recommendation for a full platform replacement with individually searchable reference pages. Updating mdBook is the smaller choice if keeping one Rust-based publishing stack matters more. The [release and feature comparison](03-research-xpd-docs.md) covers available versions. No migration has been tested.

### How should readers browse and link to the rule catalog?

I recommend one address per rule, grouped by pack, with the same Markdown available to the installed tools. A reader can share or search for one rule directly. Existing links need a route from each old fragment to its rule. One page per pack would reduce the number of pages while retaining longer lists. A single catalog page would keep today's anchor scheme and its long scroll. This choice concerns the public addresses and navigation. The build can derive counts under any of the three designs.

## Resolved design questions

The user chose research followed by a design discussion and requested a complete rewrite alongside the platform upgrade. Platform and catalog choices await discussion.
