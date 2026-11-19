---
type: research-questions
repos:
  - identifier: bjoernaagaard/xpd
    sha: b8319de2d2871945befb2f2d3734928055b85282
flow: design-discussion
---

# Research questions for the xpd documentation

1. Where do readers first encounter xpd, and which tasks can they finish from each entry point? Trace the README, package descriptions and the seven chapters in `docs/SUMMARY.md`. Record public URLs and anchors that other pages link to.
2. Which sources define the commands, options, rules and integrations that readers can use? Compare the running CLI and the rule definitions in `rules/` with the public references. Identify generated pages, omissions and places where the same facts are maintained twice.
3. What does a reader see when they follow a published example from installation through their first finding and fix? Run representative examples, capture their input and output, and check where readers get the action, explanation and visual evidence.
4. How does a change become a published page? Trace `book.toml`, the docs task in `mise.toml` and `.github/workflows/docs.yml`. Identify generated files, build checks, deployment triggers and changes that can leave published references stale.
5. How do readers navigate and search the current site on desktop, mobile and with a keyboard? Record its theme, colors, type, spacing, borders and shadows. Check diagrams and code samples in each available color scheme.
6. Which stable versions of the documentation tools and dependencies are installed, pinned and available upstream? Use dated official releases to compare the current stack with maintained alternatives. Identify which features each offers for search, reference generation, examples, diagrams, accessibility and link checks, and which the site uses today.
