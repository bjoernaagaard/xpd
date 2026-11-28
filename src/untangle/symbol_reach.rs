//! Project-wide Python symbol reachability, dynamic reference checks, and parameter analysis.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::ledger::FindingPlace;

use super::python_facts::{FunctionParameter, ParsedCall, SurveyedModule, is_init_py};
use super::zones::{EntryPoints, Zone};
use super::{Finding, UntangleKind};

/// The fewest calls of a function before one caller's argument is called a special case.
const CALLS_FOR_ONE_CALLER: usize = 3;

/// Cross-module symbol index counted over all surveyed Python modules.
pub(super) struct ProjectSymbolIndex<'m> {
    modules: &'m [SurveyedModule],
    mentions: HashMap<&'m str, usize>,
    import_mentions: HashMap<&'m str, usize>,
    quoted: HashSet<&'m str>,
    /// How many `def` statements carry each name.
    defined: HashMap<&'m str, usize>,
    /// The calls of each name, with the module of each.
    calls: HashMap<&'m str, Vec<(&'m SurveyedModule, &'m ParsedCall)>>,
}

impl<'m> ProjectSymbolIndex<'m> {
    /// Builds the cross-module mention, definition, and call index for `modules`.
    pub(super) fn new(modules: &'m [SurveyedModule]) -> Self {
        let mut project = Self {
            modules,
            mentions: HashMap::new(),
            import_mentions: HashMap::new(),
            quoted: HashSet::new(),
            defined: HashMap::new(),
            calls: HashMap::new(),
        };
        for module in modules {
            let facts = &module.facts;
            for (name, count) in &facts.mentions {
                *project.mentions.entry(name).or_default() += count;
            }
            for (name, count) in &facts.import_mentions {
                *project.import_mentions.entry(name).or_default() += count;
            }
            project
                .quoted
                .extend(facts.quoted.iter().map(String::as_str));
            for name in &facts.function_names {
                *project.defined.entry(name).or_default() += 1;
            }
            for call in &facts.calls {
                (project.calls.entry(&call.name).or_default()).push((module, call));
            }
        }
        project
    }

    /// Whether another surveyed module references `module` dynamically by dotted name or via
    /// `importlib` with the module's stem.
    pub(super) fn is_dynamically_referenced(&self, idx: usize, module: &SurveyedModule) -> bool {
        let dotted = module.facts.module_name.as_deref();
        let stem = module.file.file_stem().and_then(|part| part.to_str());
        for (other_idx, other) in self.modules.iter().enumerate() {
            if other_idx == idx {
                continue;
            }
            if let Some(name) = dotted
                && other.facts.quoted_modules.contains(name)
            {
                return true;
            }
            let dynamic_loader = other.facts.mentions.contains_key("import_module")
                || other.facts.mentions.contains_key("__import__")
                || other.facts.mentions.contains_key("importlib");
            if dynamic_loader
                && let Some(stem) = stem
                && other.facts.quoted.contains(stem)
            {
                return true;
            }
        }
        false
    }

    /// Each top-level function or class unreachable from any cross-file reference, decorator,
    /// quoted string, `main`, dunder, entry-point callable, or module top-level code. Catches
    /// self-recursive and mutually recursive dead helpers as well as single-mention definitions.
    pub(super) fn unreferenced(&self, entry_points: &EntryPoints) -> Vec<Finding> {
        let mut findings = Vec::new();
        for module in self.modules {
            let skipped = module.facts.zone == Zone::Test
                || module.facts.zone.is_generated_or_vendor()
                || is_init_py(&module.file);
            if skipped {
                continue;
            }
            let defs = &module.facts.definitions;
            let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
            for (idx, definition) in defs.iter().enumerate() {
                by_name
                    .entry(definition.name.as_str())
                    .or_default()
                    .push(idx);
            }
            let mut live = vec![false; defs.len()];
            let mut queue = VecDeque::new();
            for (idx, definition) in defs.iter().enumerate() {
                let name = definition.name.as_str();
                let total_mentions = self.mentions.get(name).copied().unwrap_or(0);
                let local_mentions = module.facts.mentions.get(name).copied().unwrap_or(0);
                let mentioned_elsewhere = total_mentions > local_mentions;
                let declared_entry = module
                    .facts
                    .module_name
                    .as_deref()
                    .is_some_and(|dotted| entry_points.is_callable(dotted, name));
                let root = definition.decorated
                    || name == "main"
                    || name.starts_with("__")
                    || declared_entry
                    || self.quoted.contains(name)
                    || module.facts.top_level_mentions.contains(name)
                    || mentioned_elsewhere;
                if root {
                    live[idx] = true;
                    queue.push_back(idx);
                }
            }
            while let Some(idx) = queue.pop_front() {
                for reference in &defs[idx].references {
                    if let Some(targets) = by_name.get(reference.as_str()) {
                        for &target in targets {
                            if !live[target] {
                                live[target] = true;
                                queue.push_back(target);
                            }
                        }
                    }
                }
            }
            for (idx, definition) in defs.iter().enumerate() {
                if !live[idx] {
                    let name = definition.name.as_str();
                    findings.push(Finding {
                        kind: UntangleKind::Unreferenced,
                        key: format!("{}\n{name}", module.key()),
                        places: vec![FindingPlace::line(&module.shown, definition.line)],
                        evidence: format!(
                            "No live code path in the surveyed Python files reaches {name}."
                        ),
                        weight: 1,
                    });
                }
            }
        }
        findings
    }

    /// The parameters that every call gives the same value, and the ones that one call alone
    /// passes. Only a function whose every use is a call that can be matched is judged.
    pub(super) fn parameters(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        for module in self.modules {
            for definition in &module.facts.definitions {
                let Some(parameters) = &definition.parameters else {
                    continue;
                };
                let name = definition.name.as_str();
                let calls = self.calls.get(name).map_or(&[][..], Vec::as_slice);
                let mentions = self.mentions.get(name).copied().unwrap_or(0);
                let imported = self.import_mentions.get(name).copied().unwrap_or(0);
                let only_called = self.defined.get(name) == Some(&1)
                    && mentions == 1 + imported + calls.len()
                    && !self.quoted.contains(name);
                if !only_called || calls.len() < 2 {
                    continue;
                }
                let Some(passed) = passed_arguments(parameters, calls) else {
                    continue;
                };
                let at = FindingPlace::line(&module.shown, definition.line);
                for (parameter, values) in parameters.iter().zip(&passed) {
                    let count = calls.len();
                    let parameter_name = &parameter.name;
                    let key = format!("{}\n{name}\n{parameter_name}", module.key());
                    if values.is_empty() && parameter.default {
                        findings.push(Finding {
                            kind: UntangleKind::ConstantParameter,
                            key: key.clone(),
                            places: vec![at.clone()],
                            evidence: format!(
                                "None of the {count} calls of {name} passes {parameter_name}, so it always has its default."
                            ),
                            weight: count,
                        });
                    } else if values.len() == count
                        && let Some(value) = &values[0].1
                        && values
                            .iter()
                            .all(|(_, other)| other.as_ref() == Some(value))
                    {
                        findings.push(Finding {
                            kind: UntangleKind::ConstantParameter,
                            key: key.clone(),
                            places: vec![at.clone()],
                            evidence: format!(
                                "Each of the {count} calls of {name} passes {parameter_name}={value}."
                            ),
                            weight: count,
                        });
                    } else if values.len() == 1
                        && parameter.default
                        && count >= CALLS_FOR_ONE_CALLER
                    {
                        let (module, call) = calls[values[0].0];
                        findings.push(Finding {
                            kind: UntangleKind::OneCallerParameter,
                            key: key.clone(),
                            places: vec![at.clone(), FindingPlace::line(&module.shown, call.line)],
                            evidence: format!(
                                "One of the {count} calls of {name} passes {parameter_name}, and the others leave it at its default."
                            ),
                            weight: count,
                        });
                    }
                }
            }
        }
        findings
    }
}

/// One call that passes a parameter: the index of the call, and the literal it passes.
type PassedArg = (usize, Option<String>);

/// For each parameter, the calls that pass it. `None` when a call cannot be matched to the parameters.
fn passed_arguments(
    parameters: &[FunctionParameter],
    calls: &[(&SurveyedModule, &ParsedCall)],
) -> Option<Vec<Vec<PassedArg>>> {
    let mut passed = vec![Vec::new(); parameters.len()];
    let positional = parameters.iter().filter(|p| p.positional).count();
    let names: BTreeSet<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    for (nth, (_, call)) in calls.iter().enumerate() {
        let fits = !call.splat
            && call.positional.len() <= positional
            && (call.keywords.iter()).all(|(name, _)| names.contains(name.as_str()));
        if !fits {
            return None;
        }
        for (slot, value) in call.positional.iter().enumerate() {
            passed[slot].push((nth, value.clone()));
        }
        for (name, value) in &call.keywords {
            let slot = parameters.iter().position(|p| p.name == *name)?;
            passed[slot].push((nth, value.clone()));
        }
    }
    Some(passed)
}
