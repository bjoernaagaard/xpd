use crate::names::{import_binding, imported_at, is_bound, is_parameter_name, statement_around};
use crate::outline;
use crate::syntax::{Node, Root};

use super::evidence::Observations;

pub(super) fn observe(root: &Root, entry: &str) -> Option<(usize, Observations)> {
    let module = outline::python_module(root);
    let function = module
        .functions
        .iter()
        .rfind(|function| function.name == entry)?;
    let line = function.definition.start_pos().line() + 1;
    let mut evidence = Observations::default();
    let mut imports = Vec::new();
    for node in root.root().dfs() {
        if node.kind() == "identifier"
            && let Some(path) = import_binding(&node).filter(|path| is_skfolio(path))
        {
            if let Some(statement) = statement_around(node.clone()) {
                evidence.note(
                    statement.start_pos().line() + 1,
                    "source-import",
                    &statement.text(),
                );
            }
            imports.push((node, path));
        } else if node.kind() == "import_from_statement"
            && node
                .field("module_name")
                .is_some_and(|module| is_skfolio(&module.text()))
            && node
                .named_children()
                .any(|child| child.kind() == "wildcard_import")
        {
            let line = node.start_pos().line() + 1;
            evidence.note(line, "source-import", &node.text());
            evidence.block(
                line,
                "wildcard-import",
                "Resolve which skfolio names this import supplies.",
            );
        }
    }
    let uncertain_paths: Vec<_> = root
        .root()
        .dfs()
        .filter_map(|node| {
            let reason = provenance_loss(&node)?;
            Some((source_path(&node, &imports)?, reason))
        })
        .collect();
    for call in function
        .definition
        .dfs()
        .filter(|node| node.kind() == "call")
    {
        if let Some(target) = call.field("function") {
            observe_call(&call, &target, &imports, &uncertain_paths, &mut evidence);
        }
    }
    evidence.block(
        line,
        "dependency-closure",
        "Trace the caller, helpers and adapters outside the observed calls. Run the migrated entry point in an isolated environment where skfolio cannot be imported.",
    );
    Some((line, evidence))
}

fn is_skfolio(path: &str) -> bool {
    path == "skfolio" || path.starts_with("skfolio.")
}

fn provenance_loss(node: &Node<'_>) -> Option<&'static str> {
    match node.kind().as_ref() {
        "attribute" if is_bound(node) => return Some("is modified in this file"),
        "identifier" if is_bound(node) || is_parameter_name(node) => return None,
        "attribute" | "identifier" => {}
        _ => return None,
    }
    let parent = node.parent()?;
    let is_field = |field| {
        parent
            .field(field)
            .is_some_and(|child| child.node_id() == node.node_id())
    };
    match parent.kind().as_ref() {
        "attribute" | "dotted_name" | "aliased_import" => None,
        "call" if is_field("function") => None,
        "keyword_argument" if is_field("name") => None,
        _ => Some("escapes as a value in this file"),
    }
}

fn observe_call<'tree>(
    call: &Node<'tree>,
    target: &Node<'tree>,
    imports: &[(Node<'tree>, String)],
    uncertain_paths: &[(String, &str)],
    evidence: &mut Observations,
) {
    let line = call.start_pos().line() + 1;
    let Some(path) = source_path(target, imports) else {
        if target.field("attribute").is_some_and(|name| {
            matches!(
                name.text().as_ref(),
                "from_pandas" | "to_pandas" | "to_numpy"
            )
        }) {
            evidence.note(line, "adapter", &call.text());
            evidence.block(line, "adapter-boundary", "Verify which types cross this adapter and whether it still imports skfolio or pandas.");
        } else {
            evidence.block(
                line,
                "unresolved-call",
                &format!(
                    "Trace {} to determine whether it reaches skfolio.",
                    target.text()
                ),
            );
        }
        return;
    };
    if let Some((uncertain, reason)) = uncertain_paths.iter().find(|(uncertain, _)| {
        path.strip_prefix(uncertain.as_str())
            .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with('.'))
    }) {
        evidence.block(
            line,
            "unresolved-call",
            &format!("Check {path}: {uncertain} {reason}."),
        );
        return;
    }
    evidence.note(line, "source-call", &format!("{path} via {}", call.text()));
    if path == "skfolio.preprocessing.prices_to_returns" {
        evidence.suggest(line, "native-returns", "Use an explicit, unique, ascending Date column. Apply the caller's missing-price policy, then compute price / price.shift(1) - 1 with Polars expressions and drop the first row. Apply log1p only for log returns.");
        evidence.block(line, "returns-boundary", "Establish the actual fill, threshold and inception options, factor join keys and order when used, and whether the caller expects one table or a pair.");
    } else {
        evidence.block(
            line,
            "unsupported-source-call",
            &format!("Establish a replacement and downstream contract for {path}."),
        );
    }
}

fn source_path<'tree>(target: &Node<'tree>, imports: &[(Node<'tree>, String)]) -> Option<String> {
    if target.kind() == "identifier" {
        return imports.iter().find_map(|(bound, path)| {
            (bound.text() == target.text() && imported_at(target, bound)).then(|| path.clone())
        });
    }
    let object = target.field("object")?;
    let name = target.field("attribute")?;
    let path = source_path(&object, imports)?;
    Some(format!("{path}.{name}", name = name.text()))
}
