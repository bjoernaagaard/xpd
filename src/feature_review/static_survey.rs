use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, ensure};
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use serde::Serialize;
use serde_json::Value;

use crate::catalog::Filters;
use crate::graph::Graph;
use crate::ledger::gix_source::Listing;
use crate::ledger::python::{called, imports_of};
use crate::ledger::{self, AdapterPolicy, Catalog, Finding, Place, SurveyOutput, Tree};
use crate::source;
use crate::syntax::{self, Node};
use crate::workspace::Workspace;

use super::{CATALOG, Feature, FeatureKind, Kind, Manifest, Rolling, fold_facts, repo_relative};

#[derive(Debug, Serialize)]
pub(super) struct Read {
    manifest: Option<Manifest>,
    graph: Value,
    pub(super) facts: Value,
}

impl Read {
    pub(super) fn has_manifest(&self) -> bool {
        self.manifest.is_some()
    }
}

pub(super) fn survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    top: Option<&Path>,
    commit: &str,
    adapter: Option<&str>,
    policy: AdapterPolicy,
) -> Result<SurveyOutput<Kind, Read>> {
    let root = tree.top.context("feature-review needs a Git repository")?;
    let manifest = read_manifest(root)?;
    if let Some(manifest) = &manifest {
        validate_manifest(manifest, root, paths)?;
    }
    let features = manifest
        .as_ref()
        .map_or(&[][..], |manifest| &manifest.features);
    let listing = Listing::of(paths, tree, top, commit)?;
    let python: BTreeMap<String, _> = listing
        .files
        .iter()
        .filter(|(_, language)| *language == SupportLang::Python)
        .filter_map(|(path, _)| {
            let tracked = tree.tracked(&listing.absolute(path)?)?;
            Some((tracked, listing.text(path)?))
        })
        .collect();
    let graph_paths: Vec<_> = python.keys().map(PathBuf::from).collect();
    let graph = Graph::read(root, &graph_paths).to_json();
    let nodes = feature_nodes(features, &graph)?;
    let known_rules: BTreeMap<_, _> = Kind::value_variants()
        .iter()
        .copied()
        .map(|kind| (kind.source_rule(), kind))
        .filter(|(rule, _)| !rule.starts_with("feature-review/"))
        .collect();
    let workspace = Workspace::open(
        root.to_path_buf(),
        None,
        &Filters {
            rules: known_rules.keys().map(|rule| (*rule).to_owned()).collect(),
            severity: None,
        },
    )?;
    let mut findings = Vec::new();
    let mut computations = Vec::new();
    for (owner, text) in &python {
        append_pack_findings(&workspace, &known_rules, root, owner, text, &mut findings);
        let parsed = syntax::parse(text);
        encoder_findings(&parsed.root(), owner, &mut findings);
        for (feature, node) in nodes
            .iter()
            .copied()
            .filter(|(feature, _)| &feature.owner == owner)
        {
            let Some(function) = feature_function(&parsed.root(), &node["function"]) else {
                continue;
            };
            if let Some(rolling) = &feature.rolling {
                rolling_findings(&function, feature, rolling, &mut findings);
            }
            if function
                .parent()
                .is_some_and(|parent| parent.kind() == "module")
                && let Some(expression) = node_arithmetic(&function, node)
            {
                computations.push((feature, position(owner, &function), expression));
            }
        }
    }
    computations.sort_by(|left, right| left.1.cmp(&right.1));
    computations.dedup_by(|left, right| left.1 == right.1);
    duplicate_findings(computations, &mut findings);

    let implemented = features
        .iter()
        .any(|feature| feature.kind == FeatureKind::Feature);
    ensure!(
        !implemented || adapter.is_some() || policy == AdapterPolicy::BestEffort,
        "recorded feature-review with implemented candidates requires an adapter"
    );
    let facts = match (adapter, &manifest) {
        (Some(spec), Some(manifest)) => fold_facts::measure(spec, root, manifest)?,
        (Some(_), None) => anyhow::bail!(
            "the feature-review adapter needs feature-review.json at the Git root, with the target, prediction cutoff, baseline and candidates"
        ),
        (None, _) => Value::Null,
    };
    let catalog = Catalog::load(CATALOG)?;
    let checklist = features
        .iter()
        .flat_map(|feature| {
            catalog.checklist(
                BTreeSet::from([(feature.id.clone(), feature.owner.clone())]),
                feature.kind.name(),
            )
        })
        .collect();
    Ok(SurveyOutput {
        read: Read {
            manifest,
            graph,
            facts,
        },
        findings: ledger::named(findings),
        checklist,
    })
}

fn feature_function<'t>(root: &Node<'t>, name: &Value) -> Option<Node<'t>> {
    root.named_children()
        .filter_map(|entry| match entry.kind().as_ref() {
            "function_definition" => Some(entry),
            "decorated_definition" => entry
                .field("definition")
                .filter(|definition| definition.kind() == "function_definition"),
            _ => None,
        })
        .find(|function| {
            function
                .field("name")
                .is_some_and(|symbol| *name == symbol.text().as_ref())
        })
}

fn append_pack_findings(
    workspace: &Workspace,
    rules: &BTreeMap<&str, Kind>,
    root: &Path,
    owner: &str,
    text: &str,
    findings: &mut Vec<Finding<Kind>>,
) {
    let linted = workspace.analyze(source::File::on_disk(&root.join(owner)), text);
    for found in &linted.found {
        let issue = found.issue();
        let rule = issue.id.to_string();
        if let Some(kind) = rules.get(rule.as_str()) {
            let mut place = Place::file(owner);
            if let Some(at) = issue.position {
                place.line = Some(at.line);
                place.column = Some(at.column);
            }
            findings.push(Finding {
                kind: *kind,
                key: format!("{owner}\n{rule}\n{}", issue.message),
                places: vec![place],
                evidence: issue.message.clone(),
                weight: 1,
            });
        }
    }
}

/// Reads `feature-review.json` at the Git root. A project without one gets the pack and encoder
/// findings only, because the rolling, duplicate and checklist parts need its declared features.
fn read_manifest(root: &Path) -> Result<Option<Manifest>> {
    let bytes = match std::fs::read(root.join("feature-review.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("cannot read feature-review.json"),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .context("invalid feature-review.json")
}

fn validate_manifest(manifest: &Manifest, root: &Path, paths: &[PathBuf]) -> Result<()> {
    ensure!(
        !manifest.target.trim().is_empty()
            && !manifest.prediction_cutoff.trim().is_empty()
            && !manifest.development.is_null(),
        "manifest needs a target, prediction cutoff and development snapshot"
    );
    let selected = paths
        .iter()
        .map(std::fs::canonicalize)
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut ids = BTreeSet::new();
    let mut columns = BTreeSet::new();
    columns.insert(manifest.target.as_str());
    for column in &manifest.baseline {
        ensure!(
            !column.trim().is_empty() && columns.insert(column),
            "duplicate or empty baseline column {column}"
        );
    }
    for feature in &manifest.features {
        let id = &feature.id;
        ensure!(
            !id.trim().is_empty()
                && !id.contains(['/', '\\'])
                && ids.insert(id)
                && !feature.name.trim().is_empty(),
            "invalid or duplicate feature ID {id}"
        );
        let owner = Path::new(&feature.owner);
        ensure!(
            repo_relative(owner),
            "feature {id} needs a repository-relative owner"
        );
        let file = root
            .join(owner)
            .canonicalize()
            .with_context(|| format!("cannot read owner of feature {id}"))?;
        ensure!(
            file.is_file()
                && file.starts_with(root)
                && selected.iter().any(|path| file.starts_with(path)),
            "owner of {id} is outside the selected paths"
        );
        if feature.kind == FeatureKind::Feature {
            let column = feature
                .column
                .as_deref()
                .with_context(|| format!("feature {id} needs an output column"))?;
            ensure!(
                !column.trim().is_empty() && columns.insert(column),
                "duplicate or empty feature output column {column}"
            );
        }
        if let Some(rolling) = &feature.rolling {
            ensure!(
                feature.node.is_some() && !rolling.source.trim().is_empty(),
                "rolling feature {id} needs a graph node and source"
            );
        }
    }
    Ok(())
}

fn feature_nodes<'a>(
    features: &'a [Feature],
    graph: &'a Value,
) -> Result<Vec<(&'a Feature, &'a Value)>> {
    let mut resolved = Vec::new();
    for feature in features {
        let Some(name) = &feature.node else { continue };
        let matches: Vec<_> = graph["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|n| n["name"] == name.as_str())
            .collect();
        ensure!(
            matches.len() == 1 && matches[0]["file"] == feature.owner.as_str(),
            "graph node {name} must resolve once in owner {}",
            feature.owner
        );
        resolved.push((feature, matches[0]));
    }
    Ok(resolved)
}

fn position(owner: &str, node: &Node<'_>) -> Place {
    let at = node.start_pos();
    Place {
        column: Some(at.column(node) + 1),
        ..Place::line(owner, at.line() + 1)
    }
}

fn finding(kind: Kind, key: String, places: Vec<Place>, evidence: String) -> Finding<Kind> {
    Finding {
        kind,
        key,
        places,
        evidence,
        weight: 1,
    }
}

fn receiver<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    node.field("function")?.field("object")
}

fn positional<'t>(call: &Node<'t>, index: usize) -> Option<Node<'t>> {
    call.field("arguments")?
        .named_children()
        .filter(|node| node.kind() != "keyword_argument")
        .nth(index)
}

fn argument<'t>(call: &Node<'t>, index: usize, key: &str) -> Option<Node<'t>> {
    syntax::keyword_arg(call, key).or_else(|| positional(call, index))
}

fn scope_start(node: &Node<'_>) -> usize {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind().as_ref(),
            "function_definition" | "class_definition" | "lambda"
        ) {
            return parent.range().start + 1;
        }
        current = parent.parent();
    }
    0
}

fn prior_binding<'t>(root: &Node<'t>, name: &Node<'t>) -> Option<Node<'t>> {
    if name.kind() != "identifier" {
        return None;
    }
    root.dfs()
        .filter(|node| {
            node.kind() == "assignment"
                && node.range().end <= name.range().start
                && scope_start(node) == scope_start(name)
                && node
                    .field("left")
                    .is_some_and(|left| left.text() == name.text())
        })
        .last()?
        .field("right")
}

fn call_is(node: &Node<'_>, name: &str) -> bool {
    called(node).is_some_and(|called| called == name)
}

fn body_expression<'t>(function: &Node<'t>) -> Option<Node<'t>> {
    let body = function.field("body")?;
    let returned = body
        .named_children()
        .filter(|n| n.kind() != "comment")
        .last()?;
    (returned.kind() == "return_statement")
        .then(|| returned.named_children().next())
        .flatten()
}

fn positive_shift(call: &Node<'_>) -> bool {
    call_is(call, "shift")
        && syntax::keyword_arg(call, "freq").is_none()
        && syntax::keyword_arg(call, "axis").is_none_or(|axis| {
            axis.text() == "0" || syntax::string_literal(&axis).as_deref() == Some("index")
        })
        && positional(call, 1).is_none()
        && argument(call, 0, "periods")
            .is_none_or(|period| period.text().parse::<u64>().is_ok_and(|n| n > 0))
}

fn trailing_roll(call: &Node<'_>) -> bool {
    call_is(call, "rolling")
        && syntax::keyword_arg(call, "center").is_none_or(|center| center.text() == "False")
        && positional(call, 2).is_none()
}

fn grouping(call: &Node<'_>, keys: &[String]) -> bool {
    call_is(call, "groupby")
        && argument(call, 0, "by").is_some_and(|arg| {
            let names = if matches!(arg.kind().as_ref(), "list" | "tuple") {
                arg.named_children().collect()
            } else {
                vec![arg]
            };
            names
                .into_iter()
                .filter_map(|node| {
                    syntax::string_literal(&node)
                        .or_else(|| {
                            node.field("subscript")
                                .and_then(|key| syntax::string_literal(&key))
                        })
                        .or_else(|| (node.kind() == "identifier").then(|| node.text().into_owned()))
                })
                .collect::<Vec<_>>()
                == keys
        })
}

fn roll_input(node: &Node<'_>, source: &str, alias: Option<&str>) -> bool {
    if node.kind() == "identifier" {
        return node.text() == source || alias.is_some_and(|alias| node.text() == alias);
    }
    if node.kind() == "subscript"
        && node
            .field("subscript")
            .and_then(|key| syntax::string_literal(&key))
            .is_some_and(|key| key == source)
    {
        return true;
    }
    node.field("value")
        .or_else(|| receiver(node))
        .is_some_and(|base| roll_input(&base, source, alias))
}

fn grouped_receiver(input: &Node<'_>, keys: &[String]) -> bool {
    grouping(input, keys)
        || (input.kind() == "subscript"
            && input
                .field("value")
                .is_some_and(|base| grouping(&base, keys)))
}

fn enclosing_transform(roll: &Node<'_>, spec: &Rolling) -> Option<(Option<String>, bool)> {
    let mut current = roll.parent();
    while let Some(node) = current {
        if node.kind() == "lambda" {
            let call = node.parent()?.parent()?;
            let group = receiver(&call)?;
            let base = group.field("value").unwrap_or_else(|| group.clone());
            if call_is(&call, "transform") && call_is(&base, "groupby") {
                let alias = roll_input(&group, &spec.source, None)
                    .then(|| node.field("parameters")?.named_children().next())
                    .flatten()
                    .map(|param| param.text().into_owned());
                return Some((alias, grouped_receiver(&group, &spec.group_keys)));
            }
            return None;
        }
        current = node.parent();
    }
    None
}

fn rolling_findings(
    function: &Node<'_>,
    feature: &Feature,
    spec: &Rolling,
    out: &mut Vec<Finding<Kind>>,
) {
    let Some(returned) = body_expression(function) else {
        return;
    };
    let expression = prior_binding(function, &returned).unwrap_or(returned);
    for reduction in expression.dfs().filter(|call| {
        called(call).is_some_and(|name| {
            matches!(
                name.as_str(),
                "mean" | "median" | "sum" | "min" | "max" | "std" | "var" | "count" | "quantile"
            )
        })
    }) {
        let Some(roll) = receiver(&reduction).filter(trailing_roll) else {
            continue;
        };
        let Some(input) =
            receiver(&roll).map(|input| prior_binding(function, &input).unwrap_or(input))
        else {
            continue;
        };
        let (alias, grouped_transform) = enclosing_transform(&roll, spec).unwrap_or_default();
        if !roll_input(&input, &spec.source, alias.as_deref()) {
            continue;
        }
        let grouped = grouped_transform || grouped_receiver(&input, &spec.group_keys);
        let exclusive = argument(&roll, 0, "window")
            .and_then(|n| syntax::string_literal(&n))
            .is_some()
            && syntax::keyword_arg(&roll, "closed")
                .and_then(|n| syntax::string_literal(&n))
                .is_some_and(|bound| matches!(bound.as_str(), "left" | "neither"));
        let post_shift = spec.group_keys.is_empty()
            && reduction
                .parent()
                .and_then(|parent| parent.parent())
                .is_some_and(|call| positive_shift(&call));
        if spec.prior_only && !positive_shift(&input) && !exclusive && !post_shift {
            out.push(finding(
                Kind::RollingIncludesCurrent,
                format!("{}\n{}", feature.owner, feature.id),
                vec![position(&feature.owner, &roll)],
                format!(
                    "{} rolls unshifted {} into a prior-only feature.",
                    feature.id, spec.source
                ),
            ));
        }
        if !spec.group_keys.is_empty() && !grouped {
            out.push(finding(
                Kind::RollingCrossesGroups,
                format!("{}\n{}", feature.owner, feature.id),
                vec![position(&feature.owner, &roll)],
                format!(
                    "{} rolls across declared groups {}.",
                    feature.id,
                    spec.group_keys.join(", ")
                ),
            ));
        }
    }
}

fn pure_arithmetic(node: &Node<'_>, dependencies: &[&str]) -> Option<String> {
    match node.kind().as_ref() {
        "identifier" => dependencies
            .contains(&node.text().as_ref())
            .then(|| node.text().into_owned()),
        "integer" | "float" => Some(node.text().into_owned()),
        "parenthesized_expression" => pure_arithmetic(&node.named_children().next()?, dependencies),
        "binary_operator" | "unary_operator" => {
            let operator = node.field("operator")?.text();
            if !matches!(operator.as_ref(), "+" | "-" | "*" | "/" | "//" | "%" | "**") {
                return None;
            }
            let operands = node
                .named_children()
                .map(|n| pure_arithmetic(&n, dependencies))
                .collect::<Option<Vec<_>>>()?;
            Some(format!("({operator} {})", operands.join(" ")))
        }
        _ => None,
    }
}

fn node_arithmetic(function: &Node<'_>, graph_node: &Value) -> Option<String> {
    let body = function.field("body")?;
    if body
        .named_children()
        .filter(|n| n.kind() != "comment")
        .count()
        != 1
    {
        return None;
    }
    let dependencies = graph_node["inputs"]
        .as_array()?
        .iter()
        .map(|input| input["name"].as_str())
        .collect::<Option<Vec<_>>>()?;
    let expression = pure_arithmetic(&body_expression(function)?, &dependencies)?;
    Some(format!("{dependencies:?} {expression}"))
}

fn duplicate_findings(computations: Vec<(&Feature, Place, String)>, out: &mut Vec<Finding<Kind>>) {
    let mut first = BTreeMap::new();
    for (feature, place, expression) in computations {
        if let Some((original, location)) = first.get(&expression) {
            out.push(finding(
                Kind::DuplicateFeatureComputation,
                format!("{}\n{}", feature.owner, feature.id),
                vec![place, Place::clone(location)],
                format!(
                    "{} repeats the arithmetic and dependencies of {original}.",
                    feature.id
                ),
            ));
        } else {
            first.insert(expression, (&feature.id, place));
        }
    }
}

fn imported_name(node: &Node<'_>, imports: &HashMap<String, String>) -> Option<String> {
    let text = node.text();
    let (first, suffix) = text.split_once('.').unwrap_or((&text, ""));
    let imported = imports.get(first)?;
    Some(if suffix.is_empty() {
        imported.clone()
    } else {
        format!("{imported}.{suffix}")
    })
}

fn constructor(call: &Node<'_>, imports: &HashMap<String, String>, qualified: &str) -> bool {
    call.field("function")
        .and_then(|function| imported_name(&function, imports))
        .as_deref()
        == Some(qualified)
}

fn model_input<'t>(
    root: &Node<'t>,
    call: &Node<'t>,
    imports: &HashMap<String, String>,
) -> Option<Node<'t>> {
    let qualified = call
        .field("function")
        .and_then(|func| imported_name(&func, imports));
    if matches!(
        qualified.as_deref(),
        Some(
            "sklearn.model_selection.cross_val_score"
                | "sklearn.model_selection.cross_validate"
                | "sklearn.model_selection.cross_val_predict"
        )
    ) {
        return argument(call, 1, "X");
    }
    if !call_is(call, "fit") {
        return None;
    }
    let model = receiver(call)?;
    let model = prior_binding(root, &model).unwrap_or(model);
    if constructor(&model, imports, "sklearn.model_selection.GridSearchCV") {
        return argument(call, 0, "X");
    }
    None
}

fn encoder_construction<'t>(
    root: &Node<'t>,
    encoder: &Node<'t>,
    imports: &HashMap<String, String>,
) -> bool {
    let binding = prior_binding(root, encoder).unwrap_or_else(|| encoder.clone());
    constructor(&binding, imports, "sklearn.preprocessing.TargetEncoder")
}

fn same_fit_rows(fit: &Node<'_>, transformed: &Node<'_>) -> bool {
    call_is(fit, "fit")
        && argument(fit, 1, "y").is_some()
        && argument(fit, 0, "X")
            .zip(argument(transformed, 0, "X"))
            .is_some_and(|(fit, transformed)| fit.text() == transformed.text())
}

fn fitted_encoder<'t>(root: &Node<'t>, data: &Node<'t>, imports: &HashMap<String, String>) -> bool {
    let Some(encoder) = receiver(data) else {
        return false;
    };
    if call_is(data, "fit_transform") {
        return argument(data, 0, "X").is_some()
            && argument(data, 1, "y").is_some()
            && encoder_construction(root, &encoder, imports);
    }
    if !call_is(data, "transform") {
        return false;
    }
    let binding = prior_binding(root, &encoder).unwrap_or_else(|| encoder.clone());
    if same_fit_rows(&binding, data) {
        return receiver(&binding)
            .is_some_and(|encoder| encoder_construction(root, &encoder, imports));
    }
    encoder_construction(root, &encoder, imports)
        && root.dfs().any(|fit| {
            fit.range().end < data.range().start
                && scope_start(&fit) == scope_start(data)
                && same_fit_rows(&fit, data)
                && receiver(&fit).is_some_and(|fitted| fitted.text() == encoder.text())
        })
}

fn encoder_findings(root: &Node<'_>, owner: &str, out: &mut Vec<Finding<Kind>>) {
    let mut scoped_imports = BTreeMap::<usize, HashMap<String, String>>::new();
    for import in root.dfs().filter(|node| {
        matches!(
            node.kind().as_ref(),
            "import_statement" | "import_from_statement"
        )
    }) {
        scoped_imports
            .entry(scope_start(&import))
            .or_default()
            .extend(imports_of(&import));
    }
    let module = scoped_imports.remove(&0).unwrap_or_default();
    for call in root.dfs().filter(|node| node.kind() == "call") {
        let local = scoped_imports.get(&scope_start(&call)).map(|local| {
            let mut imports = module.clone();
            imports.extend(local.clone());
            imports
        });
        let imports = local.as_ref().unwrap_or(&module);
        let Some(data) = model_input(root, &call, imports) else {
            continue;
        };
        let encoded = prior_binding(root, &data).unwrap_or(data);
        if fitted_encoder(root, &encoded, imports) {
            let identity = call
                .field("function")
                .map_or_else(String::new, |node| node.text().split_whitespace().collect());
            out.push(finding(
                Kind::TargetEncoderBeforeOuterCv,
                format!("{owner}\n{identity}"),
                vec![position(owner, &encoded), position(owner, &call)],
                "TargetEncoder was fitted on all rows before the outer validation split."
                    .to_owned(),
            ));
        }
    }
}
