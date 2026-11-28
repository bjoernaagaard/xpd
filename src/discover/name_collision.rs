//! Public names that more than one module defines, which a search for the name cannot tell apart.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;

use crate::ledger::python_ast::top_level_definitions;
use crate::ledger::{FindingPlace, LedgerFinding};
use crate::{source, syntax};

use super::Kind;

/// Every public function and class at the top of a module, by name, with the places that
/// define it, one place per module.
#[derive(Debug, Default)]
pub(super) struct PublicDefinitions {
    places: BTreeMap<String, Vec<FindingPlace>>,
}

/// How many public names a survey read, and how many of them one module alone defines.
#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct NameCounts {
    pub(super) public_names: usize,
    pub(super) defined_once: usize,
}

impl PublicDefinitions {
    /// Record the public names of the Python file at `path`, shown as `shown`. A test file, an
    /// `__init__.py` and a module that imports hamilton, which names a column after its function,
    /// are left out, and so are `main` and names that start with an underscore.
    pub(super) fn read(&mut self, shown: &str, path: &Path, text: &str) {
        let skipped =
            source::is_test(path) || path.file_name().is_some_and(|name| name == "__init__.py");
        if skipped {
            return;
        }
        let root = syntax::parse(text);
        let module = root.root();
        let imports_hamilton = module.children().any(|item| {
            let words: Vec<_> = item
                .text()
                .split_whitespace()
                .take(2)
                .map(str::to_owned)
                .collect();
            matches!(words.first().map(String::as_str), Some("import" | "from"))
                && words
                    .get(1)
                    .is_some_and(|module| module == "hamilton" || module.starts_with("hamilton."))
        });
        if imports_hamilton {
            return;
        }
        for (definition, _) in top_level_definitions(&module) {
            let Some(name) = definition.field("name") else {
                continue;
            };
            let name = name.text();
            if name.starts_with('_') || name == "main" {
                continue;
            }
            let places = self.places.entry(name.into_owned()).or_default();
            if places.last().is_none_or(|place| place.path != shown) {
                places.push(FindingPlace::line(shown, definition.start_pos().line() + 1));
            }
        }
    }

    /// A finding for each name that two or more modules define, and the counts of all names.
    pub(super) fn collisions(self) -> (Vec<LedgerFinding<Kind>>, NameCounts) {
        let counts = NameCounts {
            public_names: self.places.len(),
            defined_once: self
                .places
                .values()
                .filter(|places| places.len() == 1)
                .count(),
        };
        let findings = self
            .places
            .into_iter()
            .filter(|(_, places)| places.len() > 1)
            .map(|(name, places)| {
                let modules = places.len();
                LedgerFinding {
                    kind: Kind::NameCollision,
                    evidence: format!(
                        "The name {name} is defined in {modules} modules, so a search for it finds {modules} definitions."
                    ),
                    key: name,
                    weight: modules,
                    places,
                }
            })
            .collect();
        (findings, counts)
    }
}
