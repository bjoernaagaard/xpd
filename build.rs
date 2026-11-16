//! Embeds the rule packs under rules/, their sections of docs/rules.md, and the agent skills
//! under skills/ into the binary.
//!
//! Each folder `rules/<module>/` is the pack for the Python library imported as `<module>`, or
//! for Python itself when its manifest says `everyFile: true`.
//! `pack.yml` is the pack's manifest, and every other file is `<rule id>.yml`, the documents of
//! one rule. This script writes `OUT_DIR/packs.rs`, which src/pack.rs includes, so a new rule or
//! a new pack needs no Rust change. It also writes `OUT_DIR/skills.rs`, which src/skills.rs
//! includes.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

/// The file in a pack folder that is the manifest and not a rule.
const MANIFEST: &str = "pack.yml";
/// The page with one `#### <rule id>` section for every rule and package check.
const RULES_PAGE: &str = "docs/rules.md";
/// The manifest of the agent plugin that this repository is.
const PLUGIN: &str = "plugin.json";
/// The plugin's file that starts xpd with `uvx xpd@VERSION`.
const PLUGIN_MCP: &str = "mcp.json";
/// The `## ` headings of the rules page that hold a pack's sections, each with the pack's
/// module. A heading names the library as its project writes it, which is not always the name
/// it is imported by.
const PACK_HEADINGS: &[(&str, &str)] = &[
    ("pandas", "pandas"),
    ("NumPy", "numpy"),
    ("SciPy", "scipy"),
    ("scikit-learn", "sklearn"),
    ("scikit-lego", "sklego"),
    ("MAPIE", "mapie"),
    ("Category Encoders", "category_encoders"),
    ("linearmodels", "linearmodels"),
    ("arch", "arch"),
    ("pymoo", "pymoo"),
    ("plotly", "plotly"),
    ("Hamilton", "hamilton"),
    ("Python", "python"),
];

/// Fails the build when the agent plugin names another version than the crate, so a release
/// cannot ship skills under an old version or start an old xpd.
fn plugin_version() -> Result<()> {
    let version = env::var("CARGO_PKG_VERSION").context("cargo sets CARGO_PKG_VERSION")?;
    let manifest = fs::read_to_string(PLUGIN).with_context(|| format!("cannot read {PLUGIN}"))?;
    if !manifest.contains(&format!("\"version\": \"{version}\"")) {
        bail!("{PLUGIN} must have \"version\": \"{version}\", as Cargo.toml does");
    }
    // An unpinned uvx would run whatever older xpd its cache holds.
    let servers =
        fs::read_to_string(PLUGIN_MCP).with_context(|| format!("cannot read {PLUGIN_MCP}"))?;
    if !servers.contains(&format!("\"xpd@{version}\"")) {
        bail!("{PLUGIN_MCP} must start \"xpd@{version}\", as Cargo.toml does");
    }
    Ok(())
}

fn main() -> Result<()> {
    let out =
        PathBuf::from(env::var_os("OUT_DIR").context("cargo sets OUT_DIR for build scripts")?);
    let root = Path::new("rules");
    // Cargo rescans a directory given here for changes to any file inside it.
    println!("cargo:rerun-if-changed={}", root.display());
    println!("cargo:rerun-if-changed={RULES_PAGE}");
    println!("cargo:rerun-if-changed=skills");
    println!("cargo:rerun-if-changed={PLUGIN}");
    println!("cargo:rerun-if-changed={PLUGIN_MCP}");
    plugin_version()?;
    println!("cargo:rerun-if-changed=indexes");
    let indexes = sorted_entries(Path::new("indexes"))?
        .iter()
        .filter(|p| {
            p.extension().is_some_and(|e| e == "json")
                && p.file_name().is_some_and(|n| n != "questions.json")
        })
        .map(|p| include(&format!("/{}", p.display())))
        .collect::<Vec<_>>()
        .join(",\n");
    write(&out.join("indexes.rs"), &format!("&[{indexes}]"))?;
    let page =
        fs::read_to_string(RULES_PAGE).with_context(|| format!("cannot read {RULES_PAGE}"))?;
    let mut docs = rule_docs(&page)?;
    let packs = sorted_entries(root)?
        .iter()
        .filter(|path| path.is_dir())
        .map(|pack| pack_source(pack, &mut docs))
        .collect::<Result<Vec<String>>>()?;
    if let Some(module) = docs.keys().next() {
        bail!(
            "build.rs: PACK_HEADINGS names the module {module}, which has no folder under rules/"
        );
    }
    write(&out.join("packs.rs"), &format!("&[\n{}]", packs.concat()))?;
    write(&out.join("skills.rs"), &skills_source(Path::new("skills"))?)
}

fn write(target: &Path, source: &str) -> Result<()> {
    fs::write(target, source).with_context(|| format!("cannot write {}", target.display()))
}

/// The `PackSource` literal for one pack folder. It takes the pack's sections out of `docs`.
fn pack_source(pack: &Path, docs: &mut BTreeMap<&str, BTreeMap<String, String>>) -> Result<String> {
    let module = utf8_name(pack)?;
    ensure!(
        is_module_name(module),
        "rules/{module}: the folder name must be a Python module name"
    );
    let mut manifest = String::from("None");
    let mut rules = Vec::new();
    let mut ids = Vec::new();
    for file in sorted_entries(pack)? {
        let name = utf8_name(&file)?;
        // Hidden files, such as the ones a file manager leaves, are not part of the pack.
        if name.starts_with('.') {
            continue;
        }
        let include = include(&format!("/rules/{module}/{name}"));
        let yaml =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        if name == MANIFEST {
            manifest = format!("Some({include})");
            // Package checks are the only list entries of a manifest with an id.
            ids.extend(
                yaml.lines()
                    .filter_map(|line| line.trim_start().strip_prefix("- id: "))
                    .map(|id| id.trim().to_owned()),
            );
        } else {
            let id = name.strip_suffix(".yml").with_context(|| {
                format!("rules/{module}/{name}: a pack holds {MANIFEST} and <rule id>.yml files")
            })?;
            let mut found = yaml.lines().filter_map(|line| line.strip_prefix("id: "));
            ensure!(
                found.all(|found| found.trim() == id),
                "rules/{module}/{name}: every rule document in the file must have the id {id:?}"
            );
            rules.push(format!(
                "            RuleSource {{ id: {id:?}, yaml: {include} }},\n"
            ));
            ids.push(id.to_owned());
        }
    }
    ensure!(!rules.is_empty(), "rules/{module}: the pack has no rules");
    let Some(mut sections) = docs.remove(module) else {
        bail!("rules/{module}: add the pack's heading on {RULES_PAGE} to PACK_HEADINGS in build.rs")
    };
    let mut pack_docs = Vec::new();
    for id in &ids {
        let text = sections.remove(id).with_context(|| {
            format!("rules/{module}: {id} has no `#### {id}` section on {RULES_PAGE}")
        })?;
        pack_docs.push(format!(
            "            DocSource {{ id: {id:?}, text: {text:?} }},\n"
        ));
    }
    if let Some(stray) = sections.keys().next() {
        bail!("{RULES_PAGE}: the section {stray} names no rule or package check of rules/{module}");
    }
    Ok(format!(
        "    PackSource {{\n        module: {module:?},\n        manifest: {manifest},\n        rules: &[\n{}        ],\n        docs: &[\n{}        ],\n    }},\n",
        rules.concat(),
        pack_docs.concat()
    ))
}

/// The `#### <id>` sections of the rules page, by the module of the pack heading they are
/// under. A section runs from its heading to the next heading of level 4 or higher, and a `#`
/// line inside a code block is code, not a heading.
fn rule_docs(page: &str) -> Result<BTreeMap<&'static str, BTreeMap<String, String>>> {
    let mut docs: BTreeMap<&str, BTreeMap<String, String>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut module = None;
    let mut section: Option<(String, Vec<&str>)> = None;
    let mut in_code = false;
    // A heading after the last line closes the last section.
    for line in page.lines().chain(std::iter::once("# ")) {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        let level = heading_level(line).filter(|_| !in_code);
        if level.is_some_and(|level| level <= 4)
            && let (Some(module), Some((id, lines))) = (module, section.take())
        {
            let text = lines.join("\n").trim_end().to_owned();
            docs.entry(module).or_default().insert(id, text);
        }
        match level {
            Some(2) => {
                let title = line[3..].trim();
                module = PACK_HEADINGS
                    .iter()
                    .find(|(heading, _)| *heading == title)
                    .map(|(_, module)| *module);
            }
            Some(4) if module.is_some() => {
                let id = line[5..].trim().to_owned();
                // The site builds the anchor from the heading alone, so a second section of
                // the same id would get another anchor than the one `xpd rules` links to.
                ensure!(
                    seen.insert(id.clone()),
                    "{RULES_PAGE}: two sections are headed `#### {id}`"
                );
                section = Some((id, Vec::new()));
            }
            _ => {}
        }
        if let Some((_, lines)) = &mut section {
            lines.push(line);
        }
    }
    Ok(docs)
}

/// The level of a Markdown heading line such as `## NumPy`, or `None` for another line.
fn heading_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|&b| b == b'#').count();
    (level > 0 && line[level..].starts_with(' ')).then_some(level)
}

/// The `SkillSource` list for every skill folder under `root`, each with all of its files.
fn skills_source(root: &Path) -> Result<String> {
    let mut skills = Vec::new();
    for skill in sorted_entries(root)? {
        let name = utf8_name(&skill)?;
        if !skill.is_dir() || name.starts_with('.') {
            continue;
        }
        let paths = files_below(&skill)?
            .iter()
            .map(|file| {
                let parts = file.strip_prefix(&skill)?.components().map(|part| {
                    part.as_os_str()
                        .to_str()
                        .context("a skill file has no UTF-8 name")
                });
                Ok(parts.collect::<Result<Vec<&str>>>()?.join("/"))
            })
            .collect::<Result<Vec<String>>>()?;
        ensure!(
            paths.iter().any(|path| path == "SKILL.md"),
            "skills/{name}: a skill folder needs a SKILL.md"
        );
        let files: Vec<String> = paths
            .iter()
            .map(|path| {
                let text = include(&format!("/skills/{name}/{path}"));
                format!("            SkillFile {{ path: {path:?}, text: {text} }},\n")
            })
            .collect();
        skills.push(format!(
            "    SkillSource {{\n        name: {name:?},\n        files: &[\n{}        ],\n    }},\n",
            files.concat()
        ));
    }
    Ok(format!("&[\n{}]", skills.concat()))
}

/// Every file below `dir`, in name order, without hidden files and folders.
fn files_below(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in sorted_entries(dir)? {
        if utf8_name(&entry)?.starts_with('.') {
            continue;
        }
        if entry.is_dir() {
            files.extend(files_below(&entry)?);
        } else {
            files.push(entry);
        }
    }
    Ok(files)
}

/// An `include_str!` of the file at `path` below the crate root.
fn include(path: &str) -> String {
    format!("include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), {path:?}))")
}

/// The entries of `dir` in name order, so the binary does not depend on the file system's order.
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .and_then(|entries| entries.map(|entry| entry.map(|e| e.path())).collect())
        .with_context(|| format!("cannot list {}", dir.display()))?;
    entries.sort();
    Ok(entries)
}

fn utf8_name(path: &Path) -> Result<&str> {
    path.file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no UTF-8 name", path.display()))
}

/// Module names also go into a regex and a dist-info lookup, so only identifiers are allowed.
fn is_module_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
