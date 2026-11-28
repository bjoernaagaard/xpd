//! Source-only library discovery, and search by keywords and by meaning.
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::category::{Categories, Category, IndexCategories};
use crate::embed::Model;
use crate::library_cache;
use crate::outline::{self, Callable, ParameterText};
use crate::pack_names::public_module;
use crate::search::{
    DIMENSIONS, TAG_ARRAY, TAG_FALSE, TAG_NULL, TAG_OBJECT, TAG_STRING, TAG_TRUE, VECTORS_HEADER,
    tokens,
};
use crate::settings::Settings;
use crate::{pack, venv};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CallableKind {
    Class,
    Function,
    Method,
    Property,
}

impl CallableKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Function => "function",
            Self::Method => "method",
            Self::Property => "property",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Kind {
    Symbol,
    Rule,
    Recipe,
}

impl Kind {
    /// The kind's group in `Matches`, and the number of results it shows by default.
    fn group(self) -> (usize, usize) {
        match self {
            Kind::Symbol => (0, 5),
            Kind::Rule => (1, 3),
            Kind::Recipe => (2, 3),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Member {
    pub(crate) name: String,
    pub(crate) parameters: Vec<String>,
    /// The parameter list as the source writes it, as in `Entry`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) parameters_written: Vec<String>,
    pub(crate) summary: String,
    /// The member is a property, which is read and not called.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) property: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    pub(crate) kind: Kind,
    pub(crate) package: String,
    pub(crate) name: String,
    /// Defining module path when the symbol is re-exported under a shorter public alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) defined_in: Option<String>,
    /// `class` or `function` for a symbol, and `method` or `property` for a member of a class
    /// that `api` was asked for.
    pub(crate) callable: Option<CallableKind>,
    pub(crate) parameters: Vec<String>,
    /// The parameter list as the source writes it: each parameter with its type annotation and
    /// its default, and a `*` or `/` that stands alone between parameters. Empty for a function
    /// that only overloads declare.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) parameters_written: Vec<String>,
    pub(crate) summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) keywords: Vec<String>,
    pub(crate) members: Vec<Member>,
    pub(crate) rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipe: Option<crate::recipe::Recipe>,
    #[serde(
        default,
        skip_deserializing,
        skip_serializing_if = "BTreeSet::is_empty"
    )]
    pub(crate) categories: BTreeSet<Category>,
}

/// The version of an index file. Version 2 records which members of a class are properties, so
/// a member without `property` is a method. Version 1 left that out for every member.
const INDEX_SCHEMA: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Index {
    pub(crate) schema_version: u32,
    pub(crate) package: String,
    pub(crate) version: String,
    pub(crate) entries: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) recipes: Option<crate::recipe::Stats>,
    #[serde(flatten)]
    pub(crate) categories: IndexCategories,
}

/// A function or method whose parameters an entry takes. `receiver` says that Python binds its
/// first parameter to the object or the class, so a call does not pass it.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Definition {
    /// A definition in the source, by the file and the line where it starts.
    Source {
        name: String,
        file: PathBuf,
        line: usize,
        receiver: bool,
    },
    /// A function that Griffe builds, such as the `__init__` of a dataclass, which has no
    /// definition to read. Griffe writes its parameters from the fields of the class.
    Built {
        parameters: Vec<ParameterText>,
        receiver: bool,
    },
}

/// A member as py/index.py writes it, with the definitions whose parameters it takes.
#[derive(Debug, Deserialize)]
struct ResolvedMember {
    name: String,
    definitions: Vec<Definition>,
    summary: String,
    property: bool,
}

/// An entry as py/index.py writes it: Griffe's public name and documentation, and the
/// definitions whose parameters it takes.
#[derive(Debug, Deserialize)]
struct Resolved {
    kind: Kind,
    package: String,
    name: String,
    #[serde(default)]
    defined_in: Option<String>,
    callable: Option<CallableKind>,
    definitions: Vec<Definition>,
    summary: String,
    keywords: Vec<String>,
    members: Vec<ResolvedMember>,
    rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ResolvedIndex {
    package: String,
    version: String,
    entries: Vec<Resolved>,
}

/// The functions and methods of each source file that an index reads, outlined once.
#[derive(Debug, Default)]
struct Sources(HashMap<PathBuf, Vec<Callable>>);

/// The parameters of a symbol, by name and as the source writes them.
#[derive(Debug)]
struct Parameters {
    names: Vec<String>,
    written: Vec<String>,
}

impl Sources {
    /// The parameters of `name`: those of each of its definitions, each name once, without the
    /// receiver of a method. Only one definition gives a written list, because several
    /// overloads write a parameter in several ways.
    fn parameters(&mut self, name: &str, definitions: &[Definition]) -> Result<Parameters> {
        let mut parameters: Vec<ParameterText> = Vec::new();
        for definition in definitions {
            let (mut read, receiver) = match definition {
                Definition::Source {
                    name: function,
                    file,
                    line,
                    receiver,
                } => {
                    if !self.0.contains_key(file) {
                        let src = std::fs::read_to_string(file)
                            .with_context(|| format!("cannot read {}", file.display()))?;
                        self.0.insert(file.clone(), outline::python_callables(&src));
                    }
                    let callable = (self.0[file].iter())
                        .find(|c| c.line == *line && c.name == *function)
                        .with_context(|| {
                            format!("{name}: no {function} at {}:{line}", file.display())
                        })?;
                    (callable.parameters.as_slice(), *receiver)
                }
                Definition::Built {
                    parameters,
                    receiver,
                } => (parameters.as_slice(), *receiver),
            };
            // The receiver is the first parameter whatever its name is, and `*args` takes it
            // together with the arguments of the call.
            if receiver
                && let [first, rest @ ..] = read
                && first.name.is_some()
                && !first.written.starts_with('*')
            {
                read = rest;
                // `def method(self, /, value)` leaves a `/` with no parameter before it.
                if let [slash, rest @ ..] = read
                    && slash.written == "/"
                {
                    read = rest;
                }
            }
            for parameter in read {
                // A `*` or `/` that stands alone has no name, and a definition writes it once.
                let known = (parameter.name.as_ref())
                    .is_some_and(|name| parameters.iter().any(|p| p.name.as_ref() == Some(name)));
                if !known {
                    parameters.push(parameter.clone());
                }
            }
        }
        let names = parameters.iter().filter_map(|p| p.name.clone()).collect();
        let written = match definitions {
            [_] => parameters.into_iter().map(|p| p.written).collect(),
            _ => Vec::new(),
        };
        Ok(Parameters { names, written })
    }
}

/// Whether [`generate`] may invoke `uv run --with griffe==2.4.1` when the project Python lacks Griffe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GriffeFallback {
    /// Fail with the exact install command if the project Python lacks `griffe==2.4.1`.
    None,
    /// Retry transparently with `uv run --no-project --with griffe==2.4.1 python`.
    UvRun,
}

/// Gets the public API from Griffe without importing the library, and the parameters of each
/// symbol from its source.
pub(crate) fn generate(
    source: &Path,
    package: &str,
    version: &str,
    fallback: GriffeFallback,
) -> Result<Index> {
    let mut output = std::process::Command::new(venv::project_python())
        .arg("-I")
        .arg("-c")
        .arg(include_str!("../py/index.py"))
        .arg(source)
        .arg(package)
        .arg(version)
        .output()
        .context("could not start Python for Griffe")?;
    if fallback == GriffeFallback::UvRun
        && !output.status.success()
        && String::from_utf8_lossy(&output.stderr).contains("Index building needs griffe==")
        && let Ok(uv_output) = std::process::Command::new("uv")
            .args([
                "run",
                "--no-project",
                "--with",
                "griffe==2.4.1",
                "python",
                "-I",
                "-c",
                include_str!("../py/index.py"),
            ])
            .arg(source)
            .arg(package)
            .arg(version)
            .output()
    {
        output = uv_output;
    }
    ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let resolved: ResolvedIndex =
        serde_json::from_slice(&output.stdout).context("invalid Griffe index")?;
    let mut sources = Sources::default();
    let mut entries = Vec::with_capacity(resolved.entries.len());
    for entry in resolved.entries {
        let mut members = Vec::with_capacity(entry.members.len());
        for member in entry.members {
            let name = format!("{}.{}", entry.name, member.name);
            let parameters = sources.parameters(&name, &member.definitions)?;
            members.push(Member {
                parameters: parameters.names,
                parameters_written: parameters.written,
                name: member.name,
                summary: member.summary,
                property: member.property,
            });
        }
        let parameters = sources.parameters(&entry.name, &entry.definitions)?;
        entries.push(Entry {
            parameters: parameters.names,
            parameters_written: parameters.written,
            kind: entry.kind,
            package: entry.package,
            name: entry.name,
            defined_in: entry.defined_in,
            callable: entry.callable,
            summary: entry.summary,
            keywords: entry.keywords,
            members,
            rules: entry.rules,
            recipe: None,
            categories: BTreeSet::new(),
        });
    }
    let mut index = Index {
        schema_version: INDEX_SCHEMA,
        package: resolved.package,
        version: resolved.version,
        entries,
        recipes: None,
        categories: IndexCategories::default(),
    };
    ensure!(!index.entries.is_empty(), "Griffe found no public symbols");
    attach_rules(&mut index.entries, &[package])?;
    Ok(index)
}

impl Index {
    pub(crate) fn classify(&mut self, categories: &Categories) {
        self.categories.default_category = categories.default_for(&self.package);
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.kind == Kind::Symbol)
        {
            let module = entry.module();
            self.categories
                .module_categories
                .entry(module.to_owned())
                .or_insert_with(|| categories.module(&self.package, module));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmbedMode {
    Auto,
    CachedOnly,
    KeywordsOnly,
}

#[derive(Debug)]
struct LibraryLayer {
    package: String,
    entries: Vec<Entry>,
    categories: IndexCategories,
    cache_key: Option<String>,
    vectors: RefCell<Option<Vec<[u8; DIMENSIONS]>>>,
}

impl LibraryLayer {
    fn from_index(index: Index, cache_key: Option<String>) -> Self {
        Self {
            package: index.package,
            entries: index.entries,
            categories: index.categories,
            cache_key,
            vectors: RefCell::new(None),
        }
    }

    fn vectors(&self, model: Option<&Model>, embed_mode: EmbedMode) -> Vec<[u8; DIMENSIONS]> {
        let mut guard = self.vectors.borrow_mut();
        let vectors = guard.get_or_insert_with(|| match self.cache_key.as_deref() {
            Some(key) => library_cache::load_cached_vectors(key, &self.entries),
            None => vec![[0; DIMENSIONS]; self.entries.len()],
        });
        if embed_mode == EmbedMode::Auto
            && let (Some(key), Some(model)) = (self.cache_key.as_deref(), model)
            && let Err(error) =
                library_cache::ensure_cached_vectors(key, &self.entries, vectors, model)
        {
            eprintln!(
                "xpd: cannot cache vectors for {}: {error:#}. Search uses keywords only.",
                self.package
            );
        }
        vectors.clone()
    }
}

#[derive(Debug)]
pub(crate) struct Library {
    include_bundled: bool,
    layers: Vec<LibraryLayer>,
    embed_mode: EmbedMode,
    /// Loaded on the first search, so `api` never loads it. `None` inside when it is absent.
    model: OnceCell<Option<Model>>,
}

/// Stable cache key for a local index file path.
pub(crate) fn file_cache_key(path: &Path) -> String {
    let resolved = std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_path_buf());
    format!("file:{}", resolved.display())
}

/// Bundled package names and their symbol counts.
pub(crate) fn bundled_package_counts() -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for record in RECORDS {
        if record.kind == Kind::Symbol {
            *counts.entry(record.package).or_default() += 1;
        }
    }
    counts
}

impl Library {
    /// The same library, which searches by meaning only when the embedding model is already
    /// on this machine. The hook searches on every prompt, and it must not wait for a download.
    pub(crate) fn without_download(self, project: &Path) -> Self {
        Self {
            embed_mode: EmbedMode::CachedOnly,
            model: OnceCell::from(Model::cached(project)),
            ..self
        }
    }

    /// The same library, searched by keywords only.
    pub(crate) fn keywords_only(self) -> Self {
        Self {
            embed_mode: EmbedMode::KeywordsOnly,
            model: OnceCell::from(None),
            ..self
        }
    }

    /// The embedding model, respecting the configured embedding mode.
    fn model(&self) -> Option<&Model> {
        match self.embed_mode {
            EmbedMode::KeywordsOnly => None,
            EmbedMode::CachedOnly => self.model.get().and_then(Option::as_ref),
            EmbedMode::Auto => self.model.get_or_init(Model::open).as_ref(),
        }
    }

    /// Opens or downloads the embedding model and reports whether semantic search is available.
    pub(crate) fn ensure_model(&self) -> bool {
        self.model().is_some()
    }

    fn effective_layers(&self) -> Vec<usize> {
        let mut latest_by_package = BTreeMap::new();
        for (index, layer) in self.layers.iter().enumerate() {
            latest_by_package.insert(layer.package.as_str(), index);
        }
        let mut active: Vec<usize> = latest_by_package.into_values().collect();
        active.sort_unstable();
        active
    }

    fn shadowed_packages(&self) -> BTreeSet<&str> {
        self.layers
            .iter()
            .map(|layer| layer.package.as_str())
            .collect()
    }

    fn layer_for_package(&self, package: &str) -> Option<&LibraryLayer> {
        self.layers
            .iter()
            .rev()
            .find(|layer| layer.package == package)
    }
}

impl Library {
    fn portable_policy(&self, categories: &Categories, package: &str) -> Option<&IndexCategories> {
        if categories.has_policy(package) {
            return None;
        }
        self.layer_for_package(package)
            .map(|layer| &layer.categories)
    }

    fn module_category(&self, categories: &Categories, package: &str, module: &str) -> Category {
        if let Some(policy) = self.portable_policy(categories, package) {
            return policy.module_categories.get(module).map_or_else(
                || policy.default_category.unwrap_or_default(),
                |row| row.category,
            );
        }
        categories.module(package, module).category
    }

    fn package_default(&self, categories: &Categories, package: &str) -> Category {
        self.portable_policy(categories, package)
            .and_then(|policy| policy.default_category)
            .or_else(|| categories.default_for(package))
            .unwrap_or_default()
    }

    pub(crate) fn entry_categories(
        &self,
        categories: &Categories,
        entry: &Entry,
    ) -> BTreeSet<Category> {
        match entry.kind {
            Kind::Symbol => {
                BTreeSet::from([self.module_category(categories, &entry.package, entry.module())])
            }
            Kind::Rule => {
                if let Some(layer) = self.layer_for_package(&entry.package)
                    && !RECORDS
                        .iter()
                        .any(|record| record.kind == Kind::Rule && record.name == entry.name)
                {
                    let mut linked = layer
                        .entries
                        .iter()
                        .filter(|symbol| {
                            symbol.kind == Kind::Symbol && symbol.rules.contains(&entry.name)
                        })
                        .map(|symbol| {
                            self.module_category(categories, &symbol.package, symbol.module())
                        })
                        .collect::<BTreeSet<_>>();
                    if linked.is_empty() {
                        linked.insert(self.package_default(categories, &entry.package));
                    }
                    linked
                } else {
                    categories.bundled_rule(&entry.package, &entry.name)
                }
            }
            Kind::Recipe => {
                let mut found = BTreeSet::new();
                if let Some(recipe) = &entry.recipe {
                    for name in &recipe.symbols {
                        if let Ok(Some(symbol)) = api(self, name)
                            && symbol.kind == Kind::Symbol
                        {
                            found.insert(self.module_category(
                                categories,
                                &symbol.package,
                                symbol.module(),
                            ));
                        }
                    }
                    if recipe.is_playbook() {
                        for id in &entry.rules {
                            if let Some((package, _)) = id.split_once('/') {
                                found.extend(categories.bundled_rule(package, id));
                            }
                        }
                    }
                }
                if found.is_empty() {
                    found.insert(self.package_default(categories, &entry.package));
                }
                found
            }
        }
    }
}

fn read_index_file(path: &Path) -> Result<Index> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read index {}", path.display()))?;
    let index: Index = serde_json::from_str(&raw)
        .with_context(|| format!("cannot parse index {}", path.display()))?;
    validate_index(&index, &path.display().to_string())?;
    Ok(index)
}

fn validate_index(index: &Index, label: &str) -> Result<()> {
    ensure!(
        index.schema_version == INDEX_SCHEMA,
        "{label} has index schema {}, and this xpd reads schema {INDEX_SCHEMA}. Build the index again with xpd index.",
        index.schema_version
    );
    for entry in &index.entries {
        let valid_callable = match entry.kind {
            Kind::Symbol => entry.callable.is_some(),
            Kind::Rule | Kind::Recipe => entry.callable.is_none(),
        };
        ensure!(
            valid_callable,
            "{label}: entry {} has invalid callable {:?} for kind {:?}",
            entry.name,
            entry.callable,
            entry.kind
        );
    }
    Ok(())
}

fn load_specialized_manim() -> Result<LibraryLayer> {
    let index: Index = serde_json::from_str(include_str!("../indexes/specialized/manim.json"))?;
    validate_index(&index, "bundled Manim index")?;
    Ok(LibraryLayer::from_index(
        index,
        Some("specialized:manim".to_owned()),
    ))
}

pub(crate) fn load(extra: Option<&Path>) -> Result<Library> {
    let mut layers = Vec::new();
    if let Some(path) = extra {
        let index = read_index_file(path)?;
        layers.push(LibraryLayer::from_index(index, Some(file_cache_key(path))));
    }
    Ok(Library {
        include_bundled: true,
        layers,
        embed_mode: EmbedMode::Auto,
        model: OnceCell::new(),
    })
}

/// Loads bundled libraries layered with any `[tool.xpd.libraries]` configured for `project`,
/// an optional `--index` file, and the specialized Manim index when `package == Some("manim")`.
pub(crate) fn load_for_project(
    project: &Path,
    extra: Option<&Path>,
    package: Option<&str>,
) -> Result<Library> {
    let mut layers = Vec::new();
    if let Ok(settings) = Settings::find(project) {
        for path in settings.libraries.values() {
            let index = read_index_file(path)?;
            layers.push(LibraryLayer::from_index(index, Some(file_cache_key(path))));
        }
    }
    if let Some(path) = extra {
        let index = read_index_file(path)?;
        layers.push(LibraryLayer::from_index(index, Some(file_cache_key(path))));
    }
    if package == Some("manim") && !layers.iter().any(|layer| layer.package == "manim") {
        layers.push(load_specialized_manim()?);
    }
    Ok(Library {
        include_bundled: true,
        layers,
        embed_mode: EmbedMode::Auto,
        model: OnceCell::new(),
    })
}

pub(crate) fn load_for(extra: Option<&Path>, package: Option<&str>) -> Result<Library> {
    if let Ok(cwd) = std::env::current_dir() {
        load_for_project(&cwd, extra, package)
    } else {
        let mut library = load(extra)?;
        if package == Some("manim") && !library.layers.iter().any(|layer| layer.package == "manim")
        {
            library.layers.push(load_specialized_manim()?);
        }
        Ok(library)
    }
}

impl Entry {
    fn module(&self) -> &str {
        let owner = if matches!(
            self.callable,
            Some(CallableKind::Method | CallableKind::Property)
        ) {
            self.name
                .rsplit_once('.')
                .map_or(self.name.as_str(), |(class, _)| class)
        } else {
            &self.name
        };
        public_module(&self.package, owner)
    }

    /// Whether this is one of xpd's own recipes, which state the house style for a kind of
    /// task.
    pub(crate) fn is_playbook(&self) -> bool {
        (self.recipe.as_ref()).is_some_and(crate::recipe::Recipe::is_playbook)
    }

    /// The method or property `name` of this class, as an entry of its own.
    fn member(self, name: &str) -> Option<Self> {
        let member = self.members.into_iter().find(|m| m.name == name)?;
        let callable = if member.property {
            CallableKind::Property
        } else {
            CallableKind::Method
        };
        Some(Self {
            kind: Kind::Symbol,
            package: self.package,
            name: format!("{}.{}", self.name, member.name),
            defined_in: self.defined_in,
            callable: Some(callable),
            parameters: member.parameters,
            parameters_written: member.parameters_written,
            summary: member.summary,
            keywords: Vec::new(),
            members: Vec::new(),
            rules: Vec::new(),
            recipe: None,
            categories: BTreeSet::new(),
        })
    }
}

/// The entry named `name`, or else the member that `name` names as `Class.member`.
pub(crate) fn api(library: &Library, name: &str) -> Result<Option<Entry>> {
    let shadowed = library.shadowed_packages();
    let entry = |name: &str| -> Result<Option<Entry>> {
        for index in library.effective_layers().into_iter().rev() {
            let layer = &library.layers[index];
            if let Some(found) = layer.entries.iter().find(|entry| entry.name == name) {
                return Ok(Some(found.clone()));
            }
        }
        if !library.include_bundled {
            return Ok(None);
        }
        RECORDS
            .iter()
            .filter(|record| record.kind == Kind::Rule || !shadowed.contains(record.package))
            .find(|record| record.name == name)
            .map(|record| record.entry().context("embedded entry"))
            .transpose()
    };
    if let Some(found) = entry(name)? {
        return Ok(Some(found));
    }
    let Some((class, member)) = name.rsplit_once('.') else {
        return Ok(None);
    };
    Ok(entry(class)?.and_then(|class| class.member(member)))
}

/// A method of a class of the index: the name of the class with its package, and the
/// parameters of the method as the source writes them.
pub(crate) type Method = (String, Vec<String>);

/// The methods of the classes of `packages`, by the name of the method.
pub(crate) fn methods(
    library: &Library,
    packages: &BTreeSet<String>,
) -> Result<HashMap<String, Vec<Method>>> {
    let mut methods: HashMap<String, Vec<Method>> = HashMap::new();
    let mut add = |class: Entry| {
        for member in class.members.into_iter().filter(|member| !member.property) {
            let method = (class.name.clone(), member.parameters_written);
            methods.entry(member.name).or_default().push(method);
        }
    };
    let shadowed = library.shadowed_packages();
    for index in library.effective_layers() {
        let layer = &library.layers[index];
        if packages.contains(&layer.package) {
            for class in layer
                .entries
                .iter()
                .filter(|entry| entry.kind == Kind::Symbol)
            {
                add(class.clone());
            }
        }
    }
    if library.include_bundled {
        for record in RECORDS {
            if record.kind == Kind::Symbol
                && !shadowed.contains(record.package)
                && packages.contains(record.package)
            {
                add(record.entry().context("embedded entry")?);
            }
        }
    }
    Ok(methods)
}

#[derive(Debug)]
struct Stored {
    name: &'static str,
    package: &'static str,
    kind: Kind,
    summary: &'static str,
    /// The hash of the name and the summary, which build.rs computes.
    text: u64,
    start: u32,
    len: u16,
}
const DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/entries.bin"));
const STRINGS: &[&str] = include!(concat!(env!("OUT_DIR"), "/strings.rs"));

impl Stored {
    /// Whether the record is one of xpd's own recipes. build.rs gives no other record the
    /// kind and the package of a playbook.
    fn is_playbook(&self) -> bool {
        self.kind == Kind::Recipe && self.package == "xpd"
    }

    fn entry(&self) -> Result<Entry, serde_json::Error> {
        let start = usize::try_from(self.start).expect("entry offset fits usize");
        let end = start + usize::from(self.len);
        serde_json::from_value(decode(&mut &DATA[start..end]))
    }
}

fn integer(data: &mut &[u8]) -> usize {
    let (bytes, rest) = data.split_at(2);
    *data = rest;
    usize::from(u16::from_le_bytes(
        bytes.try_into().expect("build writes u16 IDs"),
    ))
}

fn decode(data: &mut &[u8]) -> serde_json::Value {
    use serde_json::Value;
    let tag = data[0];
    *data = &data[1..];
    match tag {
        TAG_NULL => Value::Null,
        TAG_FALSE => Value::Bool(false),
        TAG_TRUE => Value::Bool(true),
        TAG_STRING => Value::String(STRINGS[integer(data)].to_owned()),
        TAG_ARRAY => Value::Array((0..integer(data)).map(|_| decode(data)).collect()),
        TAG_OBJECT => Value::Object(
            (0..integer(data))
                .map(|_| {
                    let key = STRINGS[integer(data)].to_owned();
                    (key, decode(data))
                })
                .collect(),
        ),
        _ => unreachable!("build writes validated entry tags"),
    }
}

const POSTINGS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/postings.bin"));
const TABLE: &[(&str, u32, u32)] = include!(concat!(env!("OUT_DIR"), "/search.rs"));
const RECORDS: &[Stored] = include!(concat!(env!("OUT_DIR"), "/records.rs"));
/// `DIMENSIONS` signed bytes for each of `RECORDS`, all zero for a record that has no vector
/// of its current text.
const VECTORS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/vectors.bin"));
/// Reciprocal rank fusion adds `weight / (FUSION_K + rank)` from each ranking.
const FUSION_K: f64 = 60.0;

#[derive(Debug, Default, Serialize)]
pub(crate) struct Matches {
    pub(crate) symbols: Vec<Entry>,
    pub(crate) rules: Vec<Entry>,
    pub(crate) recipes: Vec<Entry>,
}

impl Matches {
    fn collect(entries: impl IntoIterator<Item = Entry>, limit: Option<usize>) -> Self {
        let mut found = Self::default();
        for entry in entries {
            let (_, default) = entry.kind.group();
            let group = match entry.kind {
                Kind::Symbol => &mut found.symbols,
                Kind::Rule => &mut found.rules,
                Kind::Recipe => &mut found.recipes,
            };
            if group.len() < limit.unwrap_or(default) {
                group.push(entry);
            }
        }
        found
    }
}

pub(crate) fn find(
    library: &Library,
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
    limit: Option<usize>,
) -> Matches {
    find_in_category(library, query, package, kind, limit, None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Candidate {
    Bundled(usize),
    Layer(usize, usize),
}

impl Candidate {
    fn kind(self, library: &Library) -> Kind {
        match self {
            Self::Bundled(index) => RECORDS[index].kind,
            Self::Layer(layer, index) => library.layers[layer].entries[index].kind,
        }
    }

    fn name(self, library: &Library) -> &str {
        match self {
            Self::Bundled(index) => RECORDS[index].name,
            Self::Layer(layer, index) => &library.layers[layer].entries[index].name,
        }
    }

    fn is_playbook(self, library: &Library) -> bool {
        match self {
            Self::Bundled(index) => RECORDS[index].is_playbook(),
            Self::Layer(layer, index) => library.layers[layer].entries[index].is_playbook(),
        }
    }

    fn to_entry(self, library: &Library) -> Entry {
        match self {
            Self::Bundled(index) => RECORDS[index].entry().expect("build validated index entry"),
            Self::Layer(layer, index) => library.layers[layer].entries[index].clone(),
        }
    }
}

pub(crate) fn find_in_category(
    library: &Library,
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
    limit: Option<usize>,
    category: Option<(&Categories, Category)>,
) -> Matches {
    let shadowed = library.shadowed_packages();
    let active_layers: Vec<usize> = library
        .effective_layers()
        .into_iter()
        .filter(|&layer_idx| {
            let layer = &library.layers[layer_idx];
            package.is_none_or(|pkg| layer.package == pkg)
                && layer
                    .entries
                    .iter()
                    .any(|entry| kind.is_none_or(|wanted| entry.kind == wanted))
        })
        .collect();
    let passes = |record: &Stored| {
        (record.kind == Kind::Rule || !shadowed.contains(record.package))
            && package.is_none_or(|pkg| record.package == pkg)
            && kind.is_none_or(|wanted| record.kind == wanted)
    };
    if active_layers.is_empty() && library.include_bundled {
        return find_bundled_only(library, query, package, kind, limit, category, &passes);
    }

    let mut ranked = Vec::new();
    if library.include_bundled {
        ranked.extend(
            keyword_ranking(query, passes)
                .into_iter()
                .map(|(index, score)| (Candidate::Bundled(index), score)),
        );
    }
    for &layer_idx in &active_layers {
        ranked.extend(layer_keyword_ranking(
            &library.layers[layer_idx],
            layer_idx,
            query,
            package,
            kind,
        ));
    }
    sort_candidates(library, &mut ranked);

    if let Some(model) = library.model() {
        let mut queries = vec![query];
        if kind.is_none_or(|wanted| wanted == Kind::Rule) && limit != Some(0) {
            queries.extend(rule_query_parts(query));
        }
        match model.queries(&queries) {
            Ok(query_vectors) => {
                let mut meaning = Vec::new();
                if library.include_bundled {
                    meaning.extend(
                        meaning_ranking(&query_vectors[0], passes)
                            .into_iter()
                            .map(|(index, score)| (Candidate::Bundled(index), score)),
                    );
                }
                for &layer_idx in &active_layers {
                    let layer = &library.layers[layer_idx];
                    let vectors = layer.vectors(Some(model), library.embed_mode);
                    meaning.extend(layer_meaning_ranking(
                        layer,
                        layer_idx,
                        &vectors,
                        &query_vectors[0],
                        package,
                        kind,
                    ));
                }
                sort_candidates(library, &mut meaning);
                ranked = fuse_candidates(library, &ranked, &meaning);
                prioritize_rule_candidates(
                    &mut ranked,
                    &queries[1..],
                    &query_vectors[1..],
                    package,
                );
            }
            Err(error) => {
                eprintln!("xpd: cannot embed the query: {error:#}. Search uses keywords only.");
            }
        }
    }

    collect_candidates(library, ranked, limit, category)
}

fn find_bundled_only(
    library: &Library,
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
    limit: Option<usize>,
    category: Option<(&Categories, Category)>,
    passes: &impl Fn(&Stored) -> bool,
) -> Matches {
    let mut ranked = keyword_ranking(query, passes);
    if let Some(model) = library.model() {
        let mut queries = vec![query];
        if kind.is_none_or(|wanted| wanted == Kind::Rule) && limit != Some(0) {
            queries.extend(rule_query_parts(query));
        }
        match model.queries(&queries) {
            Ok(vectors) => {
                ranked = fuse(&ranked, &meaning_ranking(&vectors[0], passes));
                prioritize_rules(&mut ranked, &queries[1..], &vectors[1..], package);
            }
            Err(error) => {
                eprintln!("xpd: cannot embed the query: {error:#}. Search uses keywords only.");
            }
        }
    }
    let mut counts = [0; 3];
    Matches::collect(
        ranked
            .into_iter()
            .filter(|(i, _)| {
                let record = &RECORDS[*i];
                if let Some((policy, wanted)) = category
                    && !bundled_matches_category(library, record, policy, wanted)
                {
                    return false;
                }
                let (slot, default) = record.kind.group();
                counts[slot] += 1;
                counts[slot] <= limit.unwrap_or(default)
            })
            .map(|(i, _)| RECORDS[i].entry().expect("build validated index entry")),
        limit,
    )
}

fn collect_candidates(
    library: &Library,
    ranked: Vec<(Candidate, f64)>,
    limit: Option<usize>,
    category: Option<(&Categories, Category)>,
) -> Matches {
    let mut counts = [0; 3];
    Matches::collect(
        ranked
            .into_iter()
            .filter(|&(candidate, _)| {
                if let Some((policy, wanted)) = category {
                    let matches = match candidate {
                        Candidate::Bundled(index) => {
                            bundled_matches_category(library, &RECORDS[index], policy, wanted)
                        }
                        Candidate::Layer(layer_idx, entry_idx) => library
                            .entry_categories(policy, &library.layers[layer_idx].entries[entry_idx])
                            .contains(&wanted),
                    };
                    if !matches {
                        return false;
                    }
                }
                let (slot, default) = candidate.kind(library).group();
                counts[slot] += 1;
                counts[slot] <= limit.unwrap_or(default)
            })
            .map(|(candidate, _)| candidate.to_entry(library)),
        limit,
    )
}

fn bundled_matches_category(
    library: &Library,
    record: &Stored,
    policy: &Categories,
    wanted: Category,
) -> bool {
    match record.kind {
        Kind::Symbol => {
            let module = public_module(record.package, record.name);
            library.module_category(policy, record.package, module) == wanted
        }
        Kind::Rule => policy
            .bundled_rule(record.package, record.name)
            .contains(&wanted),
        Kind::Recipe => {
            let recipe = record.entry().expect("build validated recipe");
            library.entry_categories(policy, &recipe).contains(&wanted)
        }
    }
}

fn sort_candidates(library: &Library, ranked: &mut [(Candidate, f64)]) {
    ranked.sort_by(|(a, sa), (b, sb)| {
        sb.total_cmp(sa)
            .then_with(|| b.is_playbook(library).cmp(&a.is_playbook(library)))
            .then_with(|| a.name(library).cmp(b.name(library)))
    });
}

fn fuse_candidates(
    library: &Library,
    keywords: &[(Candidate, f64)],
    meaning: &[(Candidate, f64)],
) -> Vec<(Candidate, f64)> {
    let mut fused = BTreeMap::<Candidate, f64>::new();
    for (ranking, weight, bonus) in [
        (keywords, 1.15, [0.05, 0.02, 0.02]),
        (meaning, 1.0, [0.0; 3]),
    ] {
        let mut counts = [0_u32; 3];
        for &(candidate, _) in ranking {
            let (slot, _) = candidate.kind(library).group();
            counts[slot] += 1;
            let rank = counts[slot];
            let top = usize::try_from(rank - 1)
                .ok()
                .and_then(|place| bonus.get(place))
                .copied()
                .unwrap_or_default();
            *fused.entry(candidate).or_default() += weight / (FUSION_K + f64::from(rank)) + top;
        }
    }
    let mut ranked: Vec<_> = fused.into_iter().collect();
    sort_candidates(library, &mut ranked);
    ranked
}

fn prioritize_rule_candidates(
    ranked: &mut [(Candidate, f64)],
    queries: &[&str],
    vectors: &[Vec<f32>],
    package: Option<&str>,
) {
    if queries.is_empty() {
        return;
    }
    let rankings: Vec<_> = vectors
        .iter()
        .map(|vector| {
            meaning_ranking(vector, |entry| {
                entry.kind == Kind::Rule && package.is_none_or(|pkg| entry.package == pkg)
            })
        })
        .collect();
    let leads: Vec<_> = rankings
        .iter()
        .filter_map(|ranking| ranking.first().map(|&(index, _)| Candidate::Bundled(index)))
        .collect();
    ranked.sort_by_key(|&(candidate, _)| !leads.contains(&candidate));
}

fn layer_keyword_ranking(
    layer: &LibraryLayer,
    layer_idx: usize,
    query: &str,
    package: Option<&str>,
    kind: Option<Kind>,
) -> Vec<(Candidate, f64)> {
    let documents = layer
        .entries
        .iter()
        .map(|entry| {
            let qualified = match entry.defined_in.as_deref() {
                Some(module) if module != entry.package && !entry.name.starts_with(module) => {
                    format!("{} {module}", entry.name)
                }
                _ => entry.name.clone(),
            };
            let enriched_summary = match entry.defined_in.as_deref() {
                Some(module) if module != entry.package && !entry.name.starts_with(module) => {
                    let module_words = module.replace('.', " ");
                    if entry.summary.is_empty() {
                        module_words
                    } else {
                        format!("{} {module_words}", entry.summary)
                    }
                }
                _ => entry.summary.clone(),
            };
            [
                entry
                    .name
                    .rsplit('.')
                    .next()
                    .unwrap_or(&entry.name)
                    .to_owned(),
                qualified,
                enriched_summary,
                serde_json::to_string(&entry.parameters).unwrap_or_default(),
                serde_json::to_string(&entry.keywords).unwrap_or_default(),
                entry
                    .members
                    .iter()
                    .map(|member| member.summary.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ]
        })
        .collect::<Vec<_>>();
    let table = crate::search::weights(&documents);
    let words: BTreeSet<_> = tokens(query).into_iter().collect();
    let mut scores = vec![0.0; layer.entries.len()];
    for word in &words {
        if let Some(postings) = table.get(word) {
            for &(index, score) in postings {
                scores[index] += score;
            }
        }
    }
    layer
        .entries
        .iter()
        .enumerate()
        .zip(scores)
        .filter(|((_, entry), score)| {
            *score > 0.0
                && package.is_none_or(|pkg| entry.package == pkg)
                && kind.is_none_or(|wanted| entry.kind == wanted)
        })
        .map(|((entry_idx, entry), score)| {
            (
                Candidate::Layer(layer_idx, entry_idx),
                rank_score(
                    &entry.name,
                    &entry.summary,
                    entry.kind,
                    query,
                    words.len(),
                    score,
                ),
            )
        })
        .collect()
}

fn layer_meaning_ranking(
    layer: &LibraryLayer,
    layer_idx: usize,
    vectors: &[[u8; DIMENSIONS]],
    meaning: &[f32],
    package: Option<&str>,
    kind: Option<Kind>,
) -> Vec<(Candidate, f64)> {
    layer
        .entries
        .iter()
        .enumerate()
        .zip(vectors)
        .filter(|((_, entry), _)| {
            package.is_none_or(|pkg| entry.package == pkg)
                && kind.is_none_or(|wanted| entry.kind == wanted)
        })
        .filter_map(|((entry_idx, _), vector)| {
            let (dot, norm) =
                vector
                    .iter()
                    .zip(meaning)
                    .fold((0.0, 0.0), |(dot, norm), (&v, &m)| {
                        let v = f32::from(v.cast_signed());
                        (dot + v * m, norm + v * v)
                    });
            (norm > 0.0).then(|| {
                (
                    Candidate::Layer(layer_idx, entry_idx),
                    f64::from(dot / norm.sqrt()),
                )
            })
        })
        .collect()
}

fn rule_query_parts(query: &str) -> Vec<&str> {
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, mark) in query.match_indices(['.', '?', '!', ';', '\n']) {
        let end = index + mark.len();
        if mark == "\n" || query[end..].starts_with(char::is_whitespace) {
            parts.push(query[start..end].trim());
            start = end;
        }
    }
    parts.push(query[start..].trim());
    parts.retain(|part| tokens(part).len() >= 3);
    if !(2..=5).contains(&parts.len()) {
        parts.clear();
    }
    parts
}

fn prioritize_rules(
    ranked: &mut [(usize, f64)],
    queries: &[&str],
    vectors: &[Vec<f32>],
    package: Option<&str>,
) {
    if queries.is_empty() {
        return;
    }
    let rankings: Vec<_> = vectors
        .iter()
        .map(|vector| {
            meaning_ranking(vector, |entry| {
                entry.kind == Kind::Rule && package.is_none_or(|package| entry.package == package)
            })
        })
        .collect();
    let mut leads: Vec<_> = rankings
        .iter()
        .filter_map(|ranking| ranking.first().map(|&(index, _)| index))
        .collect();
    if let ([first, second], [one, two]) = (&leads[..], queries)
        && one.ends_with('?') != two.ends_with('?')
        && RECORDS[*first].package == RECORDS[*second].package
    {
        let package = RECORDS[*first].package;
        let focus = usize::from(two.ends_with('?'));
        leads.swap(0, 1 - focus);
        let keywords = keyword_ranking(queries[focus], |entry| {
            entry.kind == Kind::Rule && entry.package == package
        });
        let meaning: Vec<_> = rankings[focus]
            .iter()
            .copied()
            .filter(|&(index, _)| RECORDS[index].package == package)
            .collect();
        leads.extend(
            fuse(&keywords, &meaning)
                .into_iter()
                .take(5)
                .map(|(index, _)| index),
        );
        ranked.sort_by_key(|&(index, _)| {
            leads
                .iter()
                .position(|&lead| lead == index)
                .unwrap_or(usize::MAX)
        });
    } else {
        ranked.sort_by_key(|&(index, _)| !leads.contains(&index));
    }
}

/// Sorts `(record, score)` pairs by score, best first. Between equal scores a playbook comes
/// before a recipe taken from a library, and then the name decides.
fn sort_ranking(ranked: &mut [(usize, f64)]) {
    ranked.sort_by(|(a, sa), (b, sb)| {
        let (a, b) = (&RECORDS[*a], &RECORDS[*b]);
        sb.total_cmp(sa)
            .then_with(|| b.is_playbook().cmp(&a.is_playbook()))
            .then_with(|| a.name.cmp(b.name))
    });
}

/// The playbooks, which are the recipes of xpd's own package.
pub(crate) fn playbooks() -> Result<Vec<Entry>> {
    (RECORDS.iter())
        .filter(|record| record.is_playbook())
        .map(|record| record.entry().context("embedded playbook"))
        .collect()
}

/// The lowest keyword score at which a playbook is taken as the answer to a prompt. The score
/// is the one that the keyword search ranks by, so the bar is the same with and without the
/// embedding model. Of the prompts tried on 2026-11-19, the ten loose asks scored 28.2 and
/// above for their playbook. Of thirty prompts about other work, none had a playbook as its
/// first recipe, and the best playbook of each scored 18.0 and below.
const PLAYBOOK_BAR: f64 = 24.0;

/// The playbook that answers `prompt`: the first recipe that `find` gives for it, when that is
/// a playbook whose keyword score clears [`PLAYBOOK_BAR`]. A prompt about other work shares
/// few words with the asks of a playbook, so it gets `None`.
pub(crate) fn playbook_for(library: &Library, prompt: &str) -> Option<Entry> {
    let first = (find(library, prompt, None, Some(Kind::Recipe), Some(1)).recipes)
        .pop()
        .filter(Entry::is_playbook)?;
    let (_, score) = *keyword_ranking(prompt, |record| record.name == first.name).first()?;
    (score >= PLAYBOOK_BAR).then_some(first)
}

/// The records that share a keyword with the query, best first.
fn keyword_ranking(query: &str, passes: impl Fn(&Stored) -> bool) -> Vec<(usize, f64)> {
    let words: BTreeSet<_> = tokens(query).into_iter().collect();
    let mut scores = vec![0.0; RECORDS.len()];
    for word in &words {
        if let Ok(i) = TABLE.binary_search_by_key(&word.as_str(), |(w, _, _)| *w) {
            let start = usize::try_from(TABLE[i].1).expect("posting offset fits usize");
            let end = usize::try_from(TABLE[i].2).expect("posting offset fits usize");
            for posting in POSTINGS[start..end].as_chunks::<4>().0 {
                let entry = usize::from(u16::from_le_bytes(
                    posting[..2]
                        .try_into()
                        .expect("build writes two byte entry IDs"),
                ));
                let score = u16::from_le_bytes(
                    posting[2..]
                        .try_into()
                        .expect("build writes two byte scores"),
                );
                scores[entry] += f64::from(score) / 256.0;
            }
        }
    }
    let mut ranked: Vec<_> = scores
        .into_iter()
        .enumerate()
        .filter(|(i, score)| *score > 0.0 && passes(&RECORDS[*i]))
        .map(|(i, score)| {
            let e = &RECORDS[i];
            (
                i,
                rank_score(e.name, e.summary, e.kind, query, words.len(), score),
            )
        })
        .collect();
    sort_ranking(&mut ranked);
    ranked
}

/// Every record with a vector, by its cosine similarity to the query's vector, best first.
fn meaning_ranking(meaning: &[f32], passes: impl Fn(&Stored) -> bool) -> Vec<(usize, f64)> {
    let mut ranked: Vec<_> = VECTORS
        .as_chunks::<DIMENSIONS>()
        .0
        .iter()
        .enumerate()
        .filter(|(i, _)| passes(&RECORDS[*i]))
        .filter_map(|(i, vector)| {
            let (dot, norm) =
                vector
                    .iter()
                    .zip(meaning)
                    .fold((0.0, 0.0), |(dot, norm), (&v, &m)| {
                        let v = f32::from(v.cast_signed());
                        (dot + v * m, norm + v * v)
                    });
            (norm > 0.0).then(|| (i, f64::from(dot / norm.sqrt())))
        })
        .collect();
    sort_ranking(&mut ranked);
    ranked
}

/// Reciprocal rank fusion of the two rankings, within each kind. The keyword ranking weighs a
/// little more, and its first three get a bonus, so an exact match keeps its place. The values
/// pass every checked question in indexes/questions.json, and 1.0 or 1.3 for the weight do not.
fn fuse(keywords: &[(usize, f64)], meaning: &[(usize, f64)]) -> Vec<(usize, f64)> {
    let mut fused = BTreeMap::<usize, f64>::new();
    for (ranking, weight, bonus) in [
        (keywords, 1.15, [0.05, 0.02, 0.02]),
        (meaning, 1.0, [0.0; 3]),
    ] {
        let mut counts = [0_u32; 3];
        for &(i, _) in ranking {
            let (slot, _) = RECORDS[i].kind.group();
            counts[slot] += 1;
            let rank = counts[slot];
            let top = usize::try_from(rank - 1)
                .ok()
                .and_then(|place| bonus.get(place))
                .copied()
                .unwrap_or_default();
            *fused.entry(i).or_default() += weight / (FUSION_K + f64::from(rank)) + top;
        }
    }
    let mut ranked: Vec<_> = fused.into_iter().collect();
    sort_ranking(&mut ranked);
    ranked
}

/// The names of the embedded records that have no vector.
pub(crate) fn records_without_vector() -> Vec<&'static str> {
    RECORDS
        .iter()
        .zip(VECTORS.as_chunks::<DIMENSIONS>().0)
        .filter(|(_, vector)| vector.iter().all(|&v| v == 0))
        .map(|(e, _)| e.name)
        .collect()
}

/// Writes the vector of every embedded record to `path`, as build.rs reads it. A record keeps
/// the vector that this build holds for it. build.rs drops the vector of a record whose text
/// changed, so only new and changed records are embedded, and a new rule costs one text. Each vector is scaled so its largest value is 127. Search
/// compares directions, so the scale drops out.
pub(crate) fn write_vectors(path: &Path) -> Result<()> {
    let stored = VECTORS.as_chunks::<DIMENSIONS>().0;
    let missing: Vec<_> = (RECORDS.iter().zip(stored))
        .filter(|(_, vector)| vector.iter().all(|&v| v == 0))
        .map(|(e, _)| (e.name, e.summary))
        .collect();
    let mut embedded = if missing.is_empty() {
        Vec::new()
    } else {
        let model = Model::open().context("the embedding model is needed to write vectors")?;
        model.documents(&missing)?
    }
    .into_iter();
    let mut data = VECTORS_HEADER.to_vec();
    for (record, vector) in RECORDS.iter().zip(stored) {
        data.extend(u16::try_from(record.name.len())?.to_le_bytes());
        data.extend(record.name.as_bytes());
        data.extend(record.text.to_le_bytes());
        if vector.iter().any(|&v| v != 0) {
            data.extend(vector);
            continue;
        }
        let vector = embedded
            .next()
            .context("every missing record was embedded")?;
        let largest = vector.iter().fold(f32::EPSILON, |m, v| m.max(v.abs()));
        data.extend(vector.iter().map(|v| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::as_conversions,
                reason = "rounded value within -127..=127"
            )]
            let byte = (v / largest * 127.0).round() as i8;
            byte.cast_unsigned()
        }));
    }
    eprintln!(
        "xpd: embedded {} records and kept {}",
        missing.len(),
        RECORDS.len() - missing.len()
    );
    std::fs::write(path, data).with_context(|| format!("cannot write {}", path.display()))
}

fn rank_score(
    name: &str,
    summary: &str,
    kind: Kind,
    query: &str,
    words: usize,
    mut score: f64,
) -> f64 {
    let phrase = query.to_lowercase();
    if words > 1 && summary.to_lowercase().contains(&phrase) {
        score += 12.0;
    }
    let leaf = name.rsplit('.').next().unwrap_or(name);
    if leaf.len() > 2
        && leaf.chars().all(|c| c.is_uppercase() || c.is_ascii_digit())
        && phrase
            .split(|c: char| !c.is_alphanumeric())
            .any(|s| s == leaf.to_lowercase())
    {
        score += 30.0;
    }
    let depth = u32::try_from(name.split('.').count().saturating_sub(2)).unwrap_or(u32::MAX);
    if kind == Kind::Symbol {
        score /= 1.0 + 0.25 * f64::from(depth);
    }
    score
}

/// Reads wheel metadata without executing the distribution.
pub(crate) fn source_version(source: &Path, package: &str) -> Result<String> {
    for item in std::fs::read_dir(source)? {
        let path = item?.path();
        if path.extension().is_none_or(|e| e != "dist-info") {
            continue;
        }
        let top = std::fs::read_to_string(path.join("top_level.txt")).unwrap_or_default();
        let record = std::fs::read_to_string(path.join("RECORD")).unwrap_or_default();
        if top.lines().any(|line| line == package)
            || record
                .lines()
                .any(|line| line.starts_with(&format!("{package}/")))
        {
            let metadata = std::fs::read_to_string(path.join("METADATA"))?;
            return metadata
                .lines()
                .find_map(|line| line.strip_prefix("Version: "))
                .map(str::to_owned)
                .context("METADATA has no Version");
        }
    }
    anyhow::bail!("no distribution metadata for {package}; pass --version for a source checkout")
}

fn attach_rules(entries: &mut [Entry], packs: &[&str]) -> Result<()> {
    for pack in packs {
        let links = pack::symbol_rules(pack)?;
        for entry in entries.iter_mut().filter(|e| e.package == *pack) {
            entry.rules = links
                .get(entry.name.rsplit('.').next().unwrap_or(&entry.name))
                .cloned()
                .unwrap_or_default();
        }
    }
    Ok(())
}
