//! Checks the decisions that `xpd doctor` ships against upstream records of the pinned Ruff and
//! ty releases, which the source directory holds, and against the reviewed inventories.
//!
//! ```text
//! cargo run --example verify_doctor_catalog -- <source-directory> [--inventories <directory>]
//! ```
//!
//! For every surface it prints the identifiers expected from upstream and missing from the
//! package, the packaged ones upstream lacks, and the duplicates on either side before they are
//! counted once. It also prints links to no record, missing metadata, and decisions or reasons
//! that differ from the inventories. It exits 1 when any list is not empty.
//!
//! The source directory is made from the pinned releases, as follows, with `ruff` 0.17.4 and
//! `ty` 0.0.106 on `PATH`:
//!
//! ```text
//! ruff/rules.json            ruff rule --all --output-format json
//! ruff/config.json           ruff config --output-format json
//! ruff/defaults-stable.txt   ruff check --isolated --no-preview --show-settings empty.py
//! ruff/defaults-preview.txt  ruff check --isolated --preview --show-settings empty.py
//! ruff/help/<command>.txt    ruff help, and ruff <command> --help for analyze, analyze graph,
//!                            check, clean, config, format, linter, rule, server and version
//! ruff/ruff.schema.json      ruff.schema.json at the tag 0.17.4
//! ruff/rule_redirects.rs     crates/ruff_linter/src/rule_redirects.rs at the tag 0.17.4
//! ruff/editor-settings.md    docs/editors/settings.md at the tag 0.17.4
//! ty/rules.json              ty explain rule --output-format json
//! ty/help/<command>.txt      ty help, and ty <command> --help for check, explain,
//!                            explain rule, server and version
//! ty/ty.schema.json          ty.schema.json of astral-sh/ruff at b6d8e21241a2f24e11c7d9b11cd26d629a866403
//! ty/diagnostic.rs           crates/ty_python_semantic/src/types/diagnostic.rs at that commit
//! ty/options.rs              crates/ty_project/src/metadata/options.rs at that commit
//! ty/rules.md                docs/reference/rules.md of astral-sh/ty at the tag 0.0.106
//! ty/editor-settings.md      docs/reference/editor-settings.md at the tag 0.0.106
//! ty/environment.md          docs/reference/environment.md at the tag 0.0.106
//! ```

#[path = "../src/doctor/catalog.rs"]
#[expect(
    dead_code,
    reason = "the verifier reads the records, not the parts doctor evaluates"
)]
mod catalog;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde_json::Value;

use catalog::{Decision, Key, Kind, Record, Surface, Tool};

/// The release whose live index lists the rules newer than the pinned Ruff.
const NEWER_RUFF: &str = "0.17.5";

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("verify_doctor_catalog: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let mut args = std::env::args_os().skip(1);
    let mut source = None;
    let mut inventories = None;
    while let Some(arg) = args.next() {
        if arg == "--inventories" {
            inventories = Some(PathBuf::from(
                args.next().context("--inventories needs a path")?,
            ));
        } else {
            source = Some(PathBuf::from(arg));
        }
    }
    let source = source
        .context("usage: verify_doctor_catalog <source-directory> [--inventories <directory>]")?;
    let records = catalog::parse_lines(catalog::DECISIONS)?;
    let mut report = Report::default();
    match catalog::Catalog::new(records.clone()) {
        Ok(catalog) => report.links(&catalog),
        Err(error) => report
            .problems
            .push(format!("the catalog does not load: {error:#}")),
    }
    report.evidence(&records);
    compare_surfaces(&source, &records, &mut report)?;
    report.newer(&source, &records)?;
    if let Some(dir) = inventories {
        report.inventories(&dir, &records)?;
    }
    Ok(report.print())
}

#[derive(Default)]
struct Report {
    /// One line per surface with its counts and its set differences.
    surfaces: Vec<String>,
    failed: bool,
    problems: Vec<String>,
    changed: Vec<String>,
    notes: Vec<String>,
}

impl Report {
    /// Records the expected and the implemented identifiers of one surface.
    fn compare(&mut self, name: &str, expected: &[String], implemented: &[String]) {
        let expected_set: BTreeSet<&String> = expected.iter().collect();
        let implemented_set: BTreeSet<&String> = implemented.iter().collect();
        let missing: Vec<_> = expected_set.difference(&implemented_set).collect();
        let extra: Vec<_> = implemented_set.difference(&expected_set).collect();
        let duplicate_expected = duplicates(expected);
        let duplicate_implemented = duplicates(implemented);
        let list = |names: Vec<String>| {
            if names.is_empty() {
                "∅".to_owned()
            } else {
                names.join(", ")
            }
        };
        self.failed |= !missing.is_empty()
            || !extra.is_empty()
            || !duplicate_expected.is_empty()
            || !duplicate_implemented.is_empty();
        self.surfaces.push(format!(
            "{name}: expected {}, implemented {}\n  expected minus implemented: {}\n  implemented minus expected: {}\n  duplicate expected: {}\n  duplicate implemented: {}",
            expected.len(),
            implemented.len(),
            list(missing.into_iter().map(|s| (*s).clone()).collect()),
            list(extra.into_iter().map(|s| (*s).clone()).collect()),
            list(duplicate_expected),
            list(duplicate_implemented),
        ));
    }

    /// Every link resolves, and every record carries the metadata its kind needs.
    fn links(&mut self, catalog: &catalog::Catalog) {
        let selectable: BTreeSet<&str> = catalog
            .ruff_rules()
            .flat_map(|rule| [Some(rule.key.id.as_str()), rule.prefix.as_deref()])
            .flatten()
            .collect();
        for record in catalog.records() {
            if let Some(target) = &record.target {
                let rules: Vec<&Record> = catalog.ruff_rules().collect();
                if !selects_a_rule(&rules, target) && !selectable.contains(target.as_str()) {
                    self.problems
                        .push(format!("unresolved redirect: {} -> {target}", record.key));
                }
            }
            let key = &record.key;
            if let Some(canonical) = &record.canonical
                && catalog.get(canonical).is_none()
            {
                self.problems
                    .push(format!("unresolved link: {key} -> {canonical}"));
            }
            let needs_default = matches!(
                (key.tool, key.surface, key.kind),
                (_, _, Kind::Rule) | (Tool::Ruff, Surface::Project, Kind::Option)
            );
            if needs_default && record.default.is_none() {
                self.problems
                    .push(format!("missing metadata: {key} has no default"));
            }
            if record.key.kind == Kind::Rule && (record.status.is_none() || record.since.is_none())
            {
                self.problems
                    .push(format!("missing metadata: {key} has no status"));
            }
            if key.tool == Tool::Ruff && record.key.kind == Kind::Rule && record.name.is_none() {
                self.problems
                    .push(format!("missing metadata: {key} has no rule name"));
            }
        }
    }

    /// Every trigger on code has an evidence rule of its name, and every evidence rule a trigger.
    fn evidence(&mut self, records: &[Record]) {
        let yaml = include_str!("../src/doctor/evidence.yml");
        let rules: BTreeSet<String> = yaml
            .lines()
            .filter_map(|line| line.strip_prefix("id: "))
            .map(str::to_owned)
            .collect();
        let triggers: BTreeSet<String> = catalog::Trigger::ALL
            .into_iter()
            .filter(|trigger| trigger.in_code())
            .map(catalog::Trigger::name)
            .collect();
        for missing in triggers.difference(&rules) {
            self.problems
                .push(format!("trigger without an evidence rule: {missing}"));
        }
        for unused in rules.difference(&triggers) {
            self.problems
                .push(format!("evidence rule without a trigger: {unused}"));
        }
        for record in records {
            if let Decision::Suggest(suggestion) = &record.decision
                && let catalog::When::If { trigger, .. } = suggestion.when
                && !catalog::Trigger::ALL.contains(&trigger)
            {
                self.problems
                    .push(format!("{} names an unknown trigger", record.key));
            }
        }
    }

    /// The live entries beyond the pinned release stay out of the pinned set, each with the
    /// release it comes from and its decision.
    fn newer(&mut self, source: &Path, records: &[Record]) -> Result<()> {
        let pinned: BTreeMap<String, String> = read_json(&source.join("ruff/rules.json"))?
            .as_array()
            .context("ruff/rules.json is no array")?
            .iter()
            .map(|rule| (ruff_id(rule), str_field(rule, "name")))
            .collect();
        for record in records.iter().filter(|r| r.key.kind == Kind::Newer) {
            let name = record.name.clone().unwrap_or_default();
            let disposition = match pinned.get(&record.key.id) {
                None => "absent from the pinned release".to_owned(),
                Some(pinned_name) if *pinned_name != name => {
                    format!("renamed: the pinned release calls it {pinned_name}")
                }
                Some(_) => {
                    self.problems.push(format!(
                        "{} is recorded as newer but the pinned release has it",
                        record.key
                    ));
                    continue;
                }
            };
            self.notes.push(format!(
                "newer entry {} {name} from {}: {disposition}; decision {}",
                record.key.id,
                record.key.release,
                record.decision.word()
            ));
        }
        Ok(())
    }

    fn print(self) -> bool {
        for surface in &self.surfaces {
            println!("{surface}");
        }
        let sections = [
            ("unresolved links and missing metadata", &self.problems),
            ("changed decisions and reasons", &self.changed),
        ];
        for (title, lines) in sections {
            println!("{title}: {}", if lines.is_empty() { "∅" } else { "" });
            for line in lines {
                println!("  {line}");
            }
        }
        for note in &self.notes {
            println!("{note}");
        }
        let ok = !self.failed && self.problems.is_empty() && self.changed.is_empty();
        println!(
            "{}",
            if ok {
                "catalog verified"
            } else {
                "catalog FAILED"
            }
        );
        ok
    }
}

/// Whether `selector` names a rule's code, or a prefix of codes within one linter.
fn selects_a_rule(rules: &[&Record], selector: &str) -> bool {
    rules.iter().any(|rule| {
        rule.key.id.starts_with(selector)
            && rule
                .prefix
                .as_deref()
                .is_some_and(|prefix| selector.starts_with(prefix))
    })
}

/// The names that occur more than once in `names`, each once.
fn duplicates(names: &[String]) -> Vec<String> {
    let mut seen = BTreeMap::new();
    for name in names {
        *seen.entry(name.clone()).or_insert(0) += 1;
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name)
        .collect()
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_str(&read(path)?).with_context(|| format!("{} is not JSON", path.display()))
}

fn str_field(value: &Value, field: &str) -> String {
    value[field].as_str().unwrap_or_default().to_owned()
}

/// A Ruff rule's code, or its name when it has no code.
fn ruff_id(rule: &Value) -> String {
    match rule["code"].as_str() {
        Some(code) if !code.is_empty() => code.to_owned(),
        _ => str_field(rule, "name"),
    }
}

fn implemented<'a>(
    records: &'a [Record],
    tool: Tool,
    surface: Surface,
    kinds: &'a [Kind],
) -> impl Iterator<Item = &'a Record> {
    records.iter().filter(move |record| {
        record.key.tool == tool && record.key.surface == surface && kinds.contains(&record.key.kind)
    })
}

fn ids<'a>(records: impl Iterator<Item = &'a Record>) -> Vec<String> {
    records.map(|record| record.key.id.clone()).collect()
}

fn compare_surfaces(source: &Path, records: &[Record], report: &mut Report) -> Result<()> {
    ruff_surfaces(source, records, report)?;
    ty_surfaces(source, records, report)
}

/// Each packaged redirect goes where upstream's goes, or upstream's target selects no rule.
fn compare_redirect_targets(
    redirects: &[(String, String)],
    records: &[Record],
    report: &mut Report,
) {
    let rules: Vec<&Record> =
        implemented(records, Tool::Ruff, Surface::Rules, &[Kind::Rule]).collect();
    for (old, new) in redirects {
        let Some(record) = records
            .iter()
            .find(|r| r.key.kind == Kind::Redirect && r.key.id == *old)
        else {
            continue;
        };
        match &record.target {
            Some(target) if target != new => {
                report
                    .problems
                    .push(format!("redirect {old} goes to {new} upstream"));
            }
            Some(_) => {}
            None if selects_a_rule(&rules, new) => {
                report.problems.push(format!(
                    "redirect {old} has no target, but {new} selects a rule"
                ));
            }
            None => report.notes.push(format!(
                "dead redirect {old} -> {new}: the target selects no rule of the release"
            )),
        }
    }
}

fn ruff_surfaces(source: &Path, records: &[Record], report: &mut Report) -> Result<()> {
    ruff_rules(source, records, report)?;
    let redirects = ruff_redirects(&read(&source.join("ruff/rule_redirects.rs"))?);
    report.compare(
        "ruff redirects",
        &redirects
            .iter()
            .map(|(old, _)| old.clone())
            .collect::<Vec<_>>(),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Rules,
            &[Kind::Redirect],
        )),
    );
    compare_redirect_targets(&redirects, records, report);
    let schema = read_json(&source.join("ruff/ruff.schema.json"))?;
    report.compare(
        "ruff project settings (schema paths)",
        &schema_paths(&schema, &BTreeSet::new()),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Project,
            &[Kind::Option, Kind::Schema, Kind::Legacy],
        )),
    );
    let config = read_json(&source.join("ruff/config.json"))?;
    report.compare(
        "ruff documented options (ruff config)",
        &config
            .as_object()
            .context("ruff/config.json is no object")?
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Project,
            &[Kind::Option],
        )),
    );
    report.compare(
        "ruff editor settings",
        &editor_settings(
            &read(&source.join("ruff/editor-settings.md"))?,
            Containers::Skipped,
        ),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Editor,
            &[Kind::Setting],
        )),
    );
    let ruff_help = help_texts(&source.join("ruff/help"))?;
    report.compare(
        "ruff CLI spellings",
        &cli_spellings(&ruff_help),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Cli,
            &[Kind::Option],
        )),
    );
    report.compare(
        "ruff environment keys",
        &env_keys(&ruff_help),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Env,
            &[Kind::Variable],
        )),
    );

    Ok(())
}

fn ty_surfaces(source: &Path, records: &[Record], report: &mut Report) -> Result<()> {
    ty_rules(source, records, report)?;
    let aliases = ty_aliases(&read(&source.join("ty/diagnostic.rs"))?);
    report.compare(
        "ty rule aliases",
        &aliases.keys().cloned().collect::<Vec<_>>(),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Rules,
            &[Kind::Alias],
        )),
    );
    for (alias, canonical) in &aliases {
        let packaged = records
            .iter()
            .find(|r| r.key.tool == Tool::Ty && r.key.kind == Kind::Alias && r.key.id == *alias)
            .and_then(|r| r.canonical.as_ref());
        if packaged.is_some_and(|key| key.id != *canonical) {
            report
                .problems
                .push(format!("ty alias {alias} names {canonical} upstream"));
        }
    }
    report.compare(
        "ty removed rules",
        &removed_rules(&read(&source.join("ty/rules.md"))?),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Rules,
            &[Kind::Removed],
        )),
    );
    let schema = read_json(&source.join("ty/ty.schema.json"))?;
    let alias_keys: BTreeSet<String> = aliases.keys().cloned().collect();
    let mut expected: Vec<String> = schema_paths(&schema, &alias_keys)
        .into_iter()
        .map(|path| format!("tool.ty.{path}"))
        .collect();
    for (canonical, alias) in serde_aliases(&read(&source.join("ty/options.rs"))?) {
        for path in expected.clone() {
            if let Some(parent) = path.strip_suffix(&format!(".{canonical}")) {
                expected.push(format!("{parent}.{alias}"));
            }
        }
    }
    report.compare(
        "ty project settings (schema paths, rule keys and aliases)",
        &expected,
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Project,
            &[Kind::Option, Kind::Alias, Kind::RuleMap, Kind::RuleMapAlias],
        )),
    );
    report.compare(
        "ty editor settings",
        &editor_settings(
            &read(&source.join("ty/editor-settings.md"))?,
            Containers::Counted,
        ),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Editor,
            &[Kind::Setting],
        )),
    );
    report.compare(
        "ty CLI spellings",
        &cli_spellings(&help_texts(&source.join("ty/help"))?),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Cli,
            &[Kind::Option],
        )),
    );
    report.compare(
        "ty environment variables",
        &markdown_headings(&read(&source.join("ty/environment.md"))?),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Env,
            &[Kind::Variable],
        )),
    );
    Ok(())
}

/// Ruff's registry, with each rule's name, status and default selection.
fn ruff_rules(source: &Path, records: &[Record], report: &mut Report) -> Result<()> {
    let upstream = read_json(&source.join("ruff/rules.json"))?;
    let upstream = upstream.as_array().context("ruff/rules.json is no array")?;
    report.compare(
        "ruff rules",
        &upstream.iter().map(ruff_id).collect::<Vec<_>>(),
        &ids(implemented(
            records,
            Tool::Ruff,
            Surface::Rules,
            &[Kind::Rule],
        )),
    );
    let stable = enabled_codes(&read(&source.join("ruff/defaults-stable.txt"))?);
    let preview = enabled_codes(&read(&source.join("ruff/defaults-preview.txt"))?);
    for rule in upstream {
        let id = ruff_id(rule);
        let Some(record) = records
            .iter()
            .find(|r| r.key.tool == Tool::Ruff && r.key.kind == Kind::Rule && r.key.id == id)
        else {
            continue;
        };
        let status = rule["status"]
            .as_object()
            .and_then(|status| status.keys().next())
            .map(|status| status.to_lowercase())
            .unwrap_or_default();
        let on = |set: &BTreeSet<String>| if set.contains(&id) { "on" } else { "off" };
        let default = format!("{} / {}", on(&stable), on(&preview));
        let checks = [
            (
                "name",
                record.name.clone().unwrap_or_default(),
                str_field(rule, "name"),
            ),
            ("status", record.status.clone().unwrap_or_default(), status),
            (
                "default",
                record.default.clone().unwrap_or_default(),
                default,
            ),
        ];
        for (field, packaged, expected) in checks {
            if packaged != expected {
                report.problems.push(format!(
                    "ruff:rules:{id} {field} is {packaged:?}, upstream {expected:?}"
                ));
            }
        }
    }
    Ok(())
}

/// The codes, or the names of rules without a code, in the enabled block of
/// `ruff check --show-settings`.
fn enabled_codes(settings: &str) -> BTreeSet<String> {
    let line = Regex::new(r"^\s+([a-z0-9-]+)(?: \(([A-Z0-9]+)\))?,$").expect("valid pattern");
    settings
        .lines()
        .skip_while(|l| !l.starts_with("linter.rules.enabled = ["))
        .skip(1)
        .take_while(|l| *l != "]")
        .filter_map(|l| {
            let captures = line.captures(l)?;
            Some(
                captures
                    .get(2)
                    .unwrap_or(captures.get(1)?)
                    .as_str()
                    .to_owned(),
            )
        })
        .collect()
}

/// The production redirects, without the branches that only tests compile.
fn ruff_redirects(source: &str) -> Vec<(String, String)> {
    let pair = Regex::new(r#"^\s*\("([^"]+)", "([^"]+)"\),"#).expect("valid pattern");
    let mut redirects = Vec::new();
    let mut test_only = false;
    for line in source.lines() {
        if line.trim_start().starts_with("#[cfg(test)]") && !line.starts_with(' ') {
            break;
        }
        if line.contains("#[cfg(any(feature = \"test-rules\", test))]") {
            test_only = true;
            continue;
        }
        if let Some(captures) = pair.captures(line) {
            if !test_only {
                redirects.push((captures[1].to_owned(), captures[2].to_owned()));
            }
            test_only = false;
        }
    }
    redirects
}

/// Every key path the JSON Schema accepts. Each `$ref` and union branch is followed into named
/// properties, and the items of an array of tables count as the array's own path. A typed
/// `additionalProperties` slot counts once as `<key>`, except a ty rules table, whose free keys
/// are the historical rule names in `rule_aliases`.
fn schema_paths(schema: &Value, rule_aliases: &BTreeSet<String>) -> Vec<String> {
    fn walk(
        schema: &Value,
        node: &Value,
        prefix: &str,
        seen: &BTreeSet<String>,
        aliases: &BTreeSet<String>,
        out: &mut Vec<String>,
    ) {
        if let Some(reference) = node["$ref"].as_str() {
            let name = reference.rsplit('/').next().unwrap_or_default().to_owned();
            if seen.contains(&name) {
                return;
            }
            let definitions = schema
                .get("definitions")
                .or_else(|| schema.get("$defs"))
                .unwrap_or(&Value::Null);
            let mut seen = seen.clone();
            seen.insert(name.clone());
            walk(schema, &definitions[&name], prefix, &seen, aliases, out);
            return;
        }
        for union in ["anyOf", "oneOf", "allOf"] {
            for branch in node[union].as_array().into_iter().flatten() {
                walk(schema, branch, prefix, seen, aliases, out);
            }
        }
        if node["type"] == "array" || node["items"].is_object() {
            walk(schema, &node["items"], prefix, seen, aliases, out);
        }
        for (key, value) in node["properties"].as_object().into_iter().flatten() {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            out.push(path.clone());
            walk(schema, value, &path, seen, aliases, out);
        }
        let slot = &node["additionalProperties"];
        if slot.is_object() && slot.as_object().is_some_and(|slot| !slot.is_empty()) {
            if prefix.ends_with("rules") && !aliases.is_empty() {
                out.extend(aliases.iter().map(|alias| format!("{prefix}.{alias}")));
            } else {
                let path = format!("{prefix}.<key>");
                out.push(path.clone());
                walk(schema, slot, &path, seen, aliases, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(schema, schema, "", &BTreeSet::new(), rule_aliases, &mut out);
    out.sort();
    out.dedup();
    out
}

/// Whether a reference counts a group of settings as a setting of its own.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Containers {
    Counted,
    Skipped,
}

/// The settings a Markdown reference documents, one per `### ` heading in backticks, and one per
/// `## ` heading in backticks when `containers` counts the groups too. A `### ` heading under a
/// `## ` heading in backticks is nested in that setting.
fn editor_settings(markdown: &str, containers: Containers) -> Vec<String> {
    let mut parent: Option<String> = None;
    let mut out = Vec::new();
    for line in markdown.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            parent = backticked(heading);
            if let Some(name) = parent
                .as_ref()
                .filter(|_| containers == Containers::Counted)
            {
                out.push(name.clone());
            }
        } else if let Some(heading) = line.strip_prefix("### ")
            && let Some(name) = backticked(heading)
        {
            out.push(match &parent {
                Some(parent) => format!("{parent}.{name}"),
                None => name,
            });
        }
    }
    out
}

/// Every `### ` heading in backticks.
fn markdown_headings(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| line.strip_prefix("### "))
        .filter_map(backticked)
        .collect()
}

fn backticked(heading: &str) -> Option<String> {
    let rest = heading.strip_prefix('`')?;
    Some(rest[..rest.find('`')?].to_owned())
}

fn help_texts(dir: &Path) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("cannot list {}", dir.display()))? {
        texts.push(read(&entry?.path())?);
    }
    Ok(texts)
}

/// The long option spellings of the help texts, each once, with inverse switches that an
/// option's description names in backticks.
fn cli_spellings(texts: &[String]) -> Vec<String> {
    let defined =
        Regex::new(r"(?m)^\s+(?:-[A-Za-z], )?(--[a-z][a-z0-9-]*)").expect("valid pattern");
    let named = Regex::new(r"`(--[a-z][a-z0-9-]*)`").expect("valid pattern");
    let spellings: BTreeSet<String> = texts
        .iter()
        .flat_map(|text| {
            defined
                .captures_iter(text)
                .chain(named.captures_iter(text))
                .map(|captures| captures[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    spellings.into_iter().collect()
}

fn env_keys(texts: &[String]) -> Vec<String> {
    let env = Regex::new(r"\[env: ([A-Z_]+)=").expect("valid pattern");
    let keys: BTreeSet<String> = texts
        .iter()
        .flat_map(|text| {
            env.captures_iter(text)
                .map(|captures| captures[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    keys.into_iter().collect()
}

fn ty_rules(source: &Path, records: &[Record], report: &mut Report) -> Result<()> {
    let upstream = read_json(&source.join("ty/rules.json"))?;
    let upstream = upstream.as_array().context("ty/rules.json is no array")?;
    report.compare(
        "ty rules",
        &upstream
            .iter()
            .map(|rule| str_field(rule, "name"))
            .collect::<Vec<_>>(),
        &ids(implemented(
            records,
            Tool::Ty,
            Surface::Rules,
            &[Kind::Rule],
        )),
    );
    for rule in upstream {
        let name = str_field(rule, "name");
        let Some(record) = records
            .iter()
            .find(|r| r.key.tool == Tool::Ty && r.key.kind == Kind::Rule && r.key.id == name)
        else {
            continue;
        };
        let checks = [
            (
                "default",
                record.default.clone(),
                str_field(rule, "default_level"),
            ),
            (
                "status",
                record.status.clone(),
                str_field(&rule["status"], "type"),
            ),
            (
                "since",
                record.since.clone(),
                str_field(&rule["status"], "since"),
            ),
        ];
        for (field, packaged, expected) in checks {
            if packaged.as_deref() != Some(expected.as_str()) {
                report.problems.push(format!(
                    "ty:rules:{name} {field} is {packaged:?}, upstream {expected:?}"
                ));
            }
        }
    }
    Ok(())
}

/// The historical rule names that the registry accepts, each with its current rule. A constant
/// such as `NOT_CALLABLE` declares the rule `not-callable`.
fn ty_aliases(source: &str) -> BTreeMap<String, String> {
    let alias =
        Regex::new(r#"register_alias\(\s*LintName::of\("([^"]+)"\),\s*&([A-Z_0-9]+),?\s*\)"#)
            .expect("valid pattern");
    alias
        .captures_iter(source)
        .map(|captures| {
            (
                captures[1].to_owned(),
                captures[2].to_lowercase().replace('_', "-"),
            )
        })
        .collect()
}

fn removed_rules(markdown: &str) -> Vec<String> {
    markdown
        .split("\n## Removed rules\n")
        .nth(1)
        .map(markdown_headings)
        .unwrap_or_default()
}

/// Each serde alias of an option, as `(field, alias)`.
fn serde_aliases(source: &str) -> Vec<(String, String)> {
    let pattern = Regex::new(r#"#\[serde\(alias = "([^"]+)"\)\][^;]*?pub ([a-z_]+):"#)
        .expect("valid pattern");
    pattern
        .captures_iter(source)
        .map(|captures| (captures[2].replace('_', "-"), captures[1].to_owned()))
        .collect()
}

/// One decision row of an inventory.
struct Row {
    key: Key,
    word: String,
    /// The text the record's reason must equal, or `None` when the inventory states only the
    /// decision.
    reason: Option<String>,
}

impl Report {
    /// Compares each packaged decision and reason with the inventory row of the same key.
    fn inventories(&mut self, dir: &Path, records: &[Record]) -> Result<()> {
        let rows = inventory_rows(dir)?;
        let packaged: BTreeMap<&Key, &Record> = records.iter().map(|r| (&r.key, r)).collect();
        let catalog = catalog::Catalog::new(records.to_vec()).ok();
        let mut seen = BTreeSet::new();
        for row in &rows {
            seen.insert(row.key.clone());
            let Some(record) = packaged.get(&row.key) else {
                self.changed
                    .push(format!("{} is in the inventory and not packaged", row.key));
                continue;
            };
            let word = match (&record.decision, &catalog) {
                (Decision::Inherit(_), Some(catalog)) if row.word != "inherit" => catalog
                    .resolve(record)
                    .map_or("unresolved", |parent| parent.decision.word()),
                _ => record.decision.word(),
            };
            if word != row.word {
                self.changed.push(format!(
                    "{} decision: packaged {word}, inventory {}",
                    row.key, row.word
                ));
            }
            if let Some(reason) = &row.reason
                && *reason != record.reason
            {
                self.changed.push(format!(
                    "{} reason: packaged {:?}, inventory {reason:?}",
                    row.key, record.reason
                ));
            }
        }
        for record in records {
            // The external inventories cover Ruff and ty. Pytest has a policy catalog.
            if record.key.tool != Tool::Pytest && !seen.contains(&record.key) {
                self.changed
                    .push(format!("{} is packaged and in no inventory", record.key));
            }
        }
        Ok(())
    }
}

fn decision_word(text: &str) -> Result<&'static str> {
    let text = text.trim_start_matches("**");
    for (prefix, word) in [
        ("Leave", "leave"),
        ("Recommend", "recommend"),
        ("Conditional", "conditional"),
    ] {
        if text.starts_with(prefix) {
            return Ok(word);
        }
    }
    bail!("no decision in {text:?}")
}

fn first_code(cell: &str) -> Option<String> {
    backticked(cell.trim())
}

/// The cells of a table row. An escaped `\|` stays inside its cell.
fn cells(line: &str) -> Vec<String> {
    let row = line.trim().trim_start_matches('|');
    let row = row.strip_suffix('|').unwrap_or(row);
    let mut cells = vec![String::new()];
    let mut escaped = false;
    for c in row.chars() {
        match c {
            '|' if !escaped => cells.push(String::new()),
            _ => cells.last_mut().expect("starts with one cell").push(c),
        }
        escaped = c == '\\';
    }
    cells.iter().map(|cell| cell.trim().to_owned()).collect()
}

/// Each table of a Markdown file, with the `## ` and `### ` headings above it and its rows.
fn tables(markdown: &str) -> Vec<(String, String, Vec<Vec<String>>)> {
    let separator = Regex::new(r"^\|[\s\-:|]+\|$").expect("valid pattern");
    let lines: Vec<&str> = markdown.lines().collect();
    let mut out = Vec::new();
    let (mut h2, mut h3) = (String::new(), String::new());
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(heading) = line.strip_prefix("## ") {
            heading.clone_into(&mut h2);
            h3.clear();
        } else if let Some(heading) = line.strip_prefix("### ") {
            heading.clone_into(&mut h3);
        }
        if line.starts_with('|')
            && lines
                .get(index + 1)
                .is_some_and(|l| separator.is_match(l.trim()))
        {
            let mut rows = Vec::new();
            index += 2;
            while index < lines.len() && lines[index].starts_with('|') {
                rows.push(cells(lines[index]));
                index += 1;
            }
            out.push((h2.clone(), h3.clone(), rows));
            continue;
        }
        index += 1;
    }
    out
}

fn inventory_rows(dir: &Path) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    ruff_rows(dir, &mut rows)?;
    runtime_rows(dir, &mut rows)?;
    ty_rows(dir, &mut rows)?;
    Ok(rows)
}

fn key(tool: Tool, surface: Surface, kind: Kind, id: String) -> Key {
    Key {
        tool,
        release: tool.pinned().to_owned(),
        surface,
        kind,
        id,
    }
}

/// A row whose decision cell states the decision and its reason.
fn full(tool: Tool, surface: Surface, kind: Kind, id: String, cell: &str) -> Result<Row> {
    Ok(Row {
        key: key(tool, surface, kind, id),
        word: decision_word(cell)?.to_owned(),
        reason: Some(cell.to_owned()),
    })
}

fn ruff_rows(dir: &Path, rows: &mut Vec<Row>) -> Result<()> {
    for (h2, _, table) in tables(&read(&dir.join("ruff-rules.md"))?) {
        for row in &table {
            match h2.as_str() {
                "Coverage" => {
                    let id = row[0].split(", ").next().unwrap_or_default().to_owned();
                    let mut newer = full(Tool::Ruff, Surface::Rules, Kind::Newer, id, &row[1])?;
                    NEWER_RUFF.clone_into(&mut newer.key.release);
                    rows.push(newer);
                }
                "Rules" => rows.push(full(
                    Tool::Ruff,
                    Surface::Rules,
                    Kind::Rule,
                    first_code(&row[0]).context("rule id")?,
                    &row[5],
                )?),
                "Redirected selectors" => rows.push(full(
                    Tool::Ruff,
                    Surface::Rules,
                    Kind::Redirect,
                    first_code(&row[0]).context("selector")?,
                    &row[2],
                )?),
                _ => {}
            }
        }
    }
    for (h2, _, table) in tables(&read(&dir.join("ruff-settings.md"))?) {
        let (surface, kind) = match h2.as_str() {
            "Project options" => (Surface::Project, Kind::Option),
            "Tables and schema-only members" => (Surface::Project, Kind::Schema),
            "Legacy flattened spellings" => (Surface::Project, Kind::Legacy),
            heading if heading.starts_with("Editor settings") => (Surface::Editor, Kind::Setting),
            _ => continue,
        };
        for row in &table {
            let cell = row.last().context("empty row")?;
            rows.push(full(
                Tool::Ruff,
                surface,
                kind,
                first_code(&row[0]).context("path")?,
                cell,
            )?);
        }
    }
    Ok(())
}

fn runtime_rows(dir: &Path, rows: &mut Vec<Row>) -> Result<()> {
    for (h2, h3, table) in tables(&read(&dir.join("runtime-settings.md"))?) {
        for row in &table {
            let (tool, surface, kind, id, word, reason) = match h2.as_str() {
                "Documented CLI options" => {
                    let tool = if h3.starts_with("ruff") {
                        Tool::Ruff
                    } else {
                        Tool::Ty
                    };
                    (
                        tool,
                        Surface::Cli,
                        Kind::Option,
                        first_code(&row[0]),
                        &row[2],
                        &row[3],
                    )
                }
                "Environment variables" => {
                    let tool = if row[0] == "ruff" {
                        Tool::Ruff
                    } else {
                        Tool::Ty
                    };
                    (
                        tool,
                        Surface::Env,
                        Kind::Variable,
                        first_code(&row[1]),
                        &row[2],
                        &row[3],
                    )
                }
                _ => continue,
            };
            rows.push(Row {
                key: key(tool, surface, kind, id.context("option")?),
                word: decision_word(word)?.to_owned(),
                reason: Some(reason.clone()),
            });
        }
    }
    Ok(())
}

fn ty_rows(dir: &Path, rows: &mut Vec<Row>) -> Result<()> {
    for (h2, _, table) in tables(&read(&dir.join("ty-rules.md"))?) {
        let kind = |row: &[String]| {
            if row.len() == 3 && row[1].starts_with('`') {
                Kind::Alias
            } else {
                Kind::Removed
            }
        };
        for row in &table {
            match h2.as_str() {
                "Active rules" => rows.push(full(
                    Tool::Ty,
                    Surface::Rules,
                    Kind::Rule,
                    first_code(&row[0]).context("rule")?,
                    &row[4],
                )?),
                "Historical rule names" => rows.push(full(
                    Tool::Ty,
                    Surface::Rules,
                    kind(row),
                    first_code(&row[0]).context("rule")?,
                    &row[2],
                )?),
                _ => {}
            }
        }
    }
    for (h2, h3, table) in tables(&read(&dir.join("ty-settings.md"))?) {
        for row in &table {
            match (h2.as_str(), h3.as_str()) {
                ("Project and standalone settings", _) => rows.push(full(
                    Tool::Ty,
                    Surface::Project,
                    Kind::Option,
                    first_code(&row[0]).context("path")?,
                    &row[3],
                )?),
                ("Compatibility setting paths", _) => rows.push(full(
                    Tool::Ty,
                    Surface::Project,
                    Kind::Alias,
                    first_code(&row[0]).context("path")?,
                    &row[2],
                )?),
                ("Editor and initialization settings", _) => rows.push(full(
                    Tool::Ty,
                    Surface::Editor,
                    Kind::Setting,
                    first_code(&row[0]).context("key")?,
                    &row[4],
                )?),
                ("Rule-map setting paths", "") => {
                    let global = first_code(&row[0]).context("path")?;
                    let word = if global.rsplit('.').next() == Some("all") {
                        "leave"
                    } else {
                        "inherit"
                    };
                    rows.push(Row {
                        key: key(Tool::Ty, Surface::Project, Kind::RuleMap, global),
                        word: word.to_owned(),
                        reason: None,
                    });
                    rows.push(Row {
                        key: key(
                            Tool::Ty,
                            Surface::Project,
                            Kind::RuleMap,
                            first_code(&row[1]).context("path")?,
                        ),
                        word: "leave".to_owned(),
                        reason: None,
                    });
                }
                ("Rule-map setting paths", "Compatibility rule-map paths") => {
                    for cell in row {
                        rows.push(Row {
                            key: key(
                                Tool::Ty,
                                Surface::Project,
                                Kind::RuleMapAlias,
                                first_code(cell).context("path")?,
                            ),
                            word: "leave".to_owned(),
                            reason: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
