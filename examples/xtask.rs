//! Repository automation tasks for xpd (`cargo xtask ...`).

#[expect(
    dead_code,
    reason = "xtask reads the workflow registry to prevent duplicate slugs"
)]
#[path = "../src/ledger/workflow_registry.rs"]
mod workflow_registry;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};

use workflow_registry::WorkflowId;

/// Repository automation commands for xpd.
#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Repository automation tasks for xpd.")]
struct XtaskCli {
    #[command(subcommand)]
    command: XtaskCommand,
}

#[derive(Debug, Subcommand)]
enum XtaskCommand {
    /// Scaffold a new recorded ledger workflow across the registry, CLI, rules, skill, and docs.
    NewWorkflow {
        /// Lowercase kebab-case workflow slug, such as `schema-review`.
        slug: String,
        /// One-sentence summary of what the workflow audits.
        #[arg(long)]
        about: String,
    },
}

fn main() -> Result<()> {
    let cli = XtaskCli::parse();
    match cli.command {
        XtaskCommand::NewWorkflow { slug, about } => scaffold_workflow(&slug, &about),
    }
}

fn scaffold_workflow(slug: &str, about: &str) -> Result<()> {
    validate_slug(slug)?;
    let summary = normalize_summary(about)?;
    let root = repo_root()?;
    let pascal = kebab_to_pascal(slug);
    let snake = slug.replace('-', "_");

    let module_path = root.join(format!("src/{snake}.rs"));
    let rules_path = root.join(format!("rules/xpd/{slug}.yml"));
    let skill_path = root.join(format!("skills/xpd-{slug}/SKILL.md"));
    let doc_path = root.join(format!("website/reference/commands/{slug}.md"));
    let doc_dir = root.join(format!("website/reference/commands/{slug}"));

    for existing in [&module_path, &rules_path, &skill_path, &doc_path] {
        ensure!(!existing.exists(), "{} already exists", existing.display());
    }

    let registry_path = root.join("src/ledger/workflow_registry.rs");
    let worker_path = root.join("src/ledger/worker.rs");
    let lib_path = root.join("src/lib.rs");
    let cli_path = root.join("src/cli.rs");

    update_workflow_registry(&registry_path, slug, &pascal)?;
    update_worker(&worker_path, &pascal)?;
    update_lib_rs(&lib_path, &snake)?;
    update_cli_rs(&cli_path, &pascal, &snake, &summary)?;

    write_file(
        &module_path,
        &render_workflow_module(slug, &pascal, &summary),
    )?;
    write_file(&rules_path, &render_rules_yaml(slug))?;
    write_file(&skill_path, &render_skill_md(slug, &summary))?;
    write_file(&doc_path, &render_command_doc(slug, &summary))?;
    write_subcommand_docs(&doc_dir, slug)?;

    format_rust_files(&[
        &registry_path,
        &worker_path,
        &lib_path,
        &cli_path,
        &module_path,
    ])?;

    println!("Scaffolded workflow `{slug}` ({pascal}):");
    println!("  src/{snake}.rs");
    println!("  rules/xpd/{slug}.yml");
    println!("  skills/xpd-{slug}/SKILL.md");
    println!("  website/reference/commands/{slug}.md");
    Ok(())
}

fn validate_slug(slug: &str) -> Result<()> {
    ensure!(!slug.is_empty(), "workflow slug must not be empty");
    let valid_start = slug.as_bytes()[0].is_ascii_lowercase();
    let valid_end = slug.as_bytes()[slug.len() - 1].is_ascii_alphanumeric();
    let valid_chars = slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    ensure!(
        valid_start && valid_end && valid_chars && !slug.contains("--"),
        "workflow slug `{slug}` must be lowercase kebab-case ([a-z][a-z0-9-]*)"
    );
    ensure!(
        !WorkflowId::ALL.iter().any(|id| id.as_str() == slug),
        "workflow `{slug}` is already registered in WorkflowId"
    );
    Ok(())
}

fn normalize_summary(about: &str) -> Result<String> {
    let trimmed = about.trim();
    ensure!(!trimmed.is_empty(), "--about summary must not be empty");
    let stripped = trimmed.trim_end_matches(['.', '!', '?']).trim();
    ensure!(!stripped.is_empty(), "--about summary must contain words");
    Ok(format!("{stripped}."))
}

fn repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("cannot determine current directory")?;
    ensure!(
        cwd.join("Cargo.toml").is_file() && cwd.join("src/ledger/workflow_registry.rs").is_file(),
        "run `cargo xtask` from the xpd repository root"
    );
    Ok(cwd)
}

fn kebab_to_pascal(slug: &str) -> String {
    let mut out = String::with_capacity(slug.len());
    for part in slug.split('-') {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars);
        }
    }
    out
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create directory {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
}

fn replace_once(path: &Path, target: &str, replacement: &str) -> Result<()> {
    let source =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let count = source.matches(target).count();
    ensure!(
        count == 1,
        "expected 1 anchor match in {}, found {count} for `{target}`",
        path.display()
    );
    let updated = source.replacen(target, replacement, 1);
    fs::write(path, updated).with_context(|| format!("cannot write {}", path.display()))
}

fn update_workflow_registry(path: &Path, slug: &str, pascal: &str) -> Result<()> {
    replace_once(
        path,
        "    Untangle,\n}",
        &format!("    Untangle,\n    {pascal},\n}}"),
    )?;
    replace_once(
        path,
        "        Self::Untangle,\n    ];",
        &format!("        Self::Untangle,\n        Self::{pascal},\n    ];"),
    )?;
    replace_once(
        path,
        "            Self::Untangle => \"untangle\",\n        }",
        &format!(
            "            Self::Untangle => \"untangle\",\n            Self::{pascal} => \"{slug}\",\n        }}"
        ),
    )?;
    replace_once(
        path,
        "            Self::Discover | Self::Untangle => base,",
        &format!("            Self::Discover | Self::Untangle | Self::{pascal} => base,"),
    )?;
    Ok(())
}

fn update_worker(path: &Path, pascal: &str) -> Result<()> {
    replace_once(
        path,
        "        | WorkflowId::Untangle => Ok(()),",
        &format!("        | WorkflowId::Untangle\n        | WorkflowId::{pascal} => Ok(()),"),
    )
}

fn update_lib_rs(path: &Path, snake: &str) -> Result<()> {
    let source =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let decl = format!("mod {snake};");
    if source.lines().any(|line| line.trim() == decl) {
        bail!("{decl} already exists in {}", path.display());
    }
    let mut lines: Vec<String> = source.lines().map(str::to_owned).collect();
    let mut insert_at = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let mod_name = trimmed
            .strip_prefix("mod ")
            .or_else(|| trimmed.strip_prefix("pub mod "))
            .and_then(|rest| rest.strip_suffix(';'));
        if let Some(existing) = mod_name {
            if snake < existing {
                insert_at = Some(idx);
                break;
            }
            insert_at = Some(idx + 1);
        }
    }
    let Some(pos) = insert_at else {
        bail!("cannot locate module declarations in {}", path.display());
    };
    lines.insert(pos, decl);
    let mut out = lines.join("\n");
    out.push('\n');
    fs::write(path, out).with_context(|| format!("cannot write {}", path.display()))
}

fn update_cli_rs(path: &Path, pascal: &str, snake: &str, summary: &str) -> Result<()> {
    replace_once(
        path,
        "    Discover(crate::discover::DiscoverArgs),",
        &format!(
            "    Discover(crate::discover::DiscoverArgs),\n    /// {summary}\n    {pascal}(crate::{snake}::{pascal}Args),"
        ),
    )?;
    replace_once(
        path,
        "        Cmd::Discover(args) => command_result(args.run(default_format)),",
        &format!(
            "        Cmd::Discover(args) => command_result(args.run(default_format)),\n        Cmd::{pascal}(args) => command_result(args.run(default_format)),"
        ),
    )
}

fn render_workflow_module(slug: &str, pascal: &str, summary: &str) -> String {
    format!(
        r#"//! `xpd {slug}`: {summary}

use std::collections::HashMap;
use std::path::{{Path, PathBuf}};
use std::process::ExitCode;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use crate::ledger::gix_source::Listing;
use crate::ledger::{{
    Ledger, LedgerFinding, LedgerKind, QuestionCatalog, Select, SurveyOutput, Tree, WorkflowId,
}};
use crate::output::Format;

/// Grouping of `xpd {slug}` findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum {pascal}Group {{
    Review,
}}

/// Finding kinds reported by `xpd {slug}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum {pascal}Kind {{
    /// Placeholder finding kind to replace with domain-specific checks.
    ReviewItem,
}}

impl LedgerKind for {pascal}Kind {{
    type Group = {pascal}Group;

    const GOALS: &'static [crate::ledger::Goal] = &[crate::ledger::Goal::Design];

    const WORKFLOW: WorkflowId = WorkflowId::{pascal};

    const GROUPS: &'static [{pascal}Group] = &[{pascal}Group::Review];

    fn group(self) -> {pascal}Group {{
        match self {{
            Self::ReviewItem => {pascal}Group::Review,
        }}
    }}

    fn heading(group: {pascal}Group) -> &'static str {{
        match group {{
            {pascal}Group::Review => "Review findings",
        }}
    }}
}}

/// Summary of what a `xpd {slug}` survey read.
#[derive(Debug, Clone, Copy, Serialize)]
struct {pascal}Read {{
    files: usize,
}}

/// {summary}
#[derive(Debug, clap::Args)]
pub(crate) struct {pascal}Args {{
    #[command(flatten)]
    select: Select<{pascal}Kind>,
    /// Print the facts and review questions for `xpd {slug}`.
    #[arg(long)]
    questions: bool,
}}

impl {pascal}Args {{
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {{
        let catalog = QuestionCatalog::load(include_str!("../rules/xpd/{slug}.yml"))?;
        if self.questions {{
            return catalog.print(self.select.format_override().unwrap_or(default));
        }}
        let survey = |paths: &[PathBuf],
                      files: Tree<'_>,
                      top: Option<&Path>,
                      commit: &str,
                      _: Option<&str>,
                      _| {{
            Ok(survey_files(
                files,
                &Listing::of(paths, files, top, commit)?,
                &catalog,
            ))
        }};
        self.select.print_with(default, None, survey, format_header)
    }}
}}

fn format_header(ledger: &Ledger<{pascal}Kind, {pascal}Read>) -> String {{
    format!(
        "xpd {slug}: {{}} in {{}} files\n",
        ledger.count(),
        ledger.read.files
    )
}}

fn survey_files(
    tree: Tree<'_>,
    listing: &Listing,
    catalog: &QuestionCatalog,
) -> SurveyOutput<{pascal}Kind, {pascal}Read> {{
    let mut tracked = HashMap::new();
    for (path, _) in &listing.files {{
        if let Some(absolute) = listing.absolute(path) {{
            let shown = tree.shown(path);
            let rel = tree.tracked(&absolute).unwrap_or_else(|| shown.clone());
            tracked.insert(shown, rel);
        }}
    }}
    let findings: Vec<LedgerFinding<{pascal}Kind>> = Vec::new();
    let read = {pascal}Read {{
        files: listing.files.len(),
    }};
    catalog.modules(read, findings, |path| tracked.get(path).cloned())
}}
"#
    )
}

fn render_rules_yaml(slug: &str) -> String {
    format!(
        r#"facts:
  - id: check-result
    text: "Whether the project checks pass."
    how: "Each check command runs at start and at each gate."
questions:
  - id: {slug}-scope
    text: "Does each surveyed module satisfy the {slug} checks and invariants?"
    look: "Inspect each surveyed module and record a verdict with evidence."
"#
    )
}

fn render_skill_md(slug: &str, summary: &str) -> String {
    format!(
        r#"---
name: xpd-{slug}
description: {summary} Run xpd {slug} to inspect findings and gate fixes in a recorded ledger.
---

# Review code with xpd {slug}

{summary} Run `xpd {slug} --questions` to read the recorded facts and checklist questions.

## Recorded run

1. **Start**: run `xpd {slug} start . --check "<project-check>"` from a clean commit.
2. **List**: run `xpd {slug} . --format json` to read finding IDs and checklist item IDs.
3. **Plan**: write `plan.json` with ordered `fixes`, `accepted` findings, and `answered` checklist verdicts, then run `xpd {slug} plan plan.json`.
4. **Approve**: run `xpd {slug} approve <fix-id>` after approving any interface change.
5. **Gate**: commit one fix and run `xpd {slug} gate <fix-id>`.
6. **Drop**: restore the current commit and run `xpd {slug} drop <fix-id> --reason "<why>"` to abandon a fix.
7. **Status and finish**: run `xpd {slug} status` to inspect progress and `xpd {slug} finish` once every fix and checklist item is resolved.
"#
    )
}

fn render_command_doc(slug: &str, summary: &str) -> String {
    format!(
        r#"Run `xpd {slug}` to run the recorded workflow. {summary}

Run `xpd {slug} --questions` to read the facts and checklist questions.

## Recorded run

1. **Start**: record the current commit, paths, and project checks from a clean tracked tree with `xpd {slug} start . --check "your-project-check"`.
2. **List**: read stable finding IDs and checklist item IDs with `xpd {slug} . --format json`.
3. **Plan**: record ordered fixes, accepted findings, and checklist verdicts with reasons using `xpd {slug} plan plan.json`.
4. **Approve**: record approval for an interface change with `xpd {slug} approve fix-id`.
5. **Gate**: commit one fix and run `xpd {slug} gate fix-id`.
6. **Drop**: return to the current commit and record a reason with `xpd {slug} drop fix-id --reason "reason"`.
7. **Status**: show the current run, recorded answers, and the next stage with `xpd {slug} status`.
8. **Finish**: compare against the starting commit with `xpd {slug} finish`.

## Example

```console
$ xpd {slug} --questions --format json | jq -c '.facts | length >= 1'
true
```

## Exit status

A successful list or recorded stage exits 0. A rejected gate or a finish blocked by findings or checklist items exits 1. Invalid input exits 2.
"#
    )
}

fn write_subcommand_docs(dir: &Path, slug: &str) -> Result<()> {
    let subcommands = [
        (
            "start",
            "Record the current commit, paths and project checks. Start from a clean tracked tree.",
            format!("xpd {slug} start . --check \"your-project-check\""),
            format!("Usage: xpd {slug} start [OPTIONS] [PATHS]..."),
        ),
        (
            "plan",
            "Record an ordered list of fixes and any findings or checklist answers that have a reason.",
            format!("xpd {slug} plan plan.json"),
            format!("Usage: xpd {slug} plan [OPTIONS] <FILE>"),
        ),
        (
            "approve",
            "Record approval for a fix that changes what callers or users see.",
            format!("xpd {slug} approve fix-id"),
            format!("Usage: xpd {slug} approve [OPTIONS] <FIX>"),
        ),
        (
            "gate",
            "Commit the fix and run its gate. The gate checks the project, compares findings and records its verdict.",
            format!("xpd {slug} gate fix-id"),
            format!("Usage: xpd {slug} gate [OPTIONS] <FIX>"),
        ),
        (
            "drop",
            "Return to the current commit and record a reason for dropping the fix.",
            format!("xpd {slug} drop fix-id --reason \"reason\""),
            format!("Usage: xpd {slug} drop [OPTIONS] --reason <REASON> <FIX>"),
        ),
        (
            "status",
            "Show the current run, its recorded answers and the next stage.",
            format!("xpd {slug} status"),
            format!("Usage: xpd {slug} status [OPTIONS]"),
        ),
        (
            "finish",
            "Compare against the starting commit. An open fix, a new finding without an accepted reason or an unanswered checklist item causes a failure.",
            format!("xpd {slug} finish"),
            format!("Usage: xpd {slug} finish [OPTIONS]"),
        ),
    ];
    for (name, intro, cmd, usage) in subcommands {
        let content = format!(
            "{intro}\n\nRun `{cmd}`.\n\n## Example\n\n```console\n$ xpd {slug} {name} --help | sed -n '/^Usage:/p'\n{usage}\n```\n"
        );
        write_file(&dir.join(format!("{name}.md")), &content)?;
    }
    Ok(())
}

fn format_rust_files(files: &[&PathBuf]) -> Result<()> {
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .args(files)
        .status()
        .context("cannot run rustfmt on scaffolded Rust files")?;
    ensure!(status.success(), "rustfmt exited with {status}");
    Ok(())
}
