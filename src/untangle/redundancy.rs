use std::collections::{BTreeSet, HashMap, HashSet};

use anyhow::Result;

use crate::index::{self, Library, Method};
use crate::ledger::{
    FindingPlace,
    python_ast::{called_method_name, imports_of, root_name},
    twins,
};
use crate::syntax::{Node, Root};

use super::{Finding, Surveyed, UntangleKind};

const CLASSES_SHOWN: usize = 3;

/// How many texts that near copies differ in the evidence names.
const SWAPS_SHOWN: usize = 3;
/// How many characters of one such text the evidence quotes.
const SWAP_CHARS: usize = 30;
/// `text` on one line, cut to the length the evidence quotes for a text that differs.
fn swapped(text: &str) -> String {
    crate::output::quote_one_line(text, SWAP_CHARS)
}

/// The near copies among the files.
pub(super) fn near_copies(files: &[(&str, twins::Shapes)]) -> Vec<Finding> {
    (twins::near_copy_groups(files).into_iter())
        .map(|twins| {
            let (_, block) = twins.members[0];
            let lines = block.lines.1 - block.lines.0 + 1;
            let mut differ: Vec<String> = (twins.swaps.iter().take(SWAPS_SHOWN))
                .map(|(here, there)| format!("`{}` and `{}`", swapped(here), swapped(there)))
                .collect();
            if twins.swaps.len() > SWAPS_SHOWN {
                differ.push(format!("{} more", twins.swaps.len() - SWAPS_SHOWN));
            }
            let swaps: Vec<String> = (twins.swaps.iter())
                .map(|(here, there)| format!("{here}\t{there}"))
                .collect();
            Finding {
                kind: UntangleKind::NearCopy,
                key: format!("{:016x}\n{}", block.shape, swaps.join("\n")),
                places: (twins.members.iter())
                    .map(|(file, block)| FindingPlace::lines(file, block.lines))
                    .collect(),
                evidence: format!(
                    "These {} stretches of {lines} lines are the same code but for {}.",
                    twins.members.len(),
                    differ.join(", ")
                ),
                weight: block.tokens * twins.members.len(),
            }
        })
        .collect()
}

/// What a call names as the thing it calls.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Callee {
    /// A name of a library with its module, as `numpy.linspace`.
    Symbol(String),
    /// A method of a value whose class the file does not say, as `groupby`.
    Method(String),
}

/// A keyword argument with a literal value, which may repeat the default of the callee.
#[derive(Debug)]
pub(super) struct Keyword {
    pub(super) line: usize,
    pub(super) callee: Callee,
    pub(super) name: String,
    pub(super) value: String,
}

/// What one Python file gives the ledger.
#[derive(Debug, Default)]
pub(super) struct DefaultArgFacts {
    pub(super) keywords: Vec<Keyword>,
    /// The packages the file imports, by the first part of the module name.
    pub(super) packages: BTreeSet<String>,
    /// The names of the functions the file defines.
    pub(super) functions: Vec<String>,
}

/// The text of `node` when it is a literal that a default could equal.
fn literal(node: &Node<'_>) -> Option<String> {
    let number = |node: &Node<'_>| matches!(node.kind().as_ref(), "integer" | "float");
    let plain = match node.kind().as_ref() {
        "integer" | "float" | "true" | "false" | "none" => true,
        "string" => crate::syntax::string_literal(node).is_some(),
        "unary_operator" => node.field("argument").is_some_and(|inner| number(&inner)),
        _ => false,
    };
    plain.then(|| node.text().into_owned())
}

/// What `call` calls, by the imports of the file. `None` for a call of the object itself.
fn callee(call: &Node<'_>, imports: &HashMap<String, String>) -> Option<Callee> {
    let function = call.field("function")?;
    let dotted = function.text();
    let plain = (dotted.split('.'))
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '_'));
    let root = root_name(&function)?;
    if let (true, Some(module)) = (plain, imports.get(&root)) {
        let rest = dotted.strip_prefix(root.as_str()).unwrap_or("");
        return Some(Callee::Symbol(format!("{module}{rest}")));
    }
    let method = function.kind() == "attribute" && !matches!(root.as_str(), "self" | "cls");
    method
        .then(|| called_method_name(call).map(Callee::Method))
        .flatten()
}

/// The name and the default of a parameter as the source of a library writes it, as
/// `observed: bool = True`. `None` for a parameter with no default, or with one that a stub
/// file leaves out.
fn default_of(written: &str) -> Option<(&str, &str)> {
    let (name, default) = match written.split_once(':') {
        Some((name, annotated)) => (name, annotated.rsplit_once(" = ")?.1),
        None => written.split_once('=')?,
    };
    let (name, default) = (name.trim(), default.trim());
    (!name.starts_with('*') && default != "...").then_some((name, default))
}

/// Whether the literal `passed` is the value `default`, whichever quotes a string has.
fn same_value(passed: &str, default: &str) -> bool {
    let unquoted = |text: &str| {
        let quote = text.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        text.strip_prefix(quote)?
            .strip_suffix(quote)
            .map(str::to_owned)
    };
    match (unquoted(passed), unquoted(default)) {
        (Some(passed), Some(default)) => passed == default,
        (None, None) => passed == default,
        _ => false,
    }
}

/// The defaults of the library functions that the files call.
struct Defaults {
    library: Library,
    /// The parameters of each name that was looked up. `None` for a name the index lacks.
    symbols: HashMap<String, Option<Vec<String>>>,
    methods: HashMap<String, Vec<Method>>,
    /// The functions that the project defines, whose calls are not calls of a library.
    own: HashSet<String>,
}

impl Defaults {
    fn new(files: &[Surveyed]) -> Result<Self> {
        let facts = || files.iter().filter_map(|file| file.defaults.as_ref());
        let packages: BTreeSet<String> = facts()
            .flat_map(|facts| facts.packages.iter().cloned())
            .collect();
        let library = index::load(None)?;
        Ok(Self {
            methods: index::methods(&library, &packages)?,
            library,
            symbols: HashMap::new(),
            own: (facts().flat_map(|facts| facts.functions.iter().cloned())).collect(),
        })
    }

    fn symbol(&mut self, name: &str) -> Result<Option<&[String]>> {
        if !self.symbols.contains_key(name) {
            let parameters = index::api(&self.library, name)?.map(|entry| entry.parameters_written);
            self.symbols.insert(name.to_owned(), parameters);
        }
        Ok(self.symbols[name].as_deref())
    }

    /// The evidence that `keyword` passes a default. `packages` holds the packages that its
    /// file imports. A method counts when every class of those packages that has a method of
    /// the name, with the parameter, gives it this default.
    fn passed(&mut self, keyword: &Keyword, packages: &BTreeSet<String>) -> Result<Option<String>> {
        let is_default = |written: &String| {
            default_of(written).is_some_and(|(name, default)| {
                name == keyword.name && same_value(&keyword.value, default)
            })
        };
        let written = format!("`{}={}`", keyword.name, keyword.value);
        let method = match &keyword.callee {
            Callee::Symbol(symbol) => match self.symbol(symbol)? {
                Some(parameters) => {
                    return Ok((parameters.iter().any(is_default))
                        .then(|| format!("{written} passes the default of `{symbol}`.")));
                }
                None => symbol.rsplit('.').next().unwrap_or(symbol),
            },
            Callee::Method(method) => method,
        };
        if self.own.contains(method) {
            return Ok(None);
        }
        let has = |written: &String| {
            default_of(written).is_some_and(|(name, _)| name == keyword.name)
                || written.split([':', '=']).next().map(str::trim) == Some(keyword.name.as_str())
        };
        let classes: Vec<&Method> = (self.methods.get(method).into_iter().flatten())
            .filter(|(class, parameters)| {
                packages.contains(class.split('.').next().unwrap_or(class))
                    && parameters.iter().any(has)
            })
            .collect();
        let agree = !classes.is_empty()
            && (classes.iter()).all(|(_, parameters)| parameters.iter().any(is_default));
        Ok(agree.then(|| {
            let mut names: Vec<String> = (classes.iter().take(CLASSES_SHOWN))
                .map(|(class, _)| class.clone())
                .collect();
            if classes.len() > CLASSES_SHOWN {
                names.push(format!("{} more", classes.len() - CLASSES_SHOWN));
            }
            format!(
                "{written} passes the default that `{method}` has in {}.",
                names.join(", ")
            )
        }))
    }
}

fn line(node: &Node<'_>) -> usize {
    node.start_pos().line() + 1
}

pub(super) fn extract_default_arg_facts(root: &Root) -> DefaultArgFacts {
    let mut facts = DefaultArgFacts::default();
    let mut imports = HashMap::new();
    for node in root.root().dfs() {
        match node.kind().as_ref() {
            "import_statement" | "import_from_statement" => {
                for (bound, module) in imports_of(&node) {
                    facts
                        .packages
                        .extend(module.split('.').next().map(str::to_owned));
                    imports.insert(bound, module);
                }
            }
            "function_definition" => facts
                .functions
                .extend(node.field("name").map(|name| name.text().into_owned())),
            _ => {}
        }
    }
    for call in root.root().dfs().filter(|node| node.kind() == "call") {
        let Some(arguments) = call.field("arguments") else {
            continue;
        };
        let keywords: Vec<(String, String)> = (arguments.children())
            .filter(|argument| argument.kind() == "keyword_argument")
            .filter_map(|argument| {
                let value = literal(&argument.field("value")?)?;
                Some((argument.field("name")?.text().into_owned(), value))
            })
            .collect();
        if keywords.is_empty() {
            continue;
        }
        let Some(callee) = callee(&call, &imports) else {
            continue;
        };
        facts
            .keywords
            .extend(keywords.into_iter().map(|(name, value)| Keyword {
                line: line(&call),
                callee: callee.clone(),
                name,
                value,
            }));
    }
    facts
}

pub(super) fn default_arguments(surveyed: &[Surveyed]) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut defaults = None;
    for file in surveyed {
        let Some(facts) = &file.defaults else {
            continue;
        };
        for keyword in &facts.keywords {
            let defaults = match &mut defaults {
                Some(defaults) => defaults,
                None => defaults.insert(Defaults::new(surveyed)?),
            };
            let Some(evidence) = defaults.passed(keyword, &facts.packages)? else {
                continue;
            };
            let callee = match &keyword.callee {
                Callee::Symbol(name) | Callee::Method(name) => name,
            };
            findings.push(Finding {
                kind: UntangleKind::DefaultArgument,
                key: format!(
                    "{}\n{callee}\n{}",
                    file.tracked.as_deref().unwrap_or(&file.shown),
                    keyword.name
                ),
                places: vec![FindingPlace::line(&file.shown, keyword.line)],
                evidence,
                weight: 1,
            });
        }
    }
    Ok(findings)
}
