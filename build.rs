//! Embeds the rule packs under rules/, the Markdown that explains each rule, and the agent
//! skills under skills/ into the binary.
//!
//! Each folder `rules/<module>/` is the pack for the Python library imported as `<module>`, or
//! for Python itself when its manifest says `everyFile: true`.
//! `pack.yml` is the pack's manifest, `<rule id>.yml` holds the documents of one rule, and
//! `<rule id>.md` explains a rule or a package check of the manifest. `pack.md` introduces the
//! pack and lists each of its rules once under a heading, and the folder `utils/` holds
//! `<utility id>.yml`, one utility rule that every rule of the pack can name with `matches`.
//! The folder `rules/xpd/` holds the Markdown of xpd's own rule, `xpd/unused-suppression`, and
//! in `playbooks/` xpd's own recipes, which join the search index. This script writes `OUT_DIR/packs.rs`, which src/pack.rs includes, so a new rule or
//! a new pack needs no Rust change. It also writes `OUT_DIR/skills.rs`, which src/skills.rs
//! includes.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use ast_grep_config::UNUSED_SUPPRESSION_ID;
use pack_names::{OWN_PACK, is_module_name};
use serde_json::Value;

#[path = "src/pack_names.rs"]
mod pack_names;
#[path = "src/search.rs"]
mod search;

/// The file in a pack folder that is the manifest and not a rule.
const MANIFEST: &str = "pack.yml";
/// The file in a pack folder that introduces the pack and groups its rules.
const PACK_PAGE: &str = "pack.md";
/// The folder in `rules/xpd/` that holds the playbooks, which are xpd's own recipes.
const PLAYBOOKS: &str = "playbooks";
/// Where the source of a playbook can be read.
const REPOSITORY: &str = "https://github.com/bjoernaagaard/xpd";
/// The folder in a pack folder that holds the pack's shared utility rules.
const UTILS: &str = "utils";
/// The manifest of the plugin that this repository is. Claude Code and Codex both read it.
const PLUGIN: &str = ".claude-plugin/plugin.json";
/// The plugin's file that starts xpd with `uvx xpd@VERSION`.
const PLUGIN_MCP: &str = "mcp.json";
/// The vector of each index record, by the record's name. `xpd find --write-vectors` writes it.
const VECTORS: &str = "indexes/vectors.bin";
use search::{
    DIMENSIONS, TAG_ARRAY, TAG_FALSE, TAG_NULL, TAG_OBJECT, TAG_STRING, TAG_TRUE, VECTORS_HEADER,
};
/// Fails the build when the plugin names another version than the crate, so a release
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
    println!("cargo:rerun-if-changed=skills");
    println!("cargo:rerun-if-changed={PLUGIN}");
    println!("cargo:rerun-if-changed={PLUGIN_MCP}");
    plugin_version()?;
    println!("cargo:rerun-if-changed=indexes");
    let mut packs = Vec::new();
    let mut docs = BTreeMap::new();
    let mut own = None;
    // `xpd check --rule` and the old links of the form rules.html#<id>, which the site sends on
    // to the rule's page, find a rule by its id alone, so an id belongs to one pack.
    let mut owners = BTreeMap::new();
    for pack in sorted_entries(root)?.iter().filter(|path| path.is_dir()) {
        let module = utf8_name(pack)?;
        if module == OWN_PACK {
            let ids = [UNUSED_SUPPRESSION_ID.to_owned()];
            own = Some(doc_sources(&rule_markdown(pack, &ids)?));
            continue;
        }
        let (source, pack_docs) = pack_source(pack)?;
        for id in pack_docs.keys() {
            if let Some(first) = owners.insert(id.clone(), module.to_owned()) {
                bail!("rules/{module}: the id {id} is also a rule of rules/{first}");
            }
        }
        packs.push(source);
        docs.insert(module.to_owned(), pack_docs);
    }
    let own = own.with_context(|| format!("rules/{OWN_PACK} is missing"))?;
    discovery_source(&out, &docs)?;
    write(&out.join("packs.rs"), &format!("&[\n{}]", packs.concat()))?;
    write(&out.join("own_docs.rs"), &format!("&[\n{own}]"))?;
    write(&out.join("skills.rs"), &skills_source(Path::new("skills"))?)
}

fn write(target: &Path, source: &str) -> Result<()> {
    fs::write(target, source).with_context(|| format!("cannot write {}", target.display()))
}

/// The `PackSource` literal for one pack folder, and the Markdown of each of its rules and
/// package checks by id.
fn pack_source(pack: &Path) -> Result<(String, BTreeMap<String, String>)> {
    let module = utf8_name(pack)?;
    ensure!(
        is_module_name(module) && !module.bytes().any(|byte| byte.is_ascii_uppercase()),
        "rules/{module}: the folder name must be a Python module name"
    );
    let mut manifest = String::from("None");
    let mut rules = Vec::new();
    let mut utils = Vec::new();
    let mut ids = Vec::new();
    for file in sorted_entries(pack)? {
        let name = utf8_name(&file)?;
        // Hidden files, such as the ones a file manager leaves, are not part of the pack.
        if name.starts_with('.') || is_markdown(&file) {
            continue;
        }
        if name == UTILS && file.is_dir() {
            utils = util_sources(module, &file)?;
            continue;
        }
        let include = include(&format!("/rules/{module}/{name}"));
        let yaml =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        if name == MANIFEST {
            manifest = format!("Some({include})");
            let data: Value =
                ast_grep_config::from_str(&yaml).with_context(|| file.display().to_string())?;
            ids.extend(
                data.get("packages")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(yaml_id)
                    .map(str::to_owned),
            );
        } else {
            let id = name.strip_suffix(".yml").with_context(|| {
                format!("rules/{module}/{name}: a pack holds {MANIFEST}, {PACK_PAGE}, <rule id>.yml and <rule id>.md files")
            })?;
            let documents: Vec<Value> = ast_grep_config::from_multiple(&yaml)
                .with_context(|| file.display().to_string())?;
            ensure!(
                !documents.is_empty() && documents.iter().all(|doc| yaml_id(doc) == Some(id)),
                "rules/{module}/{name}: every rule document in the file must have the id {id:?}"
            );
            let path = format!("rules/{module}/{name}");
            rules.push(format!(
                "            RuleSource {{ path: {path:?}, yaml: {include} }},\n"
            ));
            ids.push(id.to_owned());
        }
    }
    ensure!(!rules.is_empty(), "rules/{module}: the pack has no rules");
    let docs = rule_markdown(pack, &ids)?;
    let source = format!(
        "    PackSource {{\n        module: {module:?},\n        manifest: {manifest},\n        rules: &[\n{}        ],\n        utils: &[\n{}        ],\n        docs: &[\n{}        ],\n    }},\n",
        rules.concat(),
        utils.concat(),
        doc_sources(&docs)
    );
    Ok((source, docs))
}

/// The text of `<id>.md` in the pack folder for each id in `ids`, trimmed at the end. Fails when
/// a rule has no Markdown file, when a Markdown file names no rule, and when `pack.md` does not
/// list every id once.
fn rule_markdown(pack: &Path, ids: &[String]) -> Result<BTreeMap<String, String>> {
    let module = utf8_name(pack)?;
    let mut docs = BTreeMap::new();
    let mut page = None;
    for file in sorted_entries(pack)? {
        let name = utf8_name(&file)?;
        let Some(id) = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|_| is_markdown(&file) && !name.starts_with('.'))
        else {
            continue;
        };
        let text =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        if name == PACK_PAGE {
            page = Some(text);
        } else {
            ensure!(
                ids.iter().any(|known| known == id),
                "rules/{module}/{name} names no rule or package check of the pack"
            );
            docs.insert(id.to_owned(), text.trim_end().to_owned());
        }
    }
    for id in ids {
        ensure!(
            docs.contains_key(id),
            "rules/{module}: {id} has no rules/{module}/{id}.md, which xpd rules, xpd find, the editor hover and the site show"
        );
    }
    let page = page.with_context(|| format!("rules/{module} has no {PACK_PAGE}"))?;
    let mut listed = BTreeSet::new();
    let mut in_code = false;
    for line in page.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        // A list item that is a bare rule id puts the rule in the group of the heading above.
        let Some(item) = line.strip_prefix("- ").filter(|item| {
            !in_code
                && item
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        }) else {
            continue;
        };
        ensure!(
            docs.contains_key(item),
            "rules/{module}/{PACK_PAGE} lists {item}, which is no rule or package check of the pack"
        );
        ensure!(
            listed.insert(item),
            "rules/{module}/{PACK_PAGE} lists {item} twice"
        );
    }
    if let Some(id) = ids.iter().find(|id| !listed.contains(id.as_str())) {
        bail!("rules/{module}/{PACK_PAGE} lists no `- {id}` under a heading");
    }
    Ok(docs)
}

/// The `DocSource` literals of the Markdown of a pack.
fn doc_sources(docs: &BTreeMap<String, String>) -> String {
    docs.iter().fold(String::new(), |mut out, (id, text)| {
        // Writing to a String cannot fail.
        let _ = writeln!(
            out,
            "            DocSource {{ id: {id:?}, text: {text:?} }},"
        );
        out
    })
}

/// Whether `path` is a Markdown file, which explains a rule and is no rule itself.
fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "md")
}

/// The `RuleSource` literals for the utility rules in the `utils` folder of the pack `module`.
fn util_sources(module: &str, utils: &Path) -> Result<Vec<String>> {
    let mut sources = Vec::new();
    for file in sorted_entries(utils)? {
        let name = utf8_name(&file)?;
        if name.starts_with('.') {
            continue;
        }
        let path = format!("rules/{module}/{UTILS}/{name}");
        let id = name
            .strip_suffix(".yml")
            .with_context(|| format!("{path}: the folder holds <utility id>.yml files"))?;
        let yaml =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        let documents: Vec<Value> =
            ast_grep_config::from_multiple(&yaml).with_context(|| file.display().to_string())?;
        ensure!(
            matches!(documents.as_slice(), [document] if yaml_id(document) == Some(id)),
            "{path}: the file must hold one utility rule with the id {id:?}"
        );
        let include = include(&format!("/{path}"));
        sources.push(format!(
            "            RuleSource {{ path: {path:?}, yaml: {include} }},\n"
        ));
    }
    Ok(sources)
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

fn yaml_id(value: &Value) -> Option<&str> {
    value.get("id").and_then(Value::as_str)
}

fn discovery_source(out: &Path, docs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<()> {
    let mut entries = Vec::<serde_json::Value>::new();
    for path in sorted_entries(Path::new("indexes"))? {
        if path.extension().is_none_or(|e| e != "json") || path.ends_with("questions.json") {
            continue;
        }
        let index: serde_json::Value = serde_json::from_str(&fs::read_to_string(path)?)?;
        entries.extend(
            index["entries"]
                .as_array()
                .context("index entries")?
                .iter()
                .cloned(),
        );
    }
    entries.extend(playbooks(docs)?);
    for (package, sections) in docs {
        for (id, doc) in sections {
            entries.push(serde_json::json!({"kind":"rule","package":package,"name":format!("{package}/{id}"),"callable":null,"parameters":[],"summary":doc.trim(),"members":[],"rules":[]}));
        }
    }
    let mut records = String::from("&[\n");
    let mut strings = BTreeMap::new();
    let mut data = Vec::new();
    for entry in &entries {
        let name = entry["name"].as_str().context("entry name")?;
        let package = entry["package"].as_str().context("entry package")?;
        let kind = match entry["kind"].as_str() {
            Some("rule") => "Rule",
            Some("recipe") => "Recipe",
            _ => "Symbol",
        };
        let summary = entry["summary"].as_str().unwrap_or_default();
        let text = text_hash(name, summary);
        let mut output = entry.clone();
        output
            .as_object_mut()
            .context("entry object")?
            .remove("keywords");
        let start = data.len();
        encode(&output, &mut strings, &mut data)?;
        let len = u16::try_from(data.len() - start)?;
        writeln!(
            records,
            "Stored {{name:{name:?},package:{package:?},kind:Kind::{kind},summary:{summary:?},text:0x{:04x}_{:04x}_{:04x}_{:04x},start:0x{:04x}_{:04x},len:{len}}},",
            text >> 48,
            (text >> 32) & 0xffff,
            (text >> 16) & 0xffff,
            text & 0xffff,
            start >> 16,
            start & 0xffff
        )?;
    }
    records.push(']');
    fs::write(out.join("entries.bin"), data)?;
    let mut ordered = vec![""; strings.len()];
    for (text, id) in &strings {
        ordered[usize::from(*id)] = text;
    }
    write(&out.join("strings.rs"), &format!("&{ordered:?}"))?;
    write(&out.join("records.rs"), &records)?;
    vectors_source(out, &entries)?;
    search_source(out, &entries)
}

/// The playbooks under `rules/xpd/playbooks`, each as a recipe of the `xpd` package named
/// `xpd/recipe/<file name>`. A playbook is a Markdown file with a title, a first paragraph and
/// the sections `Asks`, `Code`, `Commands` and `Rules`. Its asks are in the summary, which the
/// keyword search and the embedding model both read, so a loose ask finds its playbook. `docs`
/// holds the rule ids of each pack, and a playbook that names another rule fails the build.
fn playbooks(docs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<Vec<serde_json::Value>> {
    let version = env::var("CARGO_PKG_VERSION").context("cargo sets CARGO_PKG_VERSION")?;
    let mut entries = Vec::new();
    for file in sorted_entries(&Path::new("rules").join(OWN_PACK).join(PLAYBOOKS))? {
        let name = utf8_name(&file)?;
        let Some(stem) = name.strip_suffix(".md") else {
            continue;
        };
        let text =
            fs::read_to_string(&file).with_context(|| format!("cannot read {}", file.display()))?;
        let mut sections = BTreeMap::<&str, Vec<&str>>::new();
        let mut section = "";
        let mut in_code = false;
        for line in text.lines() {
            if line.starts_with("```") {
                in_code = !in_code;
            } else if let Some(heading) = line.strip_prefix("## ").filter(|_| !in_code) {
                section = heading.trim();
            } else {
                sections.entry(section).or_default().push(line);
            }
        }
        let lines = |section: &str| sections.get(section).cloned().unwrap_or_default();
        // A list item without its `- ` or `1. ` mark.
        let items = |section: &str| -> Vec<String> {
            (lines(section).iter())
                .filter_map(|line| line.split_once(' '))
                .filter(|(mark, _)| *mark == "-" || mark.ends_with('.'))
                .map(|(_, item)| item.trim().to_owned())
                .collect()
        };
        let intro = lines("");
        let title = (intro.iter())
            .find_map(|line| line.strip_prefix("# "))
            .with_context(|| format!("{}: a playbook starts with a `# ` title", file.display()))?;
        let about: Vec<&str> = (intro.iter().copied())
            .filter(|line| !line.starts_with("# ") && !line.trim().is_empty())
            .collect();
        let (asks, commands, rules) = (items("Asks"), items("Commands"), items("Rules"));
        let code = format!("{}\n", lines("Code").join("\n").trim());
        ensure!(
            !about.is_empty() && !asks.is_empty() && !commands.is_empty() && code.trim() != "",
            "{}: a playbook has a first paragraph and the sections Asks, Code and Commands",
            file.display()
        );
        for rule in &rules {
            let known = rule
                .split_once('/')
                .is_some_and(|(pack, id)| docs.get(pack).is_some_and(|ids| ids.contains_key(id)));
            ensure!(known, "{}: {rule} is no rule of a pack", file.display());
        }
        let summary = format!(
            "{title}. {}\n\nIt answers asks such as: {}.",
            about.join(" "),
            asks.join(", ")
        );
        entries.push(serde_json::json!({
            "kind": "recipe",
            "package": OWN_PACK,
            "name": format!("{OWN_PACK}/recipe/{stem}"),
            "callable": null,
            "parameters": [],
            "summary": summary,
            "keywords": asks,
            "members": [],
            "rules": rules,
            "recipe": {
                "title": title,
                "symbols": [],
                "source": format!("{REPOSITORY}/blob/v{version}/rules/{OWN_PACK}/{PLAYBOOKS}/{name}"),
                "code": code,
                "license": "MIT OR Apache-2.0",
                "modified": false,
                "asks": asks,
                "commands": commands,
            },
        }));
    }
    Ok(entries)
}

/// The FNV-1a hash of the text that a record's vector is made from, which is its name and its
/// summary. A record whose text changed gets another hash, so its stored vector is dropped.
fn text_hash(name: &str, summary: &str) -> u64 {
    name.bytes()
        .chain([0])
        .chain(summary.bytes())
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

/// Writes `OUT_DIR/vectors.bin`, the vector of each entry in the order of the entries. An entry
/// gets zeros when `VECTORS` has no vector for it, or has one made from another text, and
/// `xpd find --check` fails on it.
fn vectors_source(out: &Path, entries: &[serde_json::Value]) -> Result<()> {
    let bytes = if Path::new(VECTORS).exists() {
        fs::read(VECTORS).with_context(|| format!("cannot read {VECTORS}"))?
    } else {
        VECTORS_HEADER.to_vec()
    };
    let mut rest = bytes
        .strip_prefix(VECTORS_HEADER)
        .with_context(|| format!("{VECTORS} does not start with {VECTORS_HEADER:?}"))?;
    let mut found = BTreeMap::new();
    while let Some((length, tail)) = rest.split_first_chunk::<2>() {
        let (name, tail) = tail
            .split_at_checked(usize::from(u16::from_le_bytes(*length)))
            .with_context(|| format!("{VECTORS} ends inside a name"))?;
        let (hash, tail) = tail
            .split_first_chunk::<8>()
            .with_context(|| format!("{VECTORS} ends inside a text hash"))?;
        let (vector, tail) = tail
            .split_at_checked(DIMENSIONS)
            .with_context(|| format!("{VECTORS} ends inside a vector"))?;
        found.insert(
            std::str::from_utf8(name)?,
            (u64::from_le_bytes(*hash), vector),
        );
        rest = tail;
    }
    ensure!(rest.is_empty(), "{VECTORS} ends inside a name length");
    let mut data = Vec::with_capacity(entries.len() * DIMENSIONS);
    for entry in entries {
        let name = entry["name"].as_str().context("entry name")?;
        let summary = entry["summary"].as_str().unwrap_or_default();
        match found.get(name) {
            Some((hash, vector)) if *hash == text_hash(name, summary) => {
                data.extend_from_slice(vector);
            }
            _ => data.resize(data.len() + DIMENSIONS, 0),
        }
    }
    fs::write(out.join("vectors.bin"), data)?;
    Ok(())
}

fn search_source(out: &Path, entries: &[serde_json::Value]) -> Result<()> {
    let documents = entries
        .iter()
        .map(|e| {
            let name = e["name"].as_str().unwrap_or_default();
            [
                name.rsplit('.').next().unwrap_or(name).to_owned(),
                name.to_owned(),
                e["summary"].as_str().unwrap_or_default().to_owned(),
                e["parameters"].to_string(),
                e["keywords"].to_string(),
                e["members"]
                    .as_array()
                    .map(|members| {
                        members
                            .iter()
                            .filter_map(|m| m["summary"].as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    let table = search::weights(&documents);
    let mut source = String::from("&[\n");
    let mut bytes = Vec::new();
    for (word, postings) in table {
        let start = bytes.len();
        for (i, score) in postings {
            bytes.extend(u16::try_from(i)?.to_le_bytes());
            let quantized = format!("{:.0}", score * 256.0).parse::<u16>()?;
            bytes.extend(quantized.to_le_bytes());
        }
        let end = bytes.len();
        writeln!(
            source,
            "({word:?}, 0x{:04x}_{:04x}, 0x{:04x}_{:04x}),",
            start >> 16,
            start & 0xffff,
            end >> 16,
            end & 0xffff
        )?;
    }
    source.push(']');
    fs::write(out.join("postings.bin"), bytes)?;
    write(&out.join("search.rs"), &source)
}

fn intern(text: &str, strings: &mut BTreeMap<String, u16>, data: &mut Vec<u8>) -> Result<()> {
    let next = u16::try_from(strings.len())?;
    data.extend(strings.entry(text.to_owned()).or_insert(next).to_le_bytes());
    Ok(())
}

fn encode(
    value: &serde_json::Value,
    strings: &mut BTreeMap<String, u16>,
    data: &mut Vec<u8>,
) -> Result<()> {
    use serde_json::Value;
    match value {
        Value::Null => data.push(TAG_NULL),
        Value::Bool(false) => data.push(TAG_FALSE),
        Value::Bool(true) => data.push(TAG_TRUE),
        Value::String(text) => {
            data.push(TAG_STRING);
            intern(text, strings, data)?;
        }
        Value::Array(items) => {
            data.push(TAG_ARRAY);
            data.extend(u16::try_from(items.len())?.to_le_bytes());
            for item in items {
                encode(item, strings, data)?;
            }
        }
        Value::Object(fields) => {
            data.push(TAG_OBJECT);
            data.extend(u16::try_from(fields.len())?.to_le_bytes());
            for (key, value) in fields {
                intern(key, strings, data)?;
                encode(value, strings, data)?;
            }
        }
        Value::Number(_) => bail!("discovery entries contain no numeric fields"),
    }
    Ok(())
}
