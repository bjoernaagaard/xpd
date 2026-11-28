use std::collections::BTreeMap;

use crate::ledger::python_ast::{
    assignments, call_receiver as receiver, called_method_name as called, enclosing_scope as owner,
    identifier, is_python_scope as scope, local_nodes, positional_call_args as args, resolved_call,
    scope_qualified_name as qualified,
};
use crate::ledger::{FindingPlace, LedgerFinding};
use crate::names::{is_bound, is_parameter_name};
use crate::syntax::{Node, keyword_arg, string_literal};

use super::Kind;

type Pair = (String, String);

fn pair(args: &[Node<'_>]) -> Option<Pair> {
    Some((identifier(args.first()?)?, identifier(args.get(1)?)?))
}

fn split_pair(left: &Node<'_>, right: &Node<'_>) -> Option<(Pair, Pair)> {
    if !matches!(left.kind().as_ref(), "pattern_list" | "tuple_pattern")
        || resolved_call(right)?.as_str() != "sklearn.model_selection.train_test_split"
    {
        return None;
    }
    let names: Vec<_> = left.named_children().collect();
    let [a, b, c, d] = names.as_slice() else {
        return None;
    };
    Some((
        (identifier(a)?, identifier(c)?),
        (identifier(b)?, identifier(d)?),
    ))
}

fn same_local_binding(use_site: &Node<'_>, name: &str, definition: &Node<'_>) -> bool {
    owner(use_site).is_some_and(|scope| {
        let mut binds = local_nodes(&scope).filter(|node| {
            node.kind() == "identifier"
                && node.text() == name
                && (is_bound(node) || is_parameter_name(node))
        });
        binds.next().is_some_and(|bound| {
            definition.range().contains(&bound.range().start)
                && bound.range().start < use_site.range().start
        }) && binds.next().is_none()
    })
}

fn constructor_key(call: &Node<'_>) -> Option<String> {
    if matches!(called(call)?.as_str(), "fit" | "conformalize") {
        return constructor_key(&receiver(call)?);
    }
    resolved_call(call)
}

fn dummy(name: &str) -> bool {
    matches!(
        name,
        "sklearn.dummy.DummyClassifier" | "sklearn.dummy.DummyRegressor"
    )
}

fn estimator(name: &str) -> bool {
    let Some((module, symbol)) = name.rsplit_once('.') else {
        return false;
    };
    let sklearn_modules = [
        "calibration",
        "compose",
        "discriminant_analysis",
        "ensemble",
        "gaussian_process",
        "kernel_ridge",
        "linear_model",
        "multiclass",
        "multioutput",
        "naive_bayes",
        "neighbors",
        "neural_network",
        "semi_supervised",
        "svm",
        "tree",
        "dummy",
    ];
    if let Some(part) = module.strip_prefix("sklearn.") {
        return sklearn_modules.contains(&part)
            && symbol.starts_with(char::is_uppercase)
            && !matches!(symbol, "ColumnTransformer" | "ClassifierChain" if part == "compose");
    }
    (module.starts_with("mapie.")
        && (symbol.ends_with("Classifier") || symbol.ends_with("Regressor")))
        || (module.starts_with("xgboost")
            && matches!(
                symbol,
                "XGBClassifier"
                    | "XGBRegressor"
                    | "XGBRanker"
                    | "XGBRFClassifier"
                    | "XGBRFRegressor"
            ))
        || (module.starts_with("lightgbm")
            && matches!(symbol, "LGBMClassifier" | "LGBMRegressor" | "LGBMRanker"))
}

fn model_constructor(call: &Node<'_>, scope: &Node<'_>) -> Option<String> {
    let name = constructor_key(call)?;
    if matches!(
        name.as_str(),
        "sklearn.pipeline.Pipeline" | "sklearn.pipeline.make_pipeline"
    ) {
        let contains_estimator = call.dfs().any(|node| {
            if node.kind() == "call" {
                return constructor_key(&node).is_some_and(|name| estimator(&name));
            }
            let Some(name) = identifier(&node) else {
                return false;
            };
            local_nodes(scope)
                .filter_map(|node| assignments(&node))
                .any(|(left, right)| {
                    left.text() == name
                        && same_local_binding(&node, &name, &left)
                        && constructor_key(&right).is_some_and(|name| estimator(&name))
                })
        });
        return contains_estimator.then_some(name);
    }
    (estimator(&name) && !dummy(&name)).then_some(name)
}

fn numeric(call: &Node<'_>, key: &str) -> Option<f64> {
    let value = keyword_arg(call, key)?;
    matches!(value.kind().as_ref(), "integer" | "float").then_some(())?;
    value.text().parse().ok()
}

fn literal(call: &Node<'_>, key: &str) -> Option<String> {
    string_literal(&keyword_arg(call, key)?)
}

fn positive_integer(call: &Node<'_>, name: &str) -> bool {
    keyword_arg(call, name).is_some_and(|node| {
        node.kind() == "integer" && node.text().parse::<u64>().is_ok_and(|value| value > 0)
    })
}

fn grouped_splitter(name: &str) -> bool {
    matches!(
        name,
        "GroupKFold"
            | "StratifiedGroupKFold"
            | "GroupShuffleSplit"
            | "LeaveOneGroupOut"
            | "LeavePGroupsOut"
    )
}

fn split_conflict(call: &Node<'_>, nodes: &[Node<'_>]) -> bool {
    if keyword_arg(call, "shuffle").is_some_and(|value| value.kind() != "true")
        || args(call).iter().any(|arg| arg.kind().ends_with("splat"))
    {
        return false;
    }
    let Some(original) = pair(&args(call)) else {
        return false;
    };
    nodes
        .iter()
        .filter(|other| called(other).as_deref() == Some("split"))
        .any(|other| {
            let Some(splitter) = receiver(other) else {
                return false;
            };
            let argument = args(other);
            if pair(&argument).is_some_and(|pair| pair != original)
                || argument.first().and_then(identifier).as_deref() != Some(original.0.as_str())
                || (argument.get(1).is_some() && pair(&argument).is_none())
            {
                return false;
            }
            let split_name = if let Some(name) = identifier(&splitter) {
                nodes
                    .iter()
                    .filter_map(assignments)
                    .find_map(|(left, right)| {
                        (left.text() == name && same_local_binding(&splitter, &name, &left))
                            .then(|| resolved_call(&right))
                            .flatten()
                    })
            } else {
                resolved_call(&splitter)
            };
            let Some(split_name) = split_name else {
                return false;
            };
            let Some(kind) = split_name.strip_prefix("sklearn.model_selection.") else {
                return false;
            };
            kind == "TimeSeriesSplit"
                || (grouped_splitter(kind)
                    && (keyword_arg(other, "groups").or_else(|| argument.get(2).cloned()))
                        .is_some_and(|groups| groups.kind() == "identifier"))
        })
}

fn quantile_fit(call: &Node<'_>) -> Option<f64> {
    let key = match constructor_key(call)?.as_str() {
        "sklearn.ensemble.GradientBoostingRegressor"
            if literal(call, "loss")?.as_str() == "quantile" =>
        {
            "alpha"
        }
        "lightgbm.LGBMRegressor" | "lightgbm.sklearn.LGBMRegressor"
            if literal(call, "objective")?.as_str() == "quantile" =>
        {
            "alpha"
        }
        "xgboost.XGBRegressor" | "xgboost.sklearn.XGBRegressor"
            if literal(call, "objective")?.as_str() == "reg:quantileerror" =>
        {
            "quantile_alpha"
        }
        _ => return None,
    };
    numeric(call, key)
}

fn model_pair_prediction<'t>(
    value: &Node<'t>,
    nodes: &[Node<'t>],
) -> Option<(String, String, String)> {
    let prediction = if let Some(symbol) = identifier(value) {
        nodes
            .iter()
            .filter_map(assignments)
            .find_map(|(left, right)| {
                (left.text() == symbol && same_local_binding(value, &symbol, &left))
                    .then_some(right)
            })?
    } else {
        value.clone()
    };
    let method = called(&prediction)?;
    if !matches!(
        method.as_str(),
        "predict" | "predict_proba" | "decision_function"
    ) {
        return None;
    }
    let model = identifier(&receiver(&prediction)?)?;
    let features = identifier(args(&prediction).first()?)?;
    Some((model, features, method))
}

fn metric_score<'t>(call: &Node<'t>, nodes: &[Node<'t>]) -> Option<(String, Pair, String)> {
    if called(call).as_deref() == Some("score") {
        return Some((
            identifier(&receiver(call)?)?,
            pair(&args(call))?,
            "score".to_owned(),
        ));
    }
    let name = resolved_call(call)?;
    if !name.starts_with("sklearn.metrics.") {
        return None;
    }
    let args = args(call);
    let target = identifier(args.first()?)?;
    let (model, features, method) = model_pair_prediction(args.get(1)?, nodes)?;
    let options: Vec<_> = call
        .field("arguments")?
        .named_children()
        .filter(|item| item.kind() == "keyword_argument")
        .map(|item| item.text().into_owned())
        .collect();
    Some((
        model,
        (features, target),
        format!("{name}/{method}/{options:?}"),
    ))
}

fn fitted_on(call: &Node<'_>, nodes: &[Node<'_>], model: &str, train: &Pair) -> bool {
    nodes.iter().any(|fit| {
        fit.kind() == "call"
            && fit.range().start < call.range().start
            && called(fit).as_deref() == Some("fit")
            && receiver(fit).and_then(|node| identifier(&node)).as_deref() == Some(model)
            && pair(&args(fit)).as_ref() == Some(train)
    })
}

struct LocalReview<'a, 't> {
    file: &'a str,
    nodes: Vec<Node<'t>>,
    findings: Vec<(LedgerFinding<Kind>, Option<String>)>,
}

impl<'t> LocalReview<'_, 't> {
    fn record(&mut self, node: &Node<'_>, kind: Kind, binding: Option<String>, evidence: String) {
        let mut place = FindingPlace::line(self.file, node.start_pos().line() + 1);
        place.column = Some(node.start_pos().column(node) + 1);
        let key = format!(
            "{}\n{}\n{evidence}",
            self.file,
            binding.as_deref().unwrap_or("")
        );
        self.findings.push((
            LedgerFinding {
                kind,
                key,
                places: vec![place],
                evidence,
                weight: 1,
            },
            binding,
        ));
    }

    fn scores(&mut self, call: &Node<'t>) {
        let Some((model, held, metric)) = metric_score(call, &self.nodes) else {
            return;
        };
        let constructor = self
            .nodes
            .iter()
            .filter_map(assignments)
            .find_map(|(left, right)| {
                (left.text() == model && same_local_binding(call, &model, &left)).then_some(right)
            });
        let Some(constructor) = constructor else {
            return;
        };
        let binding = qualified(call, &model);
        if resolved_call(call).as_deref() == Some("sklearn.metrics.mean_pinball_loss")
            && let Some(alpha) = numeric(call, "alpha")
            && let Some(quantile) = quantile_fit(&constructor)
            && alpha.to_bits() != quantile.to_bits()
        {
            self.record(call, Kind::MetricLossMismatch, Some(binding.clone()),
                format!("mean_pinball_loss uses alpha {alpha} for {model}, trained at quantile {quantile}. Score the intended quantile."));
        }
        let Some(model_scope) = owner(call) else {
            return;
        };
        if model_constructor(&constructor, &model_scope).is_none() {
            return;
        }
        let train = self
            .nodes
            .iter()
            .filter_map(assignments)
            .find_map(|(left, right)| {
                let (train, test) = split_pair(&left, &right)?;
                (test == held
                    && left.range().start < call.range().start
                    && same_local_binding(call, &test.0, &left)
                    && same_local_binding(call, &test.1, &left))
                .then_some(train)
            });
        let Some(train) = train else {
            return;
        };
        if !fitted_on(call, &self.nodes, &model, &train) {
            return;
        }
        let baseline = self.nodes.iter().any(|score| {
            let Some((other, test, scorer)) = metric_score(score, &self.nodes) else {
                return false;
            };
            test == held
                && scorer == metric
                && fitted_on(score, &self.nodes, &other, &train)
                && self
                    .nodes
                    .iter()
                    .filter_map(assignments)
                    .any(|(left, right)| {
                        left.text() == other
                            && same_local_binding(score, &other, &left)
                            && constructor_key(&right).is_some_and(|name| dummy(&name))
                    })
        });
        if !baseline {
            self.record(call, Kind::BaselineNotObserved, Some(binding),
                format!("{model} scores ({}, {}) with no local dummy comparison on the same fit rows, held-out rows and metric.", held.0, held.1));
        }
    }
}

pub(super) struct ModelStaticReview {
    pub(super) bindings: BTreeMap<String, Vec<usize>>,
    pub(super) findings: Vec<(LedgerFinding<Kind>, Option<String>)>,
}

pub(super) fn inspect_models(root: &Node<'_>, file: &str) -> ModelStaticReview {
    let mut bindings: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut findings = Vec::new();
    for scope in root.dfs().filter(scope) {
        let mut review = LocalReview {
            file,
            nodes: local_nodes(&scope).collect(),
            findings: Vec::new(),
        };
        for item in review.nodes.clone() {
            if let Some((left, right)) = assignments(&item)
                && let Some(name) = identifier(&left)
                && model_constructor(&right, &scope).is_some()
            {
                bindings
                    .entry(qualified(&item, &name))
                    .or_default()
                    .push(item.start_pos().line() + 1);
            }
            if item.kind() != "call" {
                continue;
            }
            if resolved_call(&item).as_deref() == Some("sklearn.model_selection.train_test_split")
                && split_conflict(&item, &review.nodes)
            {
                review.record(&item, Kind::SplitMismatch, None, "A shuffled holdout splits the same inputs as a forward or group-aware splitter in this scope.".to_owned());
            }
            review.scores(&item);
        }
        findings.extend(review.findings);
    }
    ModelStaticReview { bindings, findings }
}

fn test_named(name: &str) -> bool {
    name.split(|letter: char| !letter.is_ascii_alphabetic())
        .any(|word| word.eq_ignore_ascii_case("test"))
        && !name.to_ascii_lowercase().contains("train")
}

fn evaluation_pair(fit: &Node<'_>, library: &str) -> Option<Pair> {
    if let Some(eval) = keyword_arg(fit, "eval_set") {
        if eval.kind() != "list" {
            return None;
        }
        let entries: Vec<_> = eval
            .named_children()
            .filter(|item| item.kind() != "comment")
            .collect();
        let [entry] = entries.as_slice() else {
            return None;
        };
        if entry.kind() != "tuple" {
            return None;
        }
        let values: Vec<_> = entry.named_children().collect();
        return (values.len() == 2).then(|| pair(&values)).flatten();
    }
    (library == "lightgbm").then_some(())?;
    Some((
        identifier(&keyword_arg(fit, "eval_X")?)?,
        identifier(&keyword_arg(fit, "eval_y")?)?,
    ))
}

/// Returns the resolved test pair when a booster fit uses it for stopping and later scores it.
pub(crate) fn early_stop_test_pair(fit: &Node<'_>, library: &str) -> Option<(String, String)> {
    let model = identifier(&receiver(fit)?)?;
    let test = evaluation_pair(fit, library)?;
    if !test_named(&test.0) || !test_named(&test.1) {
        return None;
    }
    let scope = owner(fit)?;
    let nodes: Vec<_> = local_nodes(&scope).collect();
    let constructor = nodes
        .iter()
        .filter_map(assignments)
        .find_map(|(left, right)| {
            (left.text() == model && same_local_binding(fit, &model, &left)).then_some(right)
        })?;
    let constructor_name = resolved_call(&constructor)?;
    let symbol = constructor_name.rsplit('.').next()?;
    let stopping = match library {
        "xgboost" => {
            matches!(symbol, "XGBClassifier" | "XGBRegressor")
                && constructor_name.starts_with("xgboost.")
                && positive_integer(&constructor, "early_stopping_rounds")
        }
        "lightgbm" => {
            matches!(symbol, "LGBMClassifier" | "LGBMRegressor")
                && constructor_name.starts_with("lightgbm.")
                && keyword_arg(fit, "callbacks").is_some_and(|callbacks| {
                    matches!(callbacks.kind().as_ref(), "list" | "tuple")
                        && callbacks.named_children().any(|callback| {
                            matches!(
                                resolved_call(&callback).as_deref(),
                                Some(
                                    "lightgbm.early_stopping" | "lightgbm.callback.early_stopping"
                                )
                            )
                        })
                })
        }
        _ => false,
    };
    if !stopping {
        return None;
    }
    let split = nodes.iter().filter_map(assignments).any(|(left, right)| {
        split_pair(&left, &right).is_some_and(|(_, held)| held == test)
            && same_local_binding(fit, &test.0, &left)
            && same_local_binding(fit, &test.1, &left)
    });
    let scores_test = nodes.iter().any(|call| {
        call.range().start > fit.range().start
            && called(call).as_deref() == Some("score")
            && receiver(call).and_then(|node| identifier(&node)).as_deref() == Some(&model)
            && pair(&args(call)).as_ref() == Some(&test)
    });
    (split && scores_test).then_some(test)
}

pub(super) fn model_binding_at(root: &Node<'_>, line: usize, column: usize) -> Option<String> {
    let call = root.dfs().find(|node| {
        node.kind() == "call"
            && node.start_pos().line() + 1 == line
            && node.start_pos().column(node) + 1 == column
    })?;
    Some(qualified(&call, &identifier(&receiver(&call)?)?))
}
