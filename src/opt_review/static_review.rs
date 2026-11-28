use crate::ledger::python_ast::{
    assignments, call_receiver as receiver, called_method_name as called, enclosing_scope as owner,
    identifier, is_python_scope as scope, local_nodes, positional_call_args as args, resolved_call,
    resolved_name, scope_qualified_name as qualified,
};
use crate::names::{Binding, is_bound, is_parameter_name, origin};
use crate::syntax::{Node, keyword_arg, string_literal};

pub(super) fn problem_kind(call: &Node<'_>) -> Option<&'static str> {
    let Some(name) = resolved_call(call) else {
        return local_pymoo_problem(call).then_some("pymoo");
    };
    let (module, symbol) = name.rsplit_once('.')?;
    if (module == "cvxpy" || module.starts_with("cvxpy.")) && symbol == "Problem" {
        return Some("cvxpy");
    }
    if module.starts_with("pymoo.") && matches!(symbol, "Problem" | "ElementwiseProblem") {
        return Some("pymoo");
    }
    if name == "pymoo.problems.get_problem" && args(call).first().and_then(string_literal).is_some()
    {
        return Some("pymoo");
    }
    if (module == "skfolio.optimization" || module.starts_with("skfolio.optimization."))
        && matches!(
            symbol,
            "MeanRisk"
                | "RiskBudgeting"
                | "MaximumDiversification"
                | "BenchmarkTracker"
                | "DistributionallyRobustCVaR"
                | "HierarchicalRiskParity"
                | "SchurComplementary"
                | "HierarchicalEqualRiskContribution"
                | "NestedClustersOptimization"
                | "StackingOptimization"
                | "EqualWeighted"
                | "InverseVolatility"
                | "Random"
        )
    {
        return Some("skfolio");
    }
    None
}

fn pymoo_subclass<'t>(call: &Node<'t>) -> Option<Node<'t>> {
    let function = call.field("function")?;
    let name = identifier(&function)?;
    for enclosing in call.ancestors().filter(scope) {
        let mut definitions = local_nodes(&enclosing).filter(|node| {
            matches!(
                node.kind().as_ref(),
                "class_definition" | "function_definition"
            ) && node
                .field("name")
                .is_some_and(|binding| binding.text() == name)
        });
        let definition = definitions.next();
        if definitions.next().is_some()
            || local_nodes(&enclosing).any(|node| {
                node.kind() == "identifier"
                    && node.text() == name
                    && (is_bound(&node) || is_parameter_name(&node))
            })
        {
            return None;
        }
        if let Some(class) = definition {
            return (class.kind() == "class_definition"
                && class.range().start < call.range().start
                && class.field("superclasses").is_some_and(|bases| {
                    bases.named_children().any(|base| {
                        resolved_name(&base).is_some_and(|name| {
                            name.starts_with("pymoo.")
                                && (name.ends_with(".Problem")
                                    || name.ends_with(".ElementwiseProblem"))
                        })
                    })
                }))
            .then_some(class);
        }
    }
    None
}

fn local_pymoo_problem(call: &Node<'_>) -> bool {
    pymoo_subclass(call).is_some()
        && owner(call).is_some_and(|owner| {
            local_nodes(&owner).any(|node| {
                resolved_call(&node).as_deref() == Some("pymoo.optimize.minimize")
                    && keyword_arg(&node, "problem")
                        .or_else(|| args(&node).first().cloned())
                        .is_some_and(|problem| {
                            problem.node_id() == call.node_id()
                                || assigned_value(&problem)
                                    .is_some_and(|value| value.node_id() == call.node_id())
                        })
            })
        })
}

pub(super) fn definition_name(call: &Node<'_>) -> String {
    let definition = call
        .parent()
        .and_then(|parent| {
            if parent.kind() == "attribute"
                && parent
                    .field("attribute")
                    .is_some_and(|name| name.text() == "fit")
            {
                parent.parent()
            } else {
                None
            }
        })
        .unwrap_or_else(|| call.clone());
    let name = definition
        .parent()
        .and_then(|parent| {
            assignments(&parent).and_then(|(left, right)| {
                (right.node_id() == definition.node_id())
                    .then(|| identifier(&left))
                    .flatten()
            })
        })
        .unwrap_or_else(|| {
            definition
                .ancestors()
                .find(|node| {
                    matches!(
                        node.kind().as_ref(),
                        "return_statement" | "expression_statement"
                    )
                })
                .map_or_else(|| "expression".to_owned(), |node| node.kind().into_owned())
        });
    qualified(call, &name)
}

pub(super) fn front_applies(call: &Node<'_>) -> bool {
    keyword_arg(call, "n_obj").is_none_or(|count| count.text() != "1")
        && !args(call)
            .first()
            .and_then(string_literal)
            .is_some_and(|name| {
                matches!(
                    name.as_str(),
                    "sphere"
                        | "rastrigin"
                        | "ackley"
                        | "rosenbrock"
                        | "griewank"
                        | "schwefel"
                        | "zakharov"
                )
            })
}

fn assigned_value<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    match origin(node)?.binding {
        Binding::Assigned(value) => value,
        Binding::Parameter(_) => None,
    }
}

fn local_value<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    if node.kind() != "identifier" {
        return Some(node.clone());
    }
    let value = assigned_value(node)?;
    let scope = owner(node)?;
    let value_scope = owner(&value)?;
    (scope.node_id() == value_scope.node_id()).then_some(value)
}

fn call_is(call: &Node<'_>, symbol: &str) -> bool {
    resolved_call(call).is_some_and(|name| {
        name.starts_with("cvxpy.")
            && name
                .rsplit_once('.')
                .is_some_and(|(_, name)| name == symbol)
    })
}

fn objective<'t>(problem: &Node<'t>) -> Option<Node<'t>> {
    local_value(&keyword_arg(problem, "objective").or_else(|| args(problem).first().cloned())?)
}

fn constraints<'t>(problem: &Node<'t>) -> Option<Node<'t>> {
    let constraint_list =
        keyword_arg(problem, "constraints").or_else(|| args(problem).get(1).cloned())?;
    let constraint_list = local_value(&constraint_list)?;
    matches!(constraint_list.kind().as_ref(), "list" | "tuple").then_some(constraint_list)
}

fn comparison<'t>(node: &Node<'t>) -> Option<(Node<'t>, String, Node<'t>)> {
    if node.kind() != "comparison_operator" {
        return None;
    }
    let children: Vec<_> = node
        .children()
        .filter(|child| child.kind() != "comment")
        .collect();
    let [left, operator, right] = children.as_slice() else {
        return None;
    };
    Some((left.clone(), operator.text().into_owned(), right.clone()))
}

fn unit_label(leaf: &Node<'_>) -> Option<String> {
    let leaf = local_value(leaf)?;
    if !call_is(&leaf, "Variable") && !call_is(&leaf, "Parameter") {
        return None;
    }
    let name = string_literal(&keyword_arg(&leaf, "name")?)?;
    let unit = name.strip_suffix(']')?.rsplit_once('[')?.1;
    matches!(
        unit,
        "USD" | "EUR" | "DKK" | "fraction" | "percent" | "day" | "year"
    )
    .then(|| unit.to_owned())
}

pub(super) fn unit_mismatch<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    let (left, _, right) = comparison(node)?;
    let list = node.parent()?;
    (unit_label(&left)? != unit_label(&right)?
        && crate::ledger::python_ast::local_nodes(&owner(node)?).any(|problem| {
            call_is(&problem, "Problem")
                && constraints(&problem).is_some_and(|owned| owned.node_id() == list.node_id())
        }))
    .then(|| node.clone())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Curvature {
    Affine,
    Convex,
    Concave,
    Invalid,
}
use Curvature::{Affine, Concave, Convex, Invalid};

fn curvature(node: &Node<'_>) -> Option<Curvature> {
    let value = local_value(node)?;
    if (call_is(&value, "Variable") || call_is(&value, "Parameter"))
        && args(&value).is_empty()
        && keyword_arg(&value, "shape").is_none()
    {
        return Some(Affine);
    }
    let argument = args(&value).first()?.clone();
    let argument_curvature = curvature(&argument)?;
    if call_is(&value, "square") && argument_curvature == Affine {
        return Some(Convex);
    }
    if call_is(&value, "sqrt") {
        if argument_curvature == Affine {
            return Some(Concave);
        }
        let argument = local_value(&argument)?;
        if call_is(&argument, "square")
            && let Some(leaf) = args(&argument).first().and_then(local_value)
            && call_is(&leaf, "Variable")
            && leaf.field("arguments").is_some_and(|arguments| {
                arguments.named_children().all(|arg| {
                    matches!(arg.kind().as_ref(), "keyword_argument" | "comment")
                        && arg.field("name").is_none_or(|key| key.text() == "name")
                })
            })
        {
            return Some(Invalid);
        }
    }
    None
}

fn cvxpy_problem<'t>(receiver: &Node<'t>) -> Option<Node<'t>> {
    let problem = local_value(receiver)?;
    call_is(&problem, "Problem").then_some(problem)
}

fn default_dcp_solve(solve: &Node<'_>) -> bool {
    called(solve).as_deref() == Some("solve")
        && solve.field("arguments").is_some_and(|arguments| {
            arguments.named_children().all(|arg| {
                if arg.kind() == "comment" {
                    return true;
                }
                let Some(key) = arg.field("name") else {
                    return false;
                };
                match key.text().as_ref() {
                    "gp" | "qcp" => arg
                        .field("value")
                        .is_some_and(|value| value.kind() == "false"),
                    "solver" | "verbose" | "warm_start" | "eps" | "eps_abs" | "eps_rel"
                    | "max_iters" => true,
                    _ => false,
                }
            })
        })
}

fn invalid_constraint(node: &Node<'_>) -> bool {
    let Some((left, operator, right)) = comparison(node) else {
        return false;
    };
    let (Some(left), Some(right)) = (curvature(&left), curvature(&right)) else {
        return false;
    };
    match operator.as_str() {
        "<=" => !matches!(left, Affine | Convex) || !matches!(right, Affine | Concave),
        ">=" => !matches!(left, Affine | Concave) || !matches!(right, Affine | Convex),
        "==" => left != Affine || right != Affine,
        _ => false,
    }
}

pub(super) fn non_dcp<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    let valid_anchor = if node.kind() == "comparison_operator" {
        invalid_constraint(node)
    } else {
        node.parent()
            .and_then(|parent| parent.parent())
            .is_some_and(|objective| match curvature(node) {
                Some(Invalid) => call_is(&objective, "Minimize") || call_is(&objective, "Maximize"),
                Some(Concave) => call_is(&objective, "Minimize"),
                Some(Convex) => call_is(&objective, "Maximize"),
                _ => false,
            })
    };
    if !valid_anchor {
        return None;
    }
    let scope = owner(node)?;
    crate::ledger::python_ast::local_nodes(&scope)
        .filter(default_dcp_solve)
        .filter_map(|solve| cvxpy_problem(&receiver(&solve)?))
        .any(|problem| {
            args(&problem)
                .first()
                .and_then(local_value)
                .is_some_and(|objective| {
                    args(&objective)
                        .first()
                        .is_some_and(|value| value.node_id() == node.node_id())
                })
                || constraints(&problem).is_some_and(|constraint_list| {
                    constraint_list
                        .named_children()
                        .any(|child| child.node_id() == node.node_id())
                })
        })
        .then(|| node.clone())
}

fn following_statements<'t>(statement: &Node<'t>) -> impl Iterator<Item = Node<'t>> {
    std::iter::successors(statement.next(), Node::next).take_while(|node| {
        matches!(
            node.kind().as_ref(),
            "comment" | "expression_statement" | "return_statement" | "assert_statement"
        )
    })
}

fn attribute_of(node: &Node<'_>, attr: &str) -> Option<String> {
    (node.kind() == "attribute" && node.field("attribute")?.text() == attr)
        .then(|| node.field("object").as_ref().and_then(identifier))
        .flatten()
}

pub(super) fn unchecked_status<'t>(solve: &Node<'t>) -> Option<Node<'t>> {
    let statement = solve.parent()?;
    if statement.kind() != "expression_statement" || called(solve)?.as_str() != "solve" {
        return None;
    }
    let binding = receiver(solve)?;
    let name = identifier(&binding)?;
    let problem = cvxpy_problem(&binding)?;
    let mut owned = std::collections::BTreeSet::new();
    for expression in [objective(&problem), constraints(&problem)]
        .into_iter()
        .flatten()
    {
        owned.extend(expression.dfs().filter_map(|leaf| {
            identifier(&leaf)?;
            let value = local_value(&leaf)?;
            call_is(&value, "Variable").then(|| value.range().start)
        }));
    }
    for next in following_statements(&statement) {
        for node in next.dfs() {
            if attribute_of(&node, "status").as_deref() == Some(&name)
                || (node.kind() == "call"
                    && called(&node).as_deref() == Some("solve")
                    && receiver_name(&node).as_deref() == Some(&name))
            {
                return None;
            }
            if node.kind() == "attribute"
                && node
                    .field("attribute")
                    .is_some_and(|attr| attr.text() == "value")
                && node.field("object").is_some_and(|object| {
                    object.kind() == "identifier"
                        && local_value(&object).is_some_and(|value| {
                            value.node_id() == problem.node_id()
                                || owned.contains(&value.range().start)
                        })
                })
            {
                return Some(solve.clone());
            }
        }
    }
    None
}

fn receiver_name(call: &Node<'_>) -> Option<String> {
    identifier(&receiver(call)?)
}

pub(super) fn fit_on_holdout<'t>(fit: &Node<'t>) -> Option<Node<'t>> {
    if called(fit)?.as_str() != "fit" {
        return None;
    }
    let target = receiver(fit)?;
    let constructor = local_value(&target)?;
    if problem_kind(&constructor)? != "skfolio" {
        return None;
    }
    let data = identifier(args(fit).first()?)?;
    let parent = fit.parent()?;
    let (name, statement) = if let Some((left, right)) = assignments(&parent) {
        if right.node_id() != fit.node_id() {
            return None;
        }
        (identifier(&left)?, parent.parent()?)
    } else {
        (identifier(&target)?, parent)
    };
    if statement.kind() != "expression_statement" {
        return None;
    }
    let mut holdout = None;
    for next in following_statements(&statement) {
        for node in next.dfs() {
            if let Some((left, right)) = assignments(&node) {
                if identifier(&left).is_some_and(|binding| {
                    binding == name || binding == data || Some(&binding) == holdout.as_ref()
                }) {
                    return None;
                }
                if resolved_call(&right).as_deref()
                    == Some("sklearn.model_selection.train_test_split")
                    && args(&right)
                        .as_slice()
                        .first()
                        .and_then(identifier)
                        .as_deref()
                        == Some(&data)
                    && args(&right).len() == 1
                    && matches!(left.kind().as_ref(), "pattern_list" | "tuple_pattern")
                {
                    holdout = left.named_children().nth(1).as_ref().and_then(identifier);
                }
            }
            if node.kind() != "call" {
                continue;
            }
            let Some(method) = called(&node) else {
                continue;
            };
            if receiver(&node).as_ref().and_then(identifier).as_deref() != Some(&name) {
                continue;
            }
            if method == "fit" {
                return None;
            }
            if matches!(method.as_str(), "predict" | "score")
                && holdout.is_some()
                && args(&node).first().and_then(identifier) == holdout
            {
                return Some(fit.clone());
            }
        }
    }
    None
}
