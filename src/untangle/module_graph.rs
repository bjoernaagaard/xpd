//! Directed Python module import graph, Tarjan cycle detection, and orphan/facade/single-use checks.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use crate::imports::ImportScope;
use crate::ledger::FindingPlace;

use super::python_facts::{SurveyedModule, is_init_py};
use super::symbol_reach::ProjectSymbolIndex;
use super::zones::{EntryPoints, Zone};
use super::{Finding, UntangleKind};

/// The fewest commits of an imported module before its importer is compared with it. Below
/// this, the history is too short to call the module volatile.
const VOLATILE_COMMITS: usize = 8;
/// Minimum lines before a module with zero importers is flagged as orphaned.
const ORPHANED_MIN_LOC: usize = 10;
/// Maximum importers before a re-export facade is treated as an established public surface.
const FACADE_MAX_IMPORTERS: usize = 20;
/// Minimum lines for a single-use helper module to be flagged as an inlining candidate.
const SINGLE_USE_MIN_LOC: usize = 20;
/// Maximum lines for a single-use helper module to be flagged as an inlining candidate.
const SINGLE_USE_MAX_LOC: usize = 300;

/// One resolved import edge between two surveyed Python modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModuleImportEdge {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) eager: bool,
}

/// Every resolved import edge between surveyed Python modules, noting whether an eager
/// top-level import connects the pair.
pub(super) fn resolve_module_edges(modules: &[SurveyedModule]) -> Vec<ModuleImportEdge> {
    let graph = ModuleImportGraph::new(modules);
    let mut found = Vec::new();
    for (from, targets) in graph.imports.iter().enumerate() {
        for &to in targets.keys() {
            let eager = graph.eager_imports[from].contains_key(&to);
            found.push(ModuleImportEdge { from, to, eager });
        }
    }
    found
}

/// Each import between two tracked modules, as the importer and the imported module.
pub(super) fn tracked_import_pairs(modules: &[SurveyedModule]) -> HashSet<(&str, &str)> {
    let graph = ModuleImportGraph::new(modules);
    let tracked = |module: usize| modules[module].tracked.as_deref();
    (graph.imports.iter().enumerate())
        .flat_map(|(importer, imports)| {
            imports
                .keys()
                .filter_map(move |&imported| Some((tracked(importer)?, tracked(imported)?)))
        })
        .collect()
}

/// Which module imports which, by the index of each in the list of surveyed modules.
pub(super) struct ModuleImportGraph<'m> {
    modules: &'m [SurveyedModule],
    /// For each module, all modules it imports (any scope) with the line of the first import.
    imports: Vec<BTreeMap<usize, usize>>,
    /// For each module, top-level eager imports only, used for runtime import cycle detection.
    eager_imports: Vec<BTreeMap<usize, usize>>,
    /// For each module, the modules that import it across any scope.
    importers: Vec<Vec<usize>>,
}

impl<'m> ModuleImportGraph<'m> {
    /// Builds the directed import adjacency graph for `modules`.
    pub(super) fn new(modules: &'m [SurveyedModule]) -> Self {
        let index: HashMap<&Path, usize> = (modules.iter().enumerate())
            .map(|(nth, module)| (module.file.as_path(), nth))
            .collect();
        let mut imports = Vec::with_capacity(modules.len());
        let mut eager_imports = Vec::with_capacity(modules.len());
        let mut importers = vec![Vec::new(); modules.len()];

        for (nth, module) in modules.iter().enumerate() {
            let mut all_edges = BTreeMap::new();
            let mut eager_edges = BTreeMap::new();
            for import in &module.facts.imports {
                if let Some(&imported) = index.get(import.module.as_path())
                    && imported != nth
                {
                    all_edges.entry(imported).or_insert(import.line);
                    if import.scope == ImportScope::TopLevel {
                        eager_edges.entry(imported).or_insert(import.line);
                    }
                }
            }
            for &imported in all_edges.keys() {
                importers[imported].push(nth);
            }
            imports.push(all_edges);
            eager_imports.push(eager_edges);
        }
        Self {
            modules,
            imports,
            eager_imports,
            importers,
        }
    }

    fn place(&self, module: usize, line: usize) -> FindingPlace {
        FindingPlace::line(&self.modules[module].shown, line)
    }

    fn production_count(&self) -> usize {
        self.modules
            .iter()
            .filter(|module| module.facts.zone == Zone::Production)
            .count()
    }

    /// Each import of a module that changed at least twice as often as its importer, counted
    /// from the first commit of the importer. A test is left out, because it follows the code it
    /// tests, and so is an `__init__.py`, which gives its own modules their public names.
    pub(super) fn volatile(&self, changes: &HashMap<&str, Vec<usize>>) -> Vec<Finding> {
        let commits_of = |module: usize| {
            let tracked = self.modules[module].tracked.as_deref()?;
            changes.get(tracked)
        };
        let mut findings = Vec::new();
        for (importer, imports) in self.imports.iter().enumerate() {
            let module = &self.modules[importer];
            if module.facts.zone == Zone::Test
                || module.facts.zone.is_generated_or_vendor()
                || is_init_py(&module.file)
            {
                continue;
            }
            for (&imported, &line) in imports {
                let (Some(stable), Some(volatile)) = (commits_of(importer), commits_of(imported))
                else {
                    continue;
                };
                let Some(&first) = stable.last() else {
                    continue;
                };
                let (stable, volatile) = (
                    stable.len(),
                    volatile.iter().filter(|&&commit| commit <= first).count(),
                );
                if volatile >= VOLATILE_COMMITS && volatile >= stable * 2 {
                    findings.push(Finding {
                        kind: UntangleKind::VolatileImport,
                        key: format!(
                            "{}\n{}",
                            self.modules[importer].key(),
                            self.modules[imported].key()
                        ),
                        places: vec![self.place(importer, line)],
                        evidence: format!(
                            "{} changed in {stable} {} and imports {}, which changed in {volatile} commits in the same time.",
                            self.modules[importer].shown,
                            if stable == 1 { "commit" } else { "commits" },
                            self.modules[imported].shown
                        ),
                        weight: volatile / stable,
                    });
                }
            }
        }
        findings
    }

    /// Each group of modules that import one another eagerly at module level, found as the
    /// strongly connected parts of the eager import graph.
    pub(super) fn cycles(&self) -> Vec<Finding> {
        let mut state = Tarjan {
            imports: &self.eager_imports,
            order: vec![None; self.modules.len()],
            low: vec![0; self.modules.len()],
            stack: Vec::new(),
            on_stack: vec![false; self.modules.len()],
            next: 0,
            parts: Vec::new(),
        };
        for module in 0..self.modules.len() {
            if state.order[module].is_none() {
                state.visit(module);
            }
        }
        (state.parts.into_iter())
            .filter(|part| {
                part.len() > 1
                    && !part.iter().all(|&module| {
                        let zone = self.modules[module].facts.zone;
                        zone == Zone::Test || zone.is_generated_or_vendor()
                    })
            })
            .map(|mut part| {
                part.sort_unstable();
                let places = (part.iter())
                    .map(|&module| {
                        let line = (self.eager_imports[module].iter())
                            .find(|(imported, _)| part.contains(imported))
                            .map_or(1, |(_, line)| *line);
                        self.place(module, line)
                    })
                    .collect();
                Finding {
                    kind: UntangleKind::ImportCycle,
                    key: (part.iter().map(|&module| self.modules[module].key()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    places,
                    evidence: format!(
                        "These {} modules import one another in a circle at load time, so none of them can change alone.",
                        part.len()
                    ),
                    weight: part.len(),
                }
            })
            .collect()
    }

    /// Production modules with zero surveyed importers that are not entry points or public surfaces.
    pub(super) fn orphaned(
        &self,
        entry_points: &EntryPoints,
        project: &ProjectSymbolIndex<'_>,
    ) -> Vec<Finding> {
        if self.production_count() < 2 {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            if facts.zone != Zone::Production
                || !self.importers[nth].is_empty()
                || is_init_py(&module.file)
                || facts.loc < ORPHANED_MIN_LOC
                || facts.has_dunder_all
                || facts.has_main_guard
                || facts
                    .module_name
                    .as_deref()
                    .is_some_and(|name| entry_points.contains(name))
                || project.is_dynamically_referenced(nth, module)
            {
                continue;
            }
            findings.push(Finding {
                kind: UntangleKind::OrphanedModule,
                key: module.key().to_owned(),
                places: vec![FindingPlace::file(&module.shown)],
                evidence: format!(
                    "No surveyed Python file imports {} ({} lines).",
                    module.shown, facts.loc
                ),
                weight: facts.loc,
            });
        }
        findings
    }

    /// Pure re-export forwarding modules that add indirection without owning logic.
    pub(super) fn facades(&self) -> Vec<Finding> {
        let mut dir_counts: HashMap<&Path, usize> = HashMap::new();
        for module in self.modules {
            if let Some(dir) = module.file.parent() {
                *dir_counts.entry(dir).or_default() += 1;
            }
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            let importers = self.importers[nth].len();
            if facts.zone != Zone::Production
                || !facts.is_facade
                || importers > FACADE_MAX_IMPORTERS
            {
                continue;
            }
            if is_init_py(&module.file)
                && module
                    .file
                    .parent()
                    .and_then(|dir| dir_counts.get(dir))
                    .is_some_and(|&count| count > 1)
            {
                continue;
            }
            findings.push(Finding {
                kind: UntangleKind::FacadeModule,
                key: module.key().to_owned(),
                places: vec![FindingPlace::file(&module.shown)],
                evidence: format!(
                    "{} only re-exports names from other modules and has {} {}.",
                    module.shown,
                    importers,
                    if importers == 1 {
                        "importer"
                    } else {
                        "importers"
                    }
                ),
                weight: facts.loc + (FACADE_MAX_IMPORTERS + 1).saturating_sub(importers),
            });
        }
        findings
    }

    /// Production helper modules imported by only one non-test file.
    pub(super) fn single_use(&self, entry_points: &EntryPoints) -> Vec<Finding> {
        if self.production_count() < 2 {
            return Vec::new();
        }
        let mut findings = Vec::new();
        for (nth, module) in self.modules.iter().enumerate() {
            let facts = &module.facts;
            if facts.zone != Zone::Production
                || is_init_py(&module.file)
                || facts.is_facade
                || facts.has_dunder_all
                || facts.has_main_guard
                || facts.loc < SINGLE_USE_MIN_LOC
                || facts.loc > SINGLE_USE_MAX_LOC
                || facts
                    .module_name
                    .as_deref()
                    .is_some_and(|name| entry_points.contains(name))
            {
                continue;
            }
            let [sole_idx] = self.importers[nth].as_slice() else {
                continue;
            };
            let sole = &self.modules[*sole_idx];
            if matches!(sole.facts.zone, Zone::Test | Zone::Script)
                || is_init_py(&sole.file)
                || sole.facts.loc + facts.loc > SINGLE_USE_MAX_LOC
                || self.imports[*sole_idx].len() >= 5
            {
                continue;
            }
            findings.push(Finding {
                kind: UntangleKind::SingleUseModule,
                key: module.key().to_owned(),
                places: vec![
                    FindingPlace::file(&module.shown),
                    FindingPlace::file(&sole.shown),
                ],
                evidence: format!(
                    "Only {} imports {} ({} lines), so it may belong in its sole caller.",
                    sole.shown, module.shown, facts.loc
                ),
                weight: facts.loc,
            });
        }
        findings
    }
}

/// Tarjan's search for the strongly connected parts of the import graph.
struct Tarjan<'g> {
    imports: &'g [BTreeMap<usize, usize>],
    order: Vec<Option<usize>>,
    low: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    next: usize,
    parts: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, module: usize) {
        let order = self.next;
        self.next += 1;
        self.order[module] = Some(order);
        self.low[module] = order;
        self.stack.push(module);
        self.on_stack[module] = true;
        let imports = self.imports;
        for &imported in imports[module].keys() {
            match self.order[imported] {
                None => {
                    self.visit(imported);
                    self.low[module] = self.low[module].min(self.low[imported]);
                }
                Some(seen) if self.on_stack[imported] => {
                    self.low[module] = self.low[module].min(seen);
                }
                Some(_) => {}
            }
        }
        if self.low[module] == order {
            let mut part = Vec::new();
            while let Some(member) = self.stack.pop() {
                self.on_stack[member] = false;
                part.push(member);
                if member == module {
                    break;
                }
            }
            self.parts.push(part);
        }
    }
}
