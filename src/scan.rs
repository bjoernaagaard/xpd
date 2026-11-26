//! Walks paths, fixes and lints each file with the rule packs that apply to it.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ast_grep_config::CombinedScan;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{Language, NodeMatch};
use ast_grep_language::SupportLang;

use crate::fix_safety::Applicability;
use crate::pack::Pack;
use crate::pack_names::OWN_PACK;
use crate::package_check::Calls;
use crate::report::{Issue, Position};
use crate::rule::Rule;
use crate::rule_id::RuleId;
use crate::severity::Severity;
use crate::source::{File, files_under, is_python, language_of, read_source};
use crate::syntax::{Node, Root, parse_as, parses_python_cells};

/// How a file binds a name that it imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Binding {
    /// `import <module> as <name>` (or `import <module>` when `name == module`), used as a dotted prefix.
    Alias,
    /// `from <module> import <name>`, which a fix uses bare.
    Member,
}

/// One name that a file imports from `module` under its own name or as an alias.
#[derive(Debug)]
struct Import {
    module: String,
    name: String,
    binding: Binding,
}

/// A name that a fix of a pack may write, which the file does not bind to its module.
#[derive(Debug, Clone, Copy)]
struct Unbound<'p> {
    name: &'p str,
    module: &'p str,
    binding: Binding,
}

impl Unbound<'_> {
    fn import_statement(&self) -> String {
        match self.binding {
            Binding::Alias if self.name == self.module => format!("import {}", self.module),
            Binding::Alias => format!("import {} as {}", self.module, self.name),
            Binding::Member => format!("from {} import {}", self.module, self.name),
        }
    }
}

/// The number of syntax errors in a parsed file.
fn syntax_errors(root: &Root) -> usize {
    let top = root.root();
    if !top.get_inner_node().has_error() {
        return 0;
    }
    top.dfs()
        .filter(|node| node.is_error() || node.is_missing())
        .count()
}

/// The files under `paths` that `packs` have rules for, each once: the Python files, and the
/// files of each other language that a rule names. A path that is a file is always taken.
/// Folders respect `.gitignore`.
pub(crate) fn collect_files(
    root: &Path,
    paths: &[PathBuf],
    packs: &[Pack],
) -> Result<Vec<PathBuf>> {
    let languages: HashSet<SupportLang> = (packs.iter())
        .flat_map(|pack| pack.rules.iter().map(|rule| rule.config.language))
        .filter(|language| *language != SupportLang::Python)
        .collect();
    files_under(root, paths, |path| {
        is_python(path) || SupportLang::from_path(path).is_some_and(|l| languages.contains(&l))
    })
}

/// What a scan found, after any fixes it wrote.
#[derive(Debug)]
pub(crate) struct Scanned {
    pub(crate) issues: Vec<Issue>,
    /// Number of fixes written to disk.
    pub(crate) fixed: usize,
    /// Number of files that could not be read.
    pub(crate) skipped: usize,
    /// Files where autofix stopped at the pass limit.
    pub(crate) pass_limit: Vec<PathBuf>,
    /// What the `onlyWhere` shapes of the package checks matched.
    pub(crate) calls: Calls,
}

/// A fix can create a match for another fix, so fixing repeats until nothing changes, at
/// most this many times per file.
const MAX_FIX_PASSES: usize = 100;

/// Lints each file, after applying fixes up to `fix` when it is set and writing them back.
pub(crate) fn scan(
    root: &Path,
    files: &[PathBuf],
    packs: &[Pack],
    fix: Option<Applicability>,
) -> Result<Scanned> {
    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

    let per_file: Vec<Option<(Linted, usize, Option<PathBuf>)>> = files
        .par_iter()
        .map(|path| {
            let disk = root.join(path);
            let Some(mut src) = read_source(File {
                path,
                disk: Some(&disk),
            }) else {
                return Ok(None);
            };
            let mut fixed = 0;
            let mut hit_limit = None;
            let file = File {
                path,
                disk: Some(&disk),
            };
            if let Some(allowed) = fix {
                match fix_source(file, src, packs, allowed) {
                    Fixed::Converged { source, count } => {
                        src = source;
                        fixed = count;
                    }
                    Fixed::PassLimit { source, count } => {
                        src = source;
                        fixed = count;
                        hit_limit = Some(path.clone());
                    }
                }
                if fixed > 0 {
                    fs::write(&disk, &src)
                        .with_context(|| format!("cannot write {}", path.display()))?;
                }
            }
            Ok(Some((
                analyze(file, &src, packs, Edits::Report),
                fixed,
                hit_limit,
            )))
        })
        .collect::<Result<_>>()?;
    let skipped = per_file.iter().filter(|file| file.is_none()).count();
    let mut scanned = Scanned {
        issues: Vec::new(),
        fixed: 0,
        skipped,
        pass_limit: Vec::new(),
        calls: Calls::default(),
    };
    for (linted, fixed, hit_limit) in per_file.into_iter().flatten() {
        scanned.fixed += fixed;
        if let Some(path) = hit_limit {
            scanned.pass_limit.push(path);
        }
        scanned
            .issues
            .extend(linted.found.into_iter().map(Found::into_issue));
        scanned.calls.merge(linted.calls);
    }
    scanned.pass_limit.sort();
    (scanned.issues).sort_by(|a, b| (&a.file, a.position).cmp(&(&b.file, b.position)));
    Ok(scanned)
}

#[derive(Clone, Copy)]
struct Suppressions<'p> {
    present: bool,
    unused: Option<(&'p Pack, &'p Rule)>,
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
    file: File<'_>,
    language: SupportLang,
    applicable: &[&'p Pack],
    suppressions: Suppressions<'p>,
) -> Vec<MatchedRule<'p, 'r>> {
    let mut selected = Vec::new();
    let mut inactive = Vec::new();
    for &pack in applicable {
        let path = pack.glob_path(file);
        let runs_here = |rule: &&Rule| rule.config.language == language && rule.runs_on(&path);
        selected.extend(pack.rules.iter().filter(runs_here).map(|rule| (pack, rule)));
        if suppressions.present {
            inactive.extend(
                pack.inactive_rules
                    .iter()
                    .filter(runs_here)
                    .map(|rule| &rule.config),
            );
        }
    }
    let mut combined = CombinedScan::new(
        selected
            .iter()
            .map(|(_, rule)| &rule.config)
            .chain(inactive)
            .collect(),
    );
    let unused = suppressions.unused;
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
    issue: Issue,
    pub(crate) labels: Vec<Label>,
    /// The bytes of the source that the rule matched.
    pub(crate) range: Range<usize>,
    /// What the rule's fix writes for this match. `None` when the issue has no fix.
    pub(crate) fix: Option<Fix>,
}

impl Found {
    pub(crate) fn id(&self) -> &RuleId {
        &self.issue.id
    }

    pub(crate) fn issue(&self) -> Issue {
        Issue {
            fix: self.fix.as_ref().map(|fix| fix.level),
            ..self.issue.clone()
        }
    }

    pub(crate) fn into_issue(self) -> Issue {
        Issue {
            fix: self.fix.map(|fix| fix.level),
            ..self.issue
        }
    }

    pub(crate) fn invalid_rule(file: PathBuf, text: &str, message: String) -> Self {
        Self {
            labels: Vec::new(),
            issue: Issue {
                id: RuleId::qualified(OWN_PACK, "invalid-rule"),
                category: crate::category::RuleCategory::Bundled,
                file,
                position: Some(Position { line: 1, column: 1 }),
                severity: Severity::Error,
                message,
                fix: None,
                help: None,
            },
            range: 0..text.lines().next().map_or(0, str::len),
            fix: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Label {
    pub(crate) range: Range<usize>,
    pub(crate) style: ast_grep_config::LabelStyle,
    pub(crate) message: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Fix {
    pub(crate) level: Applicability,
    pub(crate) replacement: Replacement,
    pub(crate) import_offset: usize,
    imports: BTreeMap<String, String>,
}

impl Fix {
    /// Every nonoverlapping edit that applies this fix to `src`, in source order.
    pub(crate) fn edits(&self, src: &str) -> Vec<Replacement> {
        let mut out = Vec::with_capacity(2);
        if !self.imports.is_empty() {
            let lines: Vec<&str> = self.imports.values().map(String::as_str).collect();
            out.push(import_replacement(src, self.import_offset, &lines));
        }
        out.push(self.replacement.clone());
        out.sort_by_key(|edit| (edit.range.start, edit.range.end));
        out
    }

    /// `src` with this fix and any imports it needs applied.
    pub(crate) fn apply(&self, src: &str) -> String {
        let edits = self.edits(src);
        let refs: Vec<&Replacement> = edits.iter().collect();
        splice(src, &refs)
    }
}

/// New text for a range of bytes of the source.
#[derive(Debug, Clone)]
pub(crate) struct Replacement {
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) enum Fixed {
    Converged { source: String, count: usize },
    PassLimit { source: String, count: usize },
}

pub(crate) fn accepts_rewrite(path: &Path, before: &str, after: &str) -> bool {
    let language = language_of(path);
    let root = parse_as(before, language);
    Parsed::new(before, language, &root).accepts(after)
}

fn validated_batch(
    language: SupportLang,
    source: &str,
    proposals: &[&Fix],
) -> Option<(String, usize)> {
    if proposals.is_empty() {
        return None;
    }
    let root = parse_as(source, language);
    let parsed = Parsed::new(source, language, &root);
    let candidate = apply_fixes(source, proposals);
    if parsed.accepts(&candidate) {
        return Some((candidate, proposals.len()));
    }
    let mut kept = Vec::new();
    let mut current = source.to_owned();
    for &fix in proposals {
        kept.push(fix);
        let next = apply_fixes(source, &kept);
        if parsed.accepts(&next) {
            current = next;
        } else {
            kept.pop();
        }
    }
    (!kept.is_empty()).then_some((current, kept.len()))
}

pub(crate) fn fix_source(
    file: File<'_>,
    mut source: String,
    packs: &[Pack],
    allowed: Applicability,
) -> Fixed {
    let language = language_of(file.path);
    let mut count = 0;
    for _ in 0..MAX_FIX_PASSES {
        let analysis = analyze(file, &source, packs, Edits::Apply(allowed));
        let mut edits = EditBatch::default();
        for (_, fix) in analysis.proposals(allowed) {
            if edits.fits(fix) {
                edits.push(fix);
            }
        }
        let Some((next, applied)) = validated_batch(language, &source, &edits.0) else {
            return Fixed::Converged { source, count };
        };
        count += applied;
        source = next;
    }
    Fixed::PassLimit { source, count }
}

/// What one file's source holds for a run.
#[derive(Debug, Default)]
pub(crate) struct Linted {
    /// The issues, in the order of their positions.
    pub(crate) found: Vec<Found>,
    /// What the `onlyWhere` shapes of the package checks matched.
    pub(crate) calls: Calls,
}

impl Linted {
    pub(crate) fn proposals(&self, allowed: Applicability) -> Vec<(&Found, &Fix)> {
        let mut edits: Vec<_> = self
            .found
            .iter()
            .filter_map(|found| {
                let fix = found.fix.as_ref().filter(|fix| fix.level <= allowed)?;
                Some((found, fix))
            })
            .collect();
        edits.sort_by_key(|(_, fix)| fix.replacement.range.start);
        edits
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Edits {
    Report,
    Apply(Applicability),
}

impl Edits {
    fn includes(self, level: Applicability) -> bool {
        match self {
            Self::Report => true,
            Self::Apply(allowed) => level <= allowed,
        }
    }
}

pub(crate) fn analyze(file: File<'_>, src: &str, packs: &[Pack], edits: Edits) -> Linted {
    let language = language_of(file.path);
    if !packs.iter().any(|pack| pack.may_apply(src, language)) {
        return Linted::default();
    }
    let root = parse_as(src, language);
    let applicable: Vec<_> = packs
        .iter()
        .filter(|pack| pack.applies(&root, src, language))
        .collect();
    let suppressions = Suppressions {
        present: src.contains("ast-grep-ignore"),
        unused: packs
            .iter()
            .find_map(|pack| pack.unused_suppression().map(|rule| (pack, rule))),
    };
    let calls = match edits {
        Edits::Report => Calls::find(file.path, &root, &applicable),
        Edits::Apply(_) => Calls::default(),
    };
    let imports = imports(&root, language);
    let context = FixContext {
        source: src,
        import_offset: OnceCell::new(),
    };
    let parsed = Parsed::new(src, language, &root);
    let mut found = Vec::new();
    for MatchedRule {
        pack,
        rule,
        matches,
    } in matched_rules(&root, file, language, &applicable, suppressions)
    {
        let level = rule.fix.filter(|level| edits.includes(*level));
        if matches!(edits, Edits::Apply(_)) && level.is_none() {
            continue;
        }
        let unbound = unbound_names(pack, imports.as_deref());
        for m in matches {
            let start = m.start_pos();
            let fix = level.and_then(|level| context.build(rule, &m, pack, &unbound, level));
            let issue = Issue {
                id: rule.id.clone(),
                category: pack.rule_category(),
                file: file.path.to_path_buf(),
                position: Some(Position {
                    line: start.line() + 1,
                    column: start.column(&m) + 1,
                }),
                severity: rule.severity_under(pack.style),
                message: rule.config.get_message(&m),
                fix: None,
                help: pack
                    .page(rule.id.name())
                    .or_else(|| rule.config.url.clone()),
            };
            found.push(Found {
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
                fix,
            });
        }
    }
    parsed.validate(found.iter_mut().map(|finding| &mut finding.fix));
    if matches!(edits, Edits::Report) {
        found.sort_by_key(|found| found.issue.position);
    }
    Linted { found, calls }
}

/// Every import in the file that binds a name a fix can write anywhere in the file. An import
/// inside a function or a class binds the name only there, so it does not count. `None` for a
/// file that is not Python, where no name of a pack needs an import.
fn imports(root: &Root, language: SupportLang) -> Option<Vec<Import>> {
    if language != SupportLang::Python {
        return None;
    }
    let mut imports = Vec::new();
    for node in root.root().dfs() {
        let local = |node: &Node<'_>| {
            node.ancestors()
                .any(|outer| matches!(&*outer.kind(), "function_definition" | "class_definition"))
        };
        match &*node.kind() {
            "import_statement" | "aliased_import" | "import_from_statement" if local(&node) => {}
            "import_statement" => {
                for child in node.field_children("name") {
                    if child.kind() == "dotted_name" {
                        let module = child.text().into_owned();
                        if !module.contains('.') {
                            imports.push(Import {
                                name: module.clone(),
                                module,
                                binding: Binding::Alias,
                            });
                        }
                    }
                }
            }
            "aliased_import" => {
                let (Some(module), Some(alias)) = (node.field("name"), node.field("alias")) else {
                    continue;
                };
                // `from m import a as b` binds neither `a` nor the module `m` under an alias.
                if node
                    .parent()
                    .is_some_and(|p| p.kind() == "import_statement")
                {
                    imports.push(Import {
                        module: module.text().into_owned(),
                        name: alias.text().into_owned(),
                        binding: Binding::Alias,
                    });
                }
            }
            "import_from_statement" => {
                let Some(module) = node.field("module_name") else {
                    continue;
                };
                let members = node
                    .field_children("name")
                    .filter(|name| name.kind() == "dotted_name");
                for member in members {
                    imports.push(Import {
                        module: module.text().into_owned(),
                        name: member.text().into_owned(),
                        binding: Binding::Member,
                    });
                }
            }
            _ => {}
        }
    }
    Some(imports)
}

/// Byte offset in a Python module where new module-level imports belong: immediately after the
/// leading import block, or after any leading shebang, encoding comment, module docstring, and
/// `from __future__` imports.
fn import_insertion_offset(root: &Root, src: &str) -> Option<usize> {
    let clean_line_end = |byte: usize| -> Option<usize> {
        let next_nl = src[byte..].find('\n').map_or(src.len(), |d| byte + d);
        let rest = src[byte..next_nl].trim_start();
        if rest.is_empty() || rest.starts_with('#') {
            Some(if next_nl < src.len() {
                next_nl + 1
            } else {
                src.len()
            })
        } else {
            None
        }
    };
    let mut after_preamble = if src.starts_with('\u{feff}') { 3 } else { 0 };
    let mut after_imports: Option<usize> = None;
    let mut saw_non_comment = false;
    for child in root.root().children() {
        let kind = child.kind();
        match &*kind {
            "comment" => {
                if !saw_non_comment && after_imports.is_none() {
                    let text = child.text();
                    if (text.starts_with("#!") || text.contains("coding"))
                        && let Some(end) = clean_line_end(child.range().end)
                    {
                        after_preamble = end;
                    }
                }
            }
            "expression_statement" if !saw_non_comment && docstring(&child) => {
                saw_non_comment = true;
                after_preamble = clean_line_end(child.range().end)?;
            }
            "future_import_statement" => {
                saw_non_comment = true;
                after_preamble = clean_line_end(child.range().end)?;
            }
            "import_statement" | "import_from_statement" => {
                saw_non_comment = true;
                let Some(end) = clean_line_end(child.range().end) else {
                    break;
                };
                after_imports = Some(end);
            }
            _ => break,
        }
    }
    Some(after_imports.unwrap_or(after_preamble))
}

fn docstring(statement: &Node<'_>) -> bool {
    let Some(mut value) = statement.named_children().next() else {
        return false;
    };
    while value.kind() == "parenthesized_expression" {
        let Some(inside) = value.named_children().find(|node| node.kind() != "comment") else {
            return false;
        };
        value = inside;
    }
    matches!(&*value.kind(), "string" | "concatenated_string")
}

fn import_replacement(src: &str, offset: usize, lines: &[&str]) -> Replacement {
    let mut text = String::new();
    let newline = if src
        .split_once('\n')
        .is_some_and(|(first, _)| first.ends_with('\r'))
    {
        "\r\n"
    } else {
        "\n"
    };
    if !src[..offset].trim_start_matches('\u{feff}').is_empty() && !src[..offset].ends_with('\n') {
        text.push_str(newline);
    }
    for line in lines {
        text.push_str(line);
        text.push_str(newline);
    }
    Replacement {
        range: offset..offset,
        text,
    }
}

/// The names of the pack that the file does not bind to their module.
fn unbound_names<'p>(pack: &'p Pack, imports: Option<&[Import]>) -> Vec<Unbound<'p>> {
    let Some(imports) = imports else {
        return Vec::new();
    };
    let names = |names: &'p BTreeMap<String, String>, binding: fn() -> Binding| {
        names
            .iter()
            .filter(move |(name, module)| {
                !imports.iter().any(|import| {
                    import.binding == binding()
                        && import.module == **module
                        && import.name == **name
                })
            })
            .map(move |(name, module)| Unbound {
                name,
                module,
                binding: binding(),
            })
    };
    names(&pack.aliases, || Binding::Alias)
        .chain(names(&pack.members, || Binding::Member))
        .collect()
}

/// How often Python code imports from a module a name that the same block has imported from it
/// already, in the same statement or in an earlier one.
fn repeated_imports(code: &str) -> usize {
    let root = crate::syntax::parse(code);
    let mut seen = HashSet::new();
    let mut repeats = 0;
    for import in root.root().dfs() {
        if import.kind() != "import_from_statement" {
            continue;
        }
        let block = import.parent().map(|parent| parent.range().start);
        let module = import
            .field("module_name")
            .map(|module| module.text().into_owned());
        for name in import.field_children("name") {
            if !seen.insert((block, module.clone(), name.text().into_owned())) {
                repeats += 1;
            }
        }
    }
    repeats
}

/// The source of one file as a fix sees it.
struct Parsed<'s> {
    src: &'s str,
    language: SupportLang,
    /// The number of syntax errors that tree-sitter finds in `src`.
    errors: usize,
    /// Whether libcst parses `src` as Python, found out for the first fix that asks.
    python: OnceCell<bool>,
    /// How often `src` imports a name that it has imported already ([`repeated_imports`]).
    repeats: OnceCell<usize>,
}

impl<'s> Parsed<'s> {
    fn new(src: &'s str, language: SupportLang, root: &Root) -> Self {
        Self {
            src,
            language,
            errors: syntax_errors(root),
            python: OnceCell::new(),
            repeats: OnceCell::new(),
        }
    }

    /// Whether `after`, the file with some fixes applied, is still valid code. Python is parsed
    /// with libcst, which reads the grammar of Python itself, so a file that libcst cannot
    /// parse before the fixes gets no fix. The magic lines of a file that runs cell by cell are
    /// hidden from libcst first ([`hide_magics`]). tree-sitter recovers from errors that Python does
    /// not accept, so it is the check only for the other languages, which libcst does not read.
    /// A fix that renames an imported name to one the file imports already would import it
    /// twice, so Python code may not repeat more imports than it did.
    fn accepts(&self, after: &str) -> bool {
        if self.language == SupportLang::Python {
            *self.python.get_or_init(|| parses_python_cells(self.src))
                && parses_python_cells(after)
                && repeated_imports(after)
                    <= *self.repeats.get_or_init(|| repeated_imports(self.src))
        } else {
            syntax_errors(&parse_as(after, self.language)) <= self.errors
        }
    }

    /// Validate nonoverlapping edits together, bisecting each group that does not parse.
    fn validate<'f>(&self, fixes: impl Iterator<Item = &'f mut Option<Fix>>) {
        let mut ordered: Vec<_> = fixes.filter(|slot| slot.is_some()).collect();
        ordered.sort_by_key(|slot| {
            let r = &fix_in(slot.as_ref()).replacement.range;
            (r.start, r.end)
        });
        let mut layers: Vec<Vec<&mut Option<Fix>>> = Vec::new();
        for slot in ordered {
            let fix = fix_in(slot.as_ref());
            let fits = |layer: &&mut Vec<&mut Option<Fix>>| {
                fits_after(layer.iter().map(|prev| fix_in(prev.as_ref())), fix)
            };
            match layers.iter_mut().find(fits) {
                Some(layer) => layer.push(slot),
                None => layers.push(vec![slot]),
            }
        }
        for mut layer in layers {
            self.validate_group(&mut layer);
        }
    }

    fn validate_group(&self, group: &mut [&mut Option<Fix>]) {
        let applied: Vec<_> = group.iter().map(|slot| fix_in(slot.as_ref())).collect();
        if self.accepts(&apply_fixes(self.src, &applied)) {
            return;
        }
        if let [slot] = group {
            **slot = None;
        } else {
            let (first, second) = group.split_at_mut(group.len() / 2);
            self.validate_group(first);
            self.validate_group(second);
        }
    }
}

fn fix_in(slot: Option<&Fix>) -> &Fix {
    slot.expect("validation groups contain edits")
}

fn nonoverlapping(prev: &Fix, next: &Fix) -> bool {
    if prev.replacement.range.end > next.replacement.range.start {
        return false;
    }
    if !next.imports.is_empty()
        && prev.replacement.range.start < next.import_offset
        && next.import_offset < prev.replacement.range.end
    {
        return false;
    }
    prev.imports
        .iter()
        .all(|(name, line)| next.imports.get(name).is_none_or(|other| line == other))
}

fn fits_after<'a>(mut previous: impl DoubleEndedIterator<Item = &'a Fix>, next: &Fix) -> bool {
    previous
        .next_back()
        .is_none_or(|last| nonoverlapping(last, next))
        && (next.imports.is_empty() || previous.all(|fix| nonoverlapping(fix, next)))
}

#[derive(Default)]
pub(crate) struct EditBatch<'s>(Vec<&'s Fix>);

impl<'s> EditBatch<'s> {
    pub(crate) fn fits(&self, fix: &Fix) -> bool {
        fits_after(self.0.iter().copied(), fix)
    }

    pub(crate) fn push(&mut self, fix: &'s Fix) {
        self.0.push(fix);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn apply(&self, source: &str) -> String {
        apply_fixes(source, &self.0)
    }
}

/// `src` with `fixes` and their deduplicated module-level imports applied.
fn apply_fixes(src: &str, fixes: &[&Fix]) -> String {
    let mut by_offset: BTreeMap<usize, BTreeSet<&str>> = BTreeMap::new();
    for fix in fixes {
        if !fix.imports.is_empty() {
            let entry = by_offset.entry(fix.import_offset).or_default();
            entry.extend(fix.imports.values().map(String::as_str));
        }
    }
    let import_edits: Vec<Replacement> = by_offset
        .into_iter()
        .map(|(offset, lines)| {
            let ordered: Vec<&str> = lines.into_iter().collect();
            import_replacement(src, offset, &ordered)
        })
        .collect();
    let mut all: Vec<&Replacement> = Vec::with_capacity(fixes.len() + import_edits.len());
    for edit in &import_edits {
        all.push(edit);
    }
    for fix in fixes {
        all.push(&fix.replacement);
    }
    all.sort_by_key(|edit| (edit.range.start, edit.range.end));
    splice(src, &all)
}

/// `src` with `fixes` applied, which do not overlap and are in the order of their positions.
fn splice(src: &str, fixes: &[&Replacement]) -> String {
    let mut out = String::with_capacity(src.len());
    let mut copied = 0;
    for fix in fixes {
        out.push_str(&src[copied..fix.range.start]);
        out.push_str(&fix.text);
        copied = fix.range.end;
    }
    out.push_str(&src[copied..]);
    out
}

struct FixContext<'s> {
    source: &'s str,
    import_offset: OnceCell<Option<usize>>,
}

impl FixContext<'_> {
    fn build(
        &self,
        rule: &Rule,
        matched: &NodeMatch<'_, StrDoc<SupportLang>>,
        pack: &Pack,
        unbound: &[Unbound<'_>],
        level: Applicability,
    ) -> Option<Fix> {
        let fixer = rule.config.fixer.first()?;
        let edit = matched.make_edit(&rule.config.matcher, fixer);
        let range = edit.position..edit.position + edit.deleted_length;
        let original = &self.source.as_bytes()[range.clone()];
        if edit.inserted_text == original {
            return None;
        }
        if unbound.iter().any(|name| {
            !rule
                .fix_imports
                .iter()
                .any(|requested| name.name == requested)
                && name.written_in(&edit.inserted_text)
        }) {
            return None;
        }
        let mut imports = BTreeMap::new();
        let mut import_offset = 0;
        for requested in &rule.fix_imports {
            let (binding, module) = if let Some(module) = pack.members.get(requested) {
                (Binding::Member, module)
            } else {
                (Binding::Alias, pack.aliases.get(requested)?)
            };
            let name = Unbound {
                name: requested,
                module,
                binding,
            };
            if !name.written_in(&edit.inserted_text) {
                continue;
            }
            let root = matched.root();
            if requires_import(root, &name, range.start)? {
                import_offset = (*self
                    .import_offset
                    .get_or_init(|| import_insertion_offset(root, self.source)))?;
                if import_offset > range.start {
                    return None;
                }
                imports.insert(requested.clone(), name.import_statement());
            }
        }
        let text = String::from_utf8(edit.inserted_text).expect("a fix writes UTF-8 text");
        Some(Fix {
            level,
            replacement: Replacement { range, text },
            import_offset,
            imports,
        })
    }
}

/// Reuse a direct module import that precedes the fix and is the only binding of the name.
/// Otherwise insert an import only when the name is unused throughout the file.
fn requires_import(root: &Root, name: &Unbound<'_>, before: usize) -> Option<bool> {
    let expected = match name.binding {
        Binding::Alias => name.module.to_owned(),
        Binding::Member => format!("{}.{}", name.module, name.name),
    };
    let mut imported = false;
    let mut seen = false;
    for node in root.root().dfs() {
        if node.kind() == "wildcard_import" {
            return None;
        }
        if node.kind() != "identifier" || node.text() != name.name {
            continue;
        }
        seen = true;
        if !crate::names::is_bound(&node) && !crate::names::is_parameter_name(&node) {
            continue;
        }
        let import = node.ancestors().find(|parent| {
            matches!(
                &*parent.kind(),
                "import_statement" | "import_from_statement"
            )
        });
        if !import.is_some_and(|stmt| {
            stmt.range().end <= before && stmt.parent().is_some_and(|p| p.kind() == "module")
        }) || crate::names::import_binding(&node).as_deref() != Some(&expected)
        {
            return None;
        }
        imported = true;
    }
    if imported || !seen {
        Some(!imported)
    } else {
        None
    }
}

impl Unbound<'_> {
    /// Whether `text` uses the name: an alias as the start of a dotted name, such as `pd.col`,
    /// and a member as a whole name that is not an attribute or keyword label, such as
    /// `assert_allclose(`.
    fn written_in(&self, text: &[u8]) -> bool {
        let part_of_name = |byte: &u8| byte.is_ascii_alphanumeric() || *byte == b'_';
        let name = self.name.as_bytes();
        text.windows(name.len()).enumerate().any(|(i, window)| {
            let before = i.checked_sub(1).map(|i| &text[i]);
            let after_idx = i + name.len();
            let after = text.get(after_idx);
            window == name
                && !before.is_some_and(|byte| part_of_name(byte) || *byte == b'.')
                && match self.binding {
                    Binding::Alias => after == Some(&b'.'),
                    Binding::Member => {
                        if after.is_some_and(part_of_name) {
                            return false;
                        }
                        let rest = &text[after_idx..];
                        let trimmed = rest
                            .iter()
                            .position(|b| !b.is_ascii_whitespace())
                            .map_or(&[][..], |p| &rest[p..]);
                        !trimmed.starts_with(b"=") || trimmed.starts_with(b"==")
                    }
                }
        })
    }
}
