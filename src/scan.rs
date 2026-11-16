//! Walks paths, fixes and lints each file with the rule packs that apply to it.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ast_grep_config::CombinedScan;
use ast_grep_core::source::Edit;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{AstGrep, Language, NodeMatch};
use ast_grep_language::SupportLang;

use crate::pack::Pack;
use crate::report::{Issue, Position};
use crate::rule::{Applicability, Rule};

/// A parsed file.
pub(crate) type Root = AstGrep<StrDoc<SupportLang>>;

/// One node of a parsed file.
pub(crate) type Node<'t> = ast_grep_core::Node<'t, StrDoc<SupportLang>>;

/// Parses Python source. A syntax error gives a tree with error nodes, not a failure.
pub(crate) fn parse(src: &str) -> Root {
    parse_as(src, SupportLang::Python)
}

/// Parses source of `language`.
fn parse_as(src: &str, language: SupportLang) -> Root {
    use ast_grep_core::tree_sitter::LanguageExt;

    language.ast_grep(src)
}

/// The language of the file at `path`, by its name. A file that names no language is read as
/// Python, so that a script without an extension is linted.
fn language_of(path: &Path) -> SupportLang {
    SupportLang::from_path(path).unwrap_or(SupportLang::Python)
}

/// One `import <module> as <alias>` in a file.
#[derive(Debug)]
struct AliasedImport {
    module: String,
    alias: String,
}

/// One file, as a lint run names it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct File<'a> {
    /// The path that the `files` and `ignores` globs of a rule match, which selects the rules
    /// that run.
    pub(crate) path: &'a Path,
    /// Where the file is on disk, from which a rule follows its imports. `None` for code that
    /// is in no file, such as a recipe.
    pub(crate) disk: Option<&'a Path>,
}

impl<'a> File<'a> {
    /// A file that is on disk at the path its rules match.
    pub(crate) fn on_disk(path: &'a Path) -> Self {
        Self {
            path,
            disk: Some(path),
        }
    }
}

/// Whether `path` has the `.py` extension.
pub(crate) fn is_python(path: &Path) -> bool {
    path.extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("py"))
}

/// Whether the name of `path` tells its language.
pub(crate) fn names_language(path: &Path) -> bool {
    SupportLang::from_path(path).is_some()
}

fn cannot_read(path: &Path) -> String {
    format!("cannot read {}", path.display())
}

/// The files under `paths` that `packs` have rules for, each once: the Python files, and the
/// files of each other language that a rule names. A path that is a file is always taken.
/// Folders respect `.gitignore`.
pub(crate) fn collect_files(paths: &[PathBuf], packs: &[Pack]) -> Result<Vec<PathBuf>> {
    let languages: HashSet<SupportLang> = (packs.iter())
        .flat_map(|pack| pack.rules.iter().map(|rule| rule.config.language))
        .filter(|language| *language != SupportLang::Python)
        .collect();
    files_under(paths, |path| {
        is_python(path) || SupportLang::from_path(path).is_some_and(|l| languages.contains(&l))
    })
}

/// The Python files under `paths`, each once. Folders respect `.gitignore`.
pub(crate) fn python_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    files_under(paths, is_python)
}

/// The files under `paths` that `wanted` keeps, each once. A path that is a file is always
/// taken.
fn files_under(paths: &[PathBuf], wanted: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |path: &Path| -> Result<()> {
        let real = fs::canonicalize(path).with_context(|| cannot_read(path))?;
        if seen.insert(real) {
            files.push(path.to_path_buf());
        }
        Ok(())
    };
    for p in paths {
        let meta = fs::metadata(p).with_context(|| cannot_read(p))?;
        if meta.is_file() {
            add(p)?;
            continue;
        }
        for entry in ignore::WalkBuilder::new(p).build() {
            let entry = entry.with_context(|| cannot_read(p))?;
            if entry.file_type().is_some_and(|t| t.is_file()) && wanted(entry.path()) {
                add(entry.path())?;
            }
        }
    }
    Ok(files)
}

/// What a scan found, after any fixes it wrote.
#[derive(Debug)]
pub(crate) struct Scanned {
    pub(crate) issues: Vec<Issue>,
    /// Number of fixes written to disk.
    pub(crate) fixed: usize,
}

/// A fix can create a match for another fix, so fixing repeats until nothing changes, at
/// most this many times per file.
const MAX_FIX_PASSES: usize = 100;

/// Lints each file, after applying fixes up to `fix` when it is set and writing them back.
pub(crate) fn scan(
    files: &[PathBuf],
    packs: &[Pack],
    fix: Option<Applicability>,
) -> Result<Scanned> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let per_file: Vec<(Vec<Issue>, usize)> = files
        .par_iter()
        .map(|path| {
            let mut src = fs::read_to_string(path).with_context(|| cannot_read(path))?;
            let mut fixed = 0;
            let file = File::on_disk(path);
            if let Some(allowed) = fix {
                (src, fixed) = fix_source(file, src, packs, allowed);
                if fixed > 0 {
                    fs::write(path, &src)
                        .with_context(|| format!("cannot write {}", path.display()))?;
                }
            }
            let issues = lint_source(file, &src, packs)
                .into_iter()
                .map(|found| found.issue)
                .collect();
            Ok((issues, fixed))
        })
        .collect::<Result<_>>()?;
    let fixed = per_file.iter().map(|(_, n)| n).sum();
    let mut issues: Vec<Issue> = per_file.into_iter().flat_map(|(i, _)| i).collect();
    issues.sort_by(|a, b| (&a.file, a.position).cmp(&(&b.file, b.position)));
    Ok(Scanned { issues, fixed })
}

struct MatchedRule<'p, 'r> {
    pack: &'p Pack,
    rule: &'p Rule,
    matches: Vec<NodeMatch<'r, StrDoc<SupportLang>>>,
}

/// The rules of `packs` that report in the parsed file `root` of `language`, each with its
/// matches. One walk of the tree, in ast-grep's scanner, matches every selected rule and drops
/// the matches that a suppression comment names.
fn matched_rules<'p, 'r>(
    root: &'r Root,
    src: &str,
    file: File<'_>,
    language: SupportLang,
    packs: &'p [Pack],
) -> Vec<MatchedRule<'p, 'r>> {
    let mut selected: Vec<_> = packs
        .iter()
        .filter(|pack| pack.applies(root, src, language))
        .flat_map(|pack| {
            let path = pack.glob_path(file);
            pack.rules
                .iter()
                .filter(move |rule| rule.config.language == language && rule.runs_on(&path))
                .map(move |rule| (pack, rule))
        })
        .collect();
    let mut combined = CombinedScan::new(selected.iter().map(|(_, rule)| &rule.config).collect());
    let unused = packs
        .iter()
        .find_map(|pack| pack.unused_suppression().map(|rule| (pack, rule)));
    if let Some((_, rule)) = unused {
        combined.set_unused_suppression_rule(&rule.config);
    }
    selected.extend(unused);
    let mut matches: HashMap<_, _> = combined
        .scan(root, false)
        .matches
        .into_iter()
        .map(|(config, matches)| (std::ptr::from_ref(config), matches))
        .collect();
    selected
        .into_iter()
        .filter_map(|(pack, rule)| {
            let raw = matches
                .remove(&std::ptr::from_ref(&rule.config))
                .unwrap_or_default();
            let matches = rule.retain_matches(root, raw, file.disk, pack.library_in(root));
            (!matches.is_empty()).then_some(MatchedRule {
                pack,
                rule,
                matches,
            })
        })
        .collect()
}

/// One issue with the place of its match in the source, which a caller that edits the text
/// needs, as the language server does.
#[derive(Debug)]
pub(crate) struct Found {
    pub(crate) issue: Issue,
    pub(crate) labels: Vec<Label>,
    pub(crate) variant: Option<crate::rule::VariantId>,
    /// The bytes of the source that the rule matched.
    pub(crate) range: Range<usize>,
    /// What the rule's fix writes for this match. `None` when the issue has no fix.
    pub(crate) fix: Option<Replacement>,
}

#[derive(Debug)]
pub(crate) struct Label {
    pub(crate) range: Range<usize>,
    pub(crate) style: ast_grep_config::LabelStyle,
    pub(crate) message: Option<String>,
}

/// New text for a range of bytes of the source.
#[derive(Debug)]
pub(crate) struct Replacement {
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

/// `src` after every fix up to `allowed`, with the number of fixes applied. `file` is the file
/// the source belongs to.
pub(crate) fn fix_source(
    file: File<'_>,
    mut src: String,
    packs: &[Pack],
    allowed: Applicability,
) -> (String, usize) {
    let mut fixed = 0;
    for _ in 0..MAX_FIX_PASSES {
        let (next, applied) = fix_pass(file, &src, packs, allowed);
        if applied == 0 {
            break;
        }
        src = next;
        fixed += applied;
    }
    (src, fixed)
}

/// The issues in one file's source, in the order of their positions. `file` is the file the
/// source belongs to.
pub(crate) fn lint_source(file: File<'_>, src: &str, packs: &[Pack]) -> Vec<Found> {
    let language = language_of(file.path);
    if !packs.iter().any(|pack| pack.may_apply(src, language)) {
        return Vec::new();
    }
    let root = parse_as(src, language);
    let imports = aliased_imports(&root, language);
    let mut found = Vec::new();
    for MatchedRule {
        pack,
        rule,
        matches,
    } in matched_rules(&root, src, file, language, packs)
    {
        let unbound = unbound_aliases(pack, imports.as_deref());
        for m in matches {
            let start = m.start_pos();
            let edit = usable_edit(rule, &m, &unbound, src);
            let issue = Issue {
                package: pack.module.clone(),
                rule: rule.id.clone(),
                file: file.path.to_path_buf(),
                position: Some(Position {
                    line: start.line() + 1,
                    column: start.column(&m) + 1,
                }),
                severity: rule.config.severity.clone(),
                message: rule.config.get_message(&m),
                fix: rule.fix.filter(|_| edit.is_some()),
            };
            found.push(Found {
                variant: Some(rule.variant.clone()),
                issue,
                labels: rule
                    .config
                    .get_labels(&m)
                    .into_iter()
                    .map(|label| Label {
                        range: label.range(),
                        style: label.style,
                        message: label.message.map(str::to_owned),
                    })
                    .collect(),
                range: m.range(),
                fix: edit.map(|edit| Replacement {
                    range: edit.position..edit.position + edit.deleted_length,
                    text: String::from_utf8(edit.inserted_text).expect("a fix writes UTF-8 text"),
                }),
            });
        }
    }
    // A stable sort, so issues at one position keep the order of their packs and rules.
    found.sort_by_key(|found| found.issue.position);
    found
}

/// Applies every fix up to `allowed` that does not overlap an earlier one, and returns the new
/// source with the number of fixes applied. Overlapping fixes wait for the next pass.
fn fix_pass(file: File<'_>, src: &str, packs: &[Pack], allowed: Applicability) -> (String, usize) {
    let language = language_of(file.path);
    if !packs.iter().any(|pack| pack.may_apply(src, language)) {
        return (src.to_owned(), 0);
    }
    let root = parse_as(src, language);
    let imports = aliased_imports(&root, language);
    let mut edits = Vec::new();
    for MatchedRule {
        pack,
        rule,
        matches,
    } in matched_rules(&root, src, file, language, packs)
    {
        let unbound = unbound_aliases(pack, imports.as_deref());
        if rule.fix.is_none_or(|a| a > allowed) {
            continue;
        }
        for m in matches {
            edits.extend(usable_edit(rule, &m, &unbound, src));
        }
    }
    edits.sort_by_key(|e| e.position);
    let mut out = Vec::with_capacity(src.len());
    let mut copied = 0;
    let mut applied = 0;
    for edit in edits {
        if edit.position < copied {
            continue;
        }
        out.extend_from_slice(&src.as_bytes()[copied..edit.position]);
        out.extend_from_slice(&edit.inserted_text);
        copied = edit.position + edit.deleted_length;
        applied += 1;
    }
    out.extend_from_slice(&src.as_bytes()[copied..]);
    let out = String::from_utf8(out).expect("fixes splice UTF-8 text at node boundaries");
    (out, applied)
}

/// Every aliased import in the file, at any depth. `None` for a file that is not Python, where
/// no alias of a pack needs an import.
fn aliased_imports(root: &Root, language: SupportLang) -> Option<Vec<AliasedImport>> {
    if language != SupportLang::Python {
        return None;
    }
    let imports = root
        .root()
        .dfs()
        .filter(|node| node.kind() == "aliased_import")
        .filter_map(|node| {
            Some(AliasedImport {
                module: node.field("name")?.text().into_owned(),
                alias: node.field("alias")?.text().into_owned(),
            })
        })
        .collect();
    Some(imports)
}

/// The pack's aliases that the file does not bind to their module. A fix that writes `pd.`
/// applies only in a file with `import pandas as pd`.
fn unbound_aliases<'p>(pack: &'p Pack, imports: Option<&[AliasedImport]>) -> Vec<&'p str> {
    let Some(imports) = imports else {
        return Vec::new();
    };
    pack.aliases
        .iter()
        .filter(|(alias, module)| {
            !imports
                .iter()
                .any(|import| import.module == **module && import.alias == **alias)
        })
        .map(|(alias, _)| alias.as_str())
        .collect()
}

/// The edit `rule` makes for `m` in `src`, or `None` when the rule has no fix, its fix writes
/// one of the `unbound` aliases, or its fix leaves the text as it is.
fn usable_edit(
    rule: &Rule,
    m: &NodeMatch<'_, StrDoc<SupportLang>>,
    unbound: &[&str],
    src: &str,
) -> Option<Edit<String>> {
    let fixer = rule.config.fixer.first()?;
    let edit = m.make_edit(&rule.config.matcher, fixer);
    let writes_unbound = unbound
        .iter()
        .any(|alias| writes_name(&edit.inserted_text, alias));
    let old = &src.as_bytes()[edit.position..edit.position + edit.deleted_length];
    (!writes_unbound && edit.inserted_text != old).then_some(edit)
}

/// Whether `text` uses `name` as the start of a dotted name, such as `pd.col`.
fn writes_name(text: &[u8], name: &str) -> bool {
    let needle = format!("{name}.");
    text.windows(needle.len()).enumerate().any(|(i, w)| {
        w == needle.as_bytes()
            && (i == 0
                || !(text[i - 1].is_ascii_alphanumeric() || matches!(text[i - 1], b'_' | b'.')))
    })
}
