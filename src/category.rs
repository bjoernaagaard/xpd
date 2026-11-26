//! Categories of public modules and the rules that guard their symbols.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::pack_names::is_module_name;

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Category {
    Data,
    Numerics,
    Statistics,
    Features,
    Models,
    Evaluation,
    Optimization,
    Visualization,
    #[default]
    Workflow,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Data => "data",
            Self::Numerics => "numerics",
            Self::Statistics => "statistics",
            Self::Features => "features",
            Self::Models => "models",
            Self::Evaluation => "evaluation",
            Self::Optimization => "optimization",
            Self::Visualization => "visualization",
            Self::Workflow => "workflow",
        })
    }
}

pub(crate) type Prefixes = BTreeMap<String, Category>;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Policy {
    pub(crate) default_category: Option<Category>,
    #[serde(default)]
    pub(crate) category_prefixes: Prefixes,
}

impl Policy {
    pub(crate) fn validate(&self, package: &str) -> Result<()> {
        for prefix in self.category_prefixes.keys() {
            ensure!(
                (prefix == package
                    || prefix
                        .strip_prefix(package)
                        .is_some_and(|s| s.starts_with('.')))
                    && prefix.split('.').all(is_module_name),
                "category prefix {prefix:?} must be a dotted module path under {package}"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "prefix")]
enum Source {
    Prefix(String),
    PackDefault,
    LegacyDefault,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prefix(prefix) => write!(f, "prefix {prefix}"),
            Self::PackDefault => f.write_str("pack default"),
            Self::LegacyDefault => f.write_str("legacy default"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModuleCategory {
    pub(crate) category: Category,
    source: Source,
}

impl Default for ModuleCategory {
    fn default() -> Self {
        Self {
            category: Category::Workflow,
            source: Source::LegacyDefault,
        }
    }
}

impl fmt::Display for ModuleCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.category, self.source)
    }
}

pub(crate) type Modules = BTreeMap<String, ModuleCategory>;

/// Optional schema-2 metadata for using a custom index outside its project.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct IndexCategories {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) default_category: Option<Category>,
    #[serde(default)]
    pub(crate) module_categories: Modules,
}

#[derive(Debug)]
pub(crate) struct Categories {
    bundled: BTreeMap<String, Policy>,
    project: BTreeMap<String, Policy>,
}

impl Categories {
    pub(crate) fn from_policies(
        bundled: BTreeMap<String, Policy>,
        project: BTreeMap<String, Policy>,
    ) -> Self {
        Self { bundled, project }
    }

    pub(crate) fn default_for(&self, package: &str) -> Option<Category> {
        self.project
            .get(package)
            .and_then(|p| p.default_category)
            .or_else(|| self.bundled.get(package).and_then(|p| p.default_category))
    }

    pub(crate) fn has_policy(&self, package: &str) -> bool {
        self.bundled.contains_key(package)
            || self.project.get(package).is_some_and(|policy| {
                policy.default_category.is_some() || !policy.category_prefixes.is_empty()
            })
    }

    pub(crate) fn module(&self, package: &str, module: &str) -> ModuleCategory {
        let local = self.project.get(package);
        let bundled = self.bundled.get(package);
        let mut candidate = module;
        loop {
            let lookup = |policy: &Policy| policy.category_prefixes.get(candidate).copied();
            let category = local.and_then(lookup).or_else(|| bundled.and_then(lookup));
            if let Some(category) = category {
                return ModuleCategory {
                    category,
                    source: Source::Prefix(candidate.to_owned()),
                };
            }
            let Some((parent, _)) = candidate.rsplit_once('.') else {
                break;
            };
            candidate = parent;
        }
        self.default_for(package)
            .map_or_else(ModuleCategory::default, |category| ModuleCategory {
                category,
                source: Source::PackDefault,
            })
    }

    /// The shipped graph stays available when an alternate index replaces symbols.
    pub(crate) fn bundled_rule(&self, package: &str, name: &str) -> BTreeSet<Category> {
        let mut categories = RULE_MODULES
            .binary_search_by_key(&name, |(id, _)| *id)
            .ok()
            .into_iter()
            .flat_map(|index| RULE_MODULES[index].1)
            .map(|(package, module)| self.module(package, module).category)
            .collect::<BTreeSet<_>>();
        if categories.is_empty() {
            categories.insert(
                self.bundled
                    .get(package)
                    .and_then(|p| p.default_category)
                    .unwrap_or_default(),
            );
        }
        categories
    }
}

type RuleModules = (&'static str, &'static [(&'static str, &'static str)]);

const RULE_MODULES: &[RuleModules] = include!(concat!(env!("OUT_DIR"), "/rule_modules.rs"));

/// Only bundled rules have a shared symbol graph. Local rules and package checks use their owner.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RuleCategory {
    Bundled,
    Default(Category),
}

impl RuleCategory {
    pub(crate) fn resolve(
        self,
        categories: &Categories,
        id: &crate::rule_id::RuleId,
    ) -> BTreeSet<Category> {
        match self {
            Self::Bundled => categories.bundled_rule(id.package(), &id.to_string()),
            Self::Default(category) => BTreeSet::from([category]),
        }
    }
}
