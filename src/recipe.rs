//! Worked examples read as data and checked by the same rules as user code.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{
    index::{Entry, Kind},
    pack,
    rule::Applicability,
    scan,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Recipe {
    pub(crate) title: String,
    pub(crate) symbols: Vec<String>,
    pub(crate) source: String,
    pub(crate) code: Option<String>,
    pub(crate) license: String,
    pub(crate) attribution: String,
    pub(crate) modified: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Stats {
    pub(crate) candidates: usize,
    pub(crate) clean: usize,
    pub(crate) fixed: usize,
    pub(crate) dropped: usize,
    pub(crate) capped: usize,
    pub(crate) retained: usize,
}

pub(crate) struct Source<'a> {
    pub(crate) root: &'a Path,
    pub(crate) url: &'a str,
    pub(crate) license: &'a Path,
    pub(crate) notice: Option<&'a Path>,
}

fn paths(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for item in std::fs::read_dir(root)? {
        let item = item?;
        if item.file_type()?.is_symlink() {
            continue;
        }
        let path = item.path();
        let name = item.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.')
            || matches!(
                name.as_ref(),
                "tests"
                    | "test"
                    | "testing"
                    | "vendor"
                    | "vendored"
                    | "externals"
                    | "__pycache__"
                    | "node_modules"
            )
        {
            continue;
        }
        if path.is_dir() {
            paths(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("py" | "md" | "rst" | "ipynb")))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn dedent(code: &str) -> String {
    let indent = code
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut result = code
        .lines()
        .map(|line| line.get(indent..).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");
    result.push('\n');
    result
}

fn fenced(text: &str) -> Vec<(String, String, usize)> {
    let mut title = String::new();
    let mut code = None::<String>;
    let mut found = Vec::new();
    let mut start = 1;
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(block) = &mut code {
            if trimmed.starts_with("```") {
                found.push((title.clone(), code.take().unwrap_or_default(), start));
            } else {
                block.push_str(line);
                block.push('\n');
            }
        } else if trimmed.starts_with("```python") || trimmed == "```py" {
            start = number + 2;
            code = Some(String::new());
        } else if trimmed.starts_with('#') {
            trimmed
                .trim_start_matches('#')
                .trim()
                .clone_into(&mut title);
        }
    }
    found
}

fn doctest(text: &str) -> String {
    if text.contains("Traceback (most recent call last)") {
        return String::new();
    }
    let mut code = String::new();
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(line) = line
            .strip_prefix(">>> ")
            .or_else(|| line.strip_prefix("... "))
        {
            code.push_str(line.split("# doctest:").next().unwrap_or(line));
            code.push('\n');
        }
    }
    code
}

fn string_content(text: &str) -> &str {
    let Some(start) = text.find(['\'', '"']) else {
        return "";
    };
    let quote = &text[start..=start];
    let width = if text[start..].starts_with(&quote.repeat(3)) {
        3
    } else {
        1
    };
    text.get(start + width..text.len().saturating_sub(width))
        .unwrap_or_default()
}

fn notebook(path: &Path, source: &str) -> Vec<(String, String, usize)> {
    let Ok(notebook) = serde_json::from_str::<serde_json::Value>(source) else {
        return Vec::new();
    };
    let Some(cells) = notebook["cells"].as_array() else {
        return Vec::new();
    };
    let mut title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ");
    let mut examples = Vec::new();
    for cell in cells {
        let text = cell["source"]
            .as_array()
            .map(|lines| {
                lines
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<String>()
            })
            .or_else(|| cell["source"].as_str().map(str::to_owned))
            .unwrap_or_default();
        if cell["cell_type"] == "markdown" {
            if let Some(heading) = text.lines().find(|line| line.starts_with('#')) {
                heading
                    .trim_start_matches('#')
                    .trim()
                    .clone_into(&mut title);
            }
        } else if cell["cell_type"] == "code" {
            let first = text.lines().find(|s| !s.is_empty()).unwrap_or_default();
            let quoted = serde_json::to_string(first).unwrap_or_default();
            let line = source
                .lines()
                .position(|s| s.contains(quoted.trim_matches('"')))
                .unwrap_or(0)
                + 1;
            examples.push((title.clone(), text, line));
        }
    }
    examples
}

fn candidates(path: &Path, text: &str) -> Vec<(String, String, usize)> {
    if path.extension().is_some_and(|e| e == "ipynb") {
        return notebook(path, text);
    }
    if path.extension().is_some_and(|e| e == "md") {
        return fenced(text);
    }
    if path.extension().is_some_and(|e| e == "rst") {
        return vec![(
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            doctest(text),
            1,
        )];
    }
    let tree = scan::parse(text);
    let mut result = Vec::new();
    for node in tree.root().dfs().filter(|n| {
        matches!(
            n.kind().as_ref(),
            "module" | "class_definition" | "function_definition"
        )
    }) {
        if node
            .field("name")
            .is_some_and(|name| name.text().starts_with('_'))
        {
            continue;
        }
        let body = node.field("body").unwrap_or_else(|| node.clone());
        let Some(first) = body
            .children()
            .find(|n| n.is_named() && n.kind() != "comment")
        else {
            continue;
        };
        if first.kind() != "expression_statement" {
            continue;
        }
        let Some(string) = first.children().find(|n| n.kind() == "string") else {
            continue;
        };
        let raw = string.text();
        let doc = string_content(&raw);
        let title = doc
            .trim()
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_owned();
        let code = doctest(doc);
        if !code.is_empty() {
            result.push((title.clone(), code, string.start_pos().line() + 1));
        }
        result.extend(fenced(doc).into_iter().map(|(heading, code, line)| {
            (
                if heading.is_empty() {
                    title.clone()
                } else {
                    heading
                },
                code,
                string.start_pos().line() + line,
            )
        }));
    }
    if path
        .components()
        .any(|part| matches!(part.as_os_str().to_str(), Some("examples" | "gallery")))
    {
        let title = result.first().map_or_else(
            || {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace('_', " ")
            },
            |(t, _, _)| t.clone(),
        );
        result.push((title, text.to_owned(), 1));
    }
    result
}

fn symbols(code: &str, package: &str) -> Vec<String> {
    let tree = scan::parse(code);
    let mut bindings = BTreeMap::new();
    for node in tree.root().dfs() {
        if node.kind() == "import_statement" {
            for child in node.children() {
                if child.kind() == "aliased_import" {
                    if let (Some(name), Some(alias)) = (child.field("name"), child.field("alias")) {
                        bindings.insert(alias.text().into_owned(), name.text().into_owned());
                    }
                } else if child.kind() == "dotted_name" {
                    let name = child.text().into_owned();
                    bindings.insert(name.clone(), name);
                }
            }
        } else if node.kind() == "import_from_statement" {
            let Some(module) = node.field("module_name") else {
                continue;
            };
            for child in node
                .children()
                .filter(|n| n.range().start > module.range().end)
            {
                match child.kind().as_ref() {
                    "dotted_name" => {
                        bindings.insert(
                            child.text().into_owned(),
                            format!("{}.{}", module.text(), child.text()),
                        );
                    }
                    "aliased_import" => {
                        if let (Some(name), Some(alias)) =
                            (child.field("name"), child.field("alias"))
                        {
                            bindings.insert(
                                alias.text().into_owned(),
                                format!("{}.{}", module.text(), name.text()),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut used = BTreeSet::new();
    for call in tree.root().dfs().filter(|n| n.kind() == "call") {
        let Some(function) = call.field("function") else {
            continue;
        };
        let name = function.text();
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
        {
            continue;
        }
        let (head, tail) = name.split_once('.').unwrap_or((&name, ""));
        if let Some(base) = bindings.get(head) {
            let name = if tail.is_empty() {
                base.clone()
            } else {
                format!("{base}.{tail}")
            };
            if name == package || name.starts_with(&format!("{package}.")) {
                used.insert(name);
            }
        }
    }
    used.into_iter().collect()
}

fn complete(code: &str) -> bool {
    let tree = scan::parse(code);
    let root = tree.root();
    let mut defined = BTreeSet::new();
    for node in root.dfs() {
        let target = match node.kind().as_ref() {
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                node.field("left")
            }
            "function_definition" | "class_definition" => node.field("name"),
            _ => None,
        };
        if let Some(target) = target {
            for name in target.dfs().filter(|n| n.kind() == "identifier") {
                defined.insert(name.text().into_owned());
            }
        }
        if matches!(node.kind().as_ref(), "parameters" | "lambda_parameters") {
            for name in node.dfs().filter(|n| n.kind() == "identifier") {
                defined.insert(name.text().into_owned());
            }
        }
        if matches!(
            node.kind().as_ref(),
            "import_statement" | "import_from_statement"
        ) {
            for name in node.dfs().filter(|n| n.kind() == "identifier") {
                defined.insert(name.text().into_owned());
            }
        }
        if matches!(node.kind().as_ref(), "ERROR" | "MISSING" | "ellipsis") {
            return false;
        }
    }
    let builtins = "abs all any ascii bin bool breakpoint bytearray bytes callable chr classmethod compile complex delattr dict dir divmod enumerate eval exec filter float format frozenset getattr globals hasattr hash help hex id input int isinstance issubclass iter len list locals map max memoryview min next object oct open ord pow print property range repr reversed round set setattr slice sorted staticmethod str sum super tuple type vars zip __import__ __name__ __file__ Exception ValueError TypeError RuntimeError StopIteration NotImplementedError None True False";
    for name in root.dfs().filter(|n| n.kind() == "identifier") {
        if name.ancestors().any(|p| {
            matches!(
                p.kind().as_ref(),
                "import_statement" | "import_from_statement"
            )
        }) {
            continue;
        }
        if let Some(parent) = name.parent() {
            let field = if parent.kind() == "attribute" {
                "attribute"
            } else if parent.kind() == "keyword_argument" {
                "name"
            } else {
                ""
            };
            if parent
                .field(field)
                .is_some_and(|n| n.range() == name.range())
            {
                continue;
            }
        }
        let text = name.text();
        if !defined.contains(text.as_ref()) && !builtins.split_whitespace().any(|s| s == text) {
            return false;
        }
    }
    true
}

#[derive(Debug)]
struct Attribution {
    name: &'static str,
    text: String,
}
impl Attribution {
    fn read(source: &Source<'_>) -> Result<Self> {
        ensure!(
            source.url.starts_with("https://") && source.url.contains("/blob/"),
            "--source-url must point to a pinned repository revision, including /blob/<revision>"
        );
        let revision = source
            .url
            .split("/blob/")
            .nth(1)
            .and_then(|tail| tail.split('/').next())
            .unwrap_or_default();
        ensure!(
            !revision.is_empty() && !["main", "master", "HEAD", "latest"].contains(&revision),
            "pin the source URL to a tag or commit"
        );
        let license = std::fs::read_to_string(source.license)?;
        let mut attribution = license.clone();
        for name in [
            "NOTICE",
            "NOTICE.txt",
            "NOTICE.md",
            "DISCLAIMER",
            "THIRD_PARTY_NOTICES.md",
        ] {
            let roots: BTreeSet<_> = [Some(source.root), source.license.parent()]
                .into_iter()
                .flatten()
                .collect();
            for root in roots {
                let path = root.join(name);
                if path.is_file() {
                    attribution.push('\n');
                    attribution.push_str(&std::fs::read_to_string(path)?);
                }
            }
        }
        if let Some(notice) = source.notice {
            attribution.push('\n');
            attribution.push_str(&std::fs::read_to_string(notice)?);
        }
        let name = if license.contains("Apache License") && license.contains("Version 2.0") {
            "Apache-2.0"
        } else if license.contains("Permission is hereby granted")
            && license.contains("Redistributions of source code")
        {
            "NCSA"
        } else if license.contains("Permission is hereby granted, free of charge") {
            "MIT"
        } else if license.contains("Redistribution and use in source and binary forms") {
            "BSD-3-Clause"
        } else {
            "pointer only"
        };
        Ok(Attribution {
            name,
            text: attribution,
        })
    }
}

#[derive(Debug)]
struct Verified {
    code: String,
    symbols: Vec<String>,
    fixes: usize,
    clean: bool,
}

fn validate(code: String, package: &str, packs: &[pack::Pack]) -> Option<Verified> {
    if !complete(&code)
        || !code
            .lines()
            .any(|line| line.starts_with("import ") || line.starts_with("from "))
    {
        return None;
    }
    let path = Path::new("recipe.py");
    let clean = scan::lint_source(path, &code, packs).is_empty();
    let (code, fixes) = scan::fix_source(path, code, packs, Applicability::Unsafe);
    if !complete(&code) || !scan::lint_source(path, &code, packs).is_empty() {
        return None;
    }
    let symbols = symbols(&code, package);
    if symbols.is_empty()
        || symbols
            .iter()
            .any(|name| name.split('.').any(|part| part.starts_with('_')))
    {
        return None;
    }
    Some(Verified {
        code,
        symbols,
        fixes,
        clean,
    })
}

pub(crate) fn harvest(
    source: &Source<'_>,
    package: &str,
    entries: &mut Vec<Entry>,
) -> Result<Stats> {
    let attribution = Attribution::read(source)?;
    let packs = pack::load(None)?;
    let mut files = Vec::new();
    paths(source.root, &mut files)?;
    files.sort();
    let mut stats = Stats::default();
    let mut seen = BTreeSet::new();
    let first = entries.len();
    for path in files {
        if std::fs::metadata(&path)?.len() > 2_000_000 {
            continue;
        }
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        for (title, code, line) in candidates(&path, &text) {
            let code = dedent(&code);
            if code.trim().is_empty() || !seen.insert(code.clone()) {
                continue;
            }
            stats.candidates += 1;
            let Some(verified) = validate(code, package, &packs) else {
                stats.dropped += 1;
                continue;
            };
            if verified.clean {
                stats.clean += 1;
            } else {
                stats.fixed += 1;
            }
            let fixed = verified.code;
            let relative = path
                .strip_prefix(source.root)?
                .to_string_lossy()
                .replace('\\', "/");
            let source_url = format!("{}/{}#L{line}", source.url.trim_end_matches('/'), relative);
            let title = if title.is_empty() {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .replace('-', " ")
            } else {
                title
            };
            let mut notice = attribution.text.clone();
            for line in text
                .lines()
                .filter(|line| line.to_lowercase().contains("copyright"))
            {
                notice.push('\n');
                notice.push_str(line);
            }
            let recipe = Recipe {
                title: title.clone(),
                symbols: verified.symbols,
                source: source_url,
                code: (attribution.name != "pointer only").then_some(fixed.clone()),
                license: attribution.name.to_owned(),
                attribution: notice,
                modified: verified.fixes > 0,
            };
            entries.push(Entry {
                kind: Kind::Recipe,
                package: package.to_owned(),
                name: recipe_name(package, &title),
                callable: None,
                parameters: Vec::new(),
                summary: title,
                keywords: vec![fixed],
                members: Vec::new(),
                rules: Vec::new(),
                recipe: Some(recipe),
            });
        }
    }
    select(entries, first, &mut stats);
    Ok(stats)
}

fn select(entries: &mut Vec<Entry>, first: usize, stats: &mut Stats) {
    let mut accepted = entries.split_off(first);
    let public: BTreeSet<_> = entries.iter().map(|e| e.name.clone()).collect();
    let primary = |entry: &Entry| {
        entry
            .recipe
            .as_ref()
            .and_then(|r| {
                r.symbols
                    .iter()
                    .filter(|s| public.contains(*s))
                    .min_by_key(|s| (s.split('.').count(), s.len()))
                    .cloned()
            })
            .or_else(|| {
                entry.recipe.as_ref().and_then(|r| {
                    r.symbols
                        .iter()
                        .min_by_key(|s| (s.split('.').count(), s.len()))
                        .cloned()
                })
            })
            .unwrap_or_else(|| entry.name.clone())
    };
    accepted.sort_by_key(|e| {
        (
            primary(e).split('.').count(),
            e.recipe
                .as_ref()
                .and_then(|r| r.code.as_ref())
                .map_or(0, String::len),
            e.name.clone(),
        )
    });
    let mut chosen = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut bytes = 0;
    for entry in accepted {
        let length = entry
            .recipe
            .as_ref()
            .and_then(|r| r.code.as_ref())
            .map_or(0, String::len);
        if stats.retained >= 24
            || length > 8000
            || bytes + length > 96_000
            || !chosen.insert(primary(&entry))
            || !names.insert(entry.name.clone())
        {
            stats.capped += 1;
            continue;
        }
        entries.push(entry);
        stats.retained += 1;
        bytes += length;
    }
}

pub(crate) fn recipe_name(package: &str, title: &str) -> String {
    let slug = title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");
    format!("{package}/recipe/{slug}")
}
