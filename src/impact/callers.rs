use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::names::{Binders, Binding, is_bound, statement_around};
use crate::outline::Module;
use crate::syntax::{Node, Root, string_literal};

use super::{Boundary, Paths, line, literal_columns};

#[derive(Debug, Serialize)]
pub(super) struct Caller {
    pub(super) name: String,
    pub(super) line: usize,
    pub(super) call: String,
    pub(super) input: String,
    pub(super) uses: BTreeMap<usize, BTreeSet<String>>,
    pub(super) verification: String,
}

pub(super) fn find(
    root: &Root,
    module: &Module<'_>,
    entry: &str,
    entry_line: usize,
    outputs: &Paths,
    unresolved: &mut BTreeSet<Boundary>,
) -> Vec<Caller> {
    let mut found = Vec::new();
    let binders = Binders::default();
    for node in root
        .root()
        .dfs()
        .filter(|n| n.kind() == "identifier" && n.text() == entry)
    {
        if is_bound(&node) || non_reference(&node) {
            continue;
        }
        let owner = node.ancestors().find(|n| {
            matches!(
                n.kind().as_ref(),
                "function_definition" | "class_definition" | "lambda" | "module"
            )
        });
        let at_module = owner.as_ref().is_some_and(|n| n.kind() == "module");
        let function = owner.as_ref().and_then(|owner| {
            module
                .functions
                .iter()
                .find(|f| f.definition.range() == owner.range())
        });
        if function.is_some_and(|f| {
            f.parameters.iter().any(|p| p.name == entry)
                || f.definition.field("body").is_some_and(|body| {
                    body.dfs()
                        .any(|n| n.kind() == "identifier" && n.text() == entry && is_bound(&n))
                })
        }) {
            continue;
        }
        let rebound = module.assignments.iter().any(|(name, _)| name == entry)
            || (at_module && line(&node) <= entry_line);
        if let Some((call, input)) = direct_call(&node)
            && (at_module || function.is_some())
            && !rebound
        {
            let uses = uses_of(&call, outputs, &binders, unresolved);
            if !uses.is_empty() {
                found.push(Caller {
                    name: function.map_or_else(|| "(module)".to_owned(), |f| f.name.clone()),
                    line: line(&call),
                    call: call.text().into_owned(),
                    input: input.text().into_owned(),
                    uses,
                    verification: String::new(),
                });
            }
        } else {
            unresolved.insert(Boundary::new(
                line(&node),
                "indirect-call",
                format!("Resolve how this reference to {entry} is bound or invoked."),
            ));
        }
    }
    for call in root
        .root()
        .dfs()
        .filter(|n| n.kind() == "call" && dynamic_target(n, entry))
    {
        unresolved.insert(Boundary::new(
            line(&call),
            "dynamic-call",
            format!("Resolve whether {} calls {entry}.", call.text()),
        ));
    }
    found
}

fn non_reference(node: &Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        ["attribute", "name"].iter().any(|field| {
            parent
                .field(field)
                .is_some_and(|n| n.range() == node.range())
        })
    })
}

fn direct_call<'r>(node: &Node<'r>) -> Option<(Node<'r>, Node<'r>)> {
    let parent = node.parent()?;
    if parent.kind() == "call" && parent.field("function")?.range() == node.range() {
        let args = parent.field("arguments")?;
        let input = args.named_children().find(|n| n.kind() != "comment")?;
        return Some((parent, input));
    }
    if parent.kind() != "argument_list"
        || parent
            .named_children()
            .find(|n| n.kind() != "comment")?
            .range()
            != node.range()
    {
        return None;
    }
    let call = parent.parent().filter(|n| n.kind() == "call")?;
    let target = call.field("function")?;
    if target.field("attribute")?.text() != "pipe" {
        return None;
    }
    let input = target.field("object")?;
    Some((call, input))
}

fn dynamic_target(call: &Node<'_>, entry: &str) -> bool {
    let Some(target) = call.field("function") else {
        return false;
    };
    if target.kind() == "subscript" {
        return target.field("value").is_some_and(|value| {
            value.kind() == "call"
                && value
                    .field("function")
                    .is_some_and(|f| matches!(f.text().as_ref(), "globals" | "locals" | "vars"))
        }) && target
            .field("subscript")
            .and_then(|key| string_literal(&key))
            .is_none_or(|name| name == entry);
    }
    target.kind() == "call"
        && target
            .field("function")
            .is_some_and(|f| f.text() == "getattr")
        && target
            .field("arguments")
            .and_then(|args| {
                args.named_children()
                    .filter(|n| n.kind() != "comment")
                    .nth(1)
            })
            .and_then(|key| string_literal(&key))
            .is_none_or(|name| name == entry)
}

type Uses = BTreeMap<usize, BTreeSet<String>>;

fn uses_of<'r>(
    call: &Node<'r>,
    outputs: &Paths,
    binders: &Binders<'r>,
    unresolved: &mut BTreeSet<Boundary>,
) -> Uses {
    let mut uses = BTreeMap::<_, BTreeSet<_>>::new();
    let mut pending = vec![call.clone()];
    let mut seen = BTreeSet::new();
    while let Some(value) = pending.pop() {
        if outputs.is_empty() || !seen.insert((value.range().start, value.range().end)) {
            continue;
        }
        let Some(parent) = value.parent() else {
            continue;
        };
        let read = match parent.kind().as_ref() {
            "parenthesized_expression" => {
                pending.push(parent);
                continue;
            }
            "assignment"
                if parent
                    .field("right")
                    .is_some_and(|r| r.range() == value.range()) =>
            {
                if let Some(name) = parent.field("left").filter(|n| n.kind() == "identifier") {
                    pending.extend(local_uses(&name, &value, binders));
                    continue;
                }
                None
            }
            "subscript"
                if parent
                    .field("value")
                    .is_some_and(|n| n.range() == value.range()) =>
            {
                parent.field("subscript").and_then(|n| literal_columns(&n))
            }
            "return_statement" => Some(outputs.keys().cloned().collect()),
            "expression_statement" => continue,
            _ => None,
        };
        if let Some(columns) = read {
            uses.entry(line(&parent))
                .or_default()
                .extend(columns.into_iter().filter(|c| outputs.contains_key(c)));
        } else {
            let code = if parent.kind() == "subscript" {
                "dynamic-lookup"
            } else {
                "unknown-consumer"
            };
            unresolved.insert(Boundary::new(
                line(&parent),
                code,
                format!("Resolve how {} uses the pipeline result.", parent.text()),
            ));
        }
    }
    uses.retain(|_, columns| !columns.is_empty());
    uses
}

fn local_uses<'r>(name: &Node<'r>, value: &Node<'r>, binders: &Binders<'r>) -> Vec<Node<'r>> {
    let Some(block) = statement_around(name.clone()).and_then(|n| n.parent()) else {
        return Vec::new();
    };
    block.dfs()
        .filter(|n| n.range().start > value.range().end && n.kind() == "identifier" && n.text() == name.text())
        .filter(|n| !is_bound(n) && !non_reference(n))
        .filter(|n| {
            statement_around(n.clone())
                .and_then(|s| binders.origin_before(&s, &name.text()))
                .is_some_and(|origin| matches!(origin.binding, Binding::Assigned(Some(rhs)) if rhs.range() == value.range()))
        })
        .collect()
}
