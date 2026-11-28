use anyhow::Result;
use ast_grep_config::{GlobalRules, RuleConfig};
use ast_grep_language::SupportLang;
use clap::ValueEnum;

use crate::ledger::{FindingPlace, LedgerFinding};
use crate::syntax::{self, Node};

use super::Kind;

const RULES: [&str; 4] = [
    include_str!("../../rules/tsreview/random-time-split.yml"),
    include_str!("../../rules/tsreview/ungrouped-lag.yml"),
    include_str!("../../rules/tsreview/missing-future-exog.yml"),
    include_str!("../../rules/tsreview/mape-on-intermittent.yml"),
];

pub(super) fn configs() -> Result<Vec<RuleConfig<SupportLang>>> {
    RULES.into_iter().try_fold(Vec::new(), |mut rules, yaml| {
        rules.extend(ast_grep_config::from_yaml_string(
            yaml,
            &GlobalRules::default(),
        )?);
        Ok(rules)
    })
}

fn called<'tree>(call: &Node<'tree>) -> Option<Node<'tree>> {
    let function = call.field("function")?;
    if function.kind() == "attribute" {
        function.field("attribute")
    } else {
        Some(function)
    }
}

fn is_call(node: &Node<'_>, name: &str) -> bool {
    called(node).is_some_and(|function| function.text() == name)
}

fn uses_key(node: &Node<'_>, key: &str) -> bool {
    node.dfs()
        .any(|node| syntax::string_literal(&node).is_some_and(|value| value == key))
}

fn positional<'tree>(call: &Node<'tree>) -> Option<Node<'tree>> {
    call.field("arguments")?
        .named_children()
        .find(|node| node.kind() != "keyword_argument")
}

fn simple_frame(node: &Node<'_>) -> Option<String> {
    match node.kind().as_ref() {
        "identifier" => Some(node.text().into_owned()),
        "subscript" => simple_frame(&node.field("value")?),
        "call" => simple_frame(&node.field("function")?.field("object")?),
        "attribute" => simple_frame(&node.field("object")?),
        _ => None,
    }
}

fn sorted_by_time(call: &Node<'_>) -> bool {
    is_call(call, "sort_values") && uses_key(call, "ds")
}

fn time_split<'tree>(source: &Node<'tree>, call: &Node<'tree>) -> bool {
    let frame = if is_call(call, "sample") {
        if syntax::keyword_arg(call, "frac").is_none() {
            return false;
        }
        call.field("function")
            .and_then(|function| function.field("object"))
    } else if is_call(call, "KFold") || is_call(call, "ShuffleSplit") {
        if is_call(call, "KFold")
            && syntax::keyword_arg(call, "shuffle").is_none_or(|n| n.text() != "True")
        {
            return false;
        }
        call.parent()
            .filter(|n| n.kind() == "attribute")
            .and_then(|n| n.parent())
            .filter(|n| is_call(n, "split"))
            .and_then(|n| positional(&n))
            .or_else(|| {
                let name = call.parent()?.field("left")?;
                source
                    .dfs()
                    .filter(|node| is_call(node, "split"))
                    .find(|node| {
                        node.field("function")
                            .and_then(|f| f.field("object"))
                            .is_some_and(|receiver| receiver.text() == name.text())
                    })
                    .and_then(|node| positional(&node))
            })
    } else {
        if syntax::keyword_arg(call, "shuffle").is_some_and(|node| node.text() == "False") {
            return false;
        }
        positional(call)
    };
    let Some(frame) = frame else { return false };
    let name = simple_frame(&frame);
    let forecast = [
        "StatsForecast",
        "MLForecast",
        "SARIMAX",
        "ARIMA",
        "VAR",
        "ExponentialSmoothing",
    ]
    .iter()
    .any(|model| source.dfs().any(|node| is_call(&node, model)));
    frame.dfs().any(|node| sorted_by_time(&node))
        || source.dfs().any(|node| {
            let receiver = node
                .field("function")
                .and_then(|function| function.field("object"));
            (sorted_by_time(&node) && receiver.as_ref().and_then(simple_frame) == name)
                || (forecast
                    && is_call(&node, "fit")
                    && (positional(&node).as_ref().and_then(simple_frame) == name
                        || syntax::keyword_arg(&node, "df")
                            .as_ref()
                            .and_then(simple_frame)
                            == name))
                || (node.kind() == "assignment"
                    && uses_key(&node, "ds")
                    && node.field("left").as_ref().and_then(simple_frame) == name)
        })
}

fn grouped_in_time(node: &Node<'_>, key_alias: Option<&str>) -> (bool, bool) {
    let calls = node.dfs().filter(|node| node.kind() == "call");
    let mut sorted = false;
    let mut grouped = false;
    for call in calls {
        sorted |= is_call(&call, "sort_values")
            && ((uses_key(&call, "unique_id") && uses_key(&call, "ds"))
                || key_alias
                    .is_some_and(|key| positional(&call).is_some_and(|arg| arg.text() == key)));
        grouped |= is_call(&call, "groupby") && uses_key(&call, "unique_id");
    }
    (sorted, grouped)
}

fn unsafe_forecast_lags(source: &Node<'_>, constructor: &Node<'_>) -> bool {
    if syntax::keyword_arg(constructor, "lags").is_none()
        && syntax::keyword_arg(constructor, "lag_transforms").is_none()
    {
        return false;
    }
    let Some(model) = constructor.parent().and_then(|node| node.field("left")) else {
        return false;
    };
    source.dfs().filter(|node| is_call(node, "fit")).any(|fit| {
        if !method_on(&fit, &model.text()) {
            return false;
        }
        let Some(frame) = syntax::keyword_arg(&fit, "df").or_else(|| positional(&fit)) else {
            return false;
        };
        let Some(name) = simple_frame(&frame) else {
            return false;
        };
        let defined_panel = source.dfs().any(|node| {
            node.kind() == "assignment"
                && node.field("left").is_some_and(|left| left.text() == name)
                && node.field("right").is_some_and(|right| {
                    uses_key(&right, "unique_id")
                        && uses_key(&right, "ds")
                        && !grouped_in_time(&right, None).0
                })
        });
        defined_panel
            && !grouped_in_time(&frame, None).0
            && !source.dfs().any(|node| {
                ["shift", "rolling", "diff", "sort_values"]
                    .iter()
                    .any(|op| is_call(&node, op))
                    && node
                        .field("function")
                        .and_then(|function| function.field("object"))
                        .as_ref()
                        .and_then(simple_frame)
                        .as_deref()
                        == Some(name.as_str())
            })
    })
}

fn method_on(call: &Node<'_>, name: &str) -> bool {
    call.field("function")
        .and_then(|function| function.field("object"))
        .is_some_and(|receiver| receiver.text() == name)
}

fn ungrouped_lag(source: &Node<'_>, call: &Node<'_>) -> bool {
    if !(uses_key(source, "unique_id") && uses_key(source, "ds")) {
        return false;
    }
    if is_call(call, "MLForecast") {
        return unsafe_forecast_lags(source, call);
    }
    if is_call(call, "transform") {
        return source.dfs().any(|node| is_call(&node, "GroupedArray"))
            && !source.dfs().any(|node| {
                is_call(&node, "GroupedArray") && node.dfs().any(|part| part.text() == "0")
                    || is_call(&node, "concatenate") && node.dfs().any(|part| part.text() == "0")
            });
    }
    let key_alias = source.dfs().find_map(|node| {
        let right = node.field("right")?;
        (node.kind() == "assignment"
            && matches!(right.kind().as_ref(), "list" | "tuple")
            && uses_key(&right, "unique_id")
            && uses_key(&right, "ds"))
        .then(|| node.field("left").map(|key| key.text().into_owned()))
        .flatten()
    });
    let (mut sorted, mut grouped) = grouped_in_time(call, key_alias.as_deref());
    let receiver = call
        .field("function")
        .and_then(|function| function.field("object"));
    let mut name = receiver.as_ref().and_then(simple_frame);
    let mut statement = call.clone();
    while let Some(parent) = statement.parent() {
        if matches!(parent.kind().as_ref(), "block" | "module") {
            break;
        }
        statement = parent;
    }
    while let Some(previous) = statement.prev() {
        for assignment in previous
            .named_children()
            .filter(|node| node.kind() == "assignment")
        {
            if assignment.field("left").as_ref().and_then(simple_frame) == name
                && let Some(right) = assignment.field("right")
            {
                let (was_sorted, was_grouped) = grouped_in_time(&right, key_alias.as_deref());
                sorted |= was_sorted;
                grouped |= was_grouped;
                name = simple_frame(&right);
            }
        }
        statement = previous;
    }
    !(sorted && grouped)
}

fn declares_exogenous_columns(source: &Node<'_>) -> bool {
    source.dfs().any(|node| {
        node.kind() == "dictionary"
            && uses_key(&node, "ds")
            && uses_key(&node, "y")
            && node.named_children().any(|pair| {
                pair.field("key")
                    .as_ref()
                    .and_then(syntax::string_literal)
                    .is_some_and(|key| !matches!(key.as_str(), "unique_id" | "ds" | "y"))
            })
    })
}

fn builds_future_rows(
    source: &Node<'_>,
    supplied: &Node<'_>,
    call: &Node<'_>,
    model: &str,
) -> bool {
    let horizon = syntax::keyword_arg(call, "h").or_else(|| positional(call));
    let future_builder = |node: &Node<'_>| {
        (is_call(node, "make_future_dataframe") && method_on(node, model))
            || (is_call(node, "date_range")
                && uses_key(source, "unique_id")
                && syntax::keyword_arg(node, "periods").is_some_and(|periods| {
                    horizon
                        .as_ref()
                        .is_some_and(|horizon| periods.text() == horizon.text())
                }))
    };
    if supplied.dfs().any(|node| future_builder(&node)) {
        return true;
    }
    let name = simple_frame(supplied);
    source
        .dfs()
        .filter(|node| {
            node.kind() == "assignment"
                && node.field("left").as_ref().and_then(simple_frame) == name
                && node.start_pos().line() < call.start_pos().line()
        })
        .filter_map(|node| node.field("right"))
        .any(|definition| {
            definition.dfs().any(|node| future_builder(&node))
                || source.dfs().any(|node| {
                    node.kind() == "assignment"
                        && node.start_pos().line() < definition.start_pos().line()
                        && node.field("left").is_some_and(|left| {
                            definition.dfs().any(|part| {
                                part.kind() == "identifier" && part.text() == left.text()
                            })
                        })
                        && node
                            .field("right")
                            .is_some_and(|right| right.dfs().any(|part| future_builder(&part)))
                })
        })
}

fn missing_future_exog(source: &Node<'_>, call: &Node<'_>) -> bool {
    let receiver = call
        .field("function")
        .and_then(|function| function.field("object"));
    let Some(receiver) = receiver else {
        return false;
    };
    let model = receiver.text();
    let fit = source.dfs().any(|node| {
        is_call(&node, "fit")
            && method_on(&node, &model)
            && (["dynamic_features", "exog", "X", "X_df"]
                .iter()
                .any(|arg| syntax::keyword_arg(&node, arg).is_some())
                || (syntax::keyword_arg(&node, "static_features")
                    .is_some_and(|arg| arg.text() == "[]")
                    && declares_exogenous_columns(source)))
    });
    let supplied = syntax::keyword_arg(call, "X_df").filter(|value| value.text() != "None");
    if !fit && supplied.is_none() {
        return false;
    }
    let provisioned = source
        .dfs()
        .any(|node| is_call(&node, "get_missing_future") && method_on(&node, &model))
        || supplied.is_some_and(|supplied| builds_future_rows(source, &supplied, call, &model));
    !provisioned
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Demand {
    Intermittent,
    Unclassified,
}

fn mape_on_intermittent(source: &Node<'_>, call: &Node<'_>, demand: Demand) -> bool {
    let mape = is_call(call, "mape")
        || call.field("arguments").is_some_and(|arguments| {
            arguments
                .dfs()
                .any(|part| part.kind() == "identifier" && part.text() == "mape")
        });
    mape && (demand == Demand::Intermittent
        || source.dfs().any(|node| {
            if node.kind() != "pair" && node.kind() != "assignment" {
                return false;
            }
            let (key, value) = if node.kind() == "pair" {
                ("key", "value")
            } else {
                ("left", "right")
            };
            node.field(key)
                .is_some_and(|part| uses_key(&part, "y") || part.text() == "y")
                && node.field(value).is_some_and(|part| {
                    part.dfs().any(|number| {
                        matches!(number.kind().as_ref(), "integer" | "float")
                            && number.text().parse::<f64>() == Ok(0.0)
                    })
                })
        }))
}

pub(super) fn findings(
    configs: &[RuleConfig<SupportLang>],
    text: &str,
    shown: &str,
    tracked: &str,
    demand: Demand,
) -> Vec<LedgerFinding<Kind>> {
    let tree = syntax::parse(text);
    let root = tree.root();
    let mut findings = Vec::new();
    for config in configs {
        let Ok(kind) = Kind::from_str(&config.id, false) else {
            continue;
        };
        for matched in root.find_all(&config.matcher) {
            let call = matched.get_node();
            let violates = match kind {
                Kind::RandomTimeSplit => time_split(&root, call),
                Kind::UngroupedLag => ungrouped_lag(&root, call),
                Kind::MissingFutureExog => missing_future_exog(&root, call),
                Kind::MapeOnIntermittent => mape_on_intermittent(&root, call, demand),
                _ => false,
            };
            if violates {
                findings.push(LedgerFinding {
                    kind,
                    key: format!("{tracked}\n{}", call.text()),
                    places: vec![FindingPlace::line(shown, call.start_pos().line() + 1)],
                    evidence: format!("{} {}", call.text(), config.message),
                    weight: 1,
                });
            }
        }
    }
    findings
}

pub(super) fn cv_windows(text: &str) -> Vec<serde_json::Value> {
    syntax::parse(text)
        .root()
        .dfs()
        .filter(|node| is_call(node, "cross_validation"))
        .map(|node| {
            let option =
                |name| syntax::keyword_arg(&node, name).map(|value| value.text().into_owned());
            serde_json::json!({
                "line": node.start_pos().line() + 1,
                "n_windows": option("n_windows"),
                "cutoffs": option("cutoffs"),
                "step_size": option("step_size"),
                "h": option("h")
            })
        })
        .collect()
}
