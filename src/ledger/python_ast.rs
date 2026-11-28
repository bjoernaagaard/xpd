//! Shared Python syntax tree query helpers for ledger surveys.

use crate::names::{is_bound, is_parameter_name};
use crate::syntax::Node;

/// The name at the left end of `node`, as `frame` in `frame.loc[rows].sum`.
pub(crate) fn root_name(node: &Node<'_>) -> Option<String> {
    match node.kind().as_ref() {
        "identifier" => Some(node.text().into_owned()),
        "attribute" => root_name(&node.field("object")?),
        "subscript" => root_name(&node.field("value")?),
        "call" => root_name(&node.field("function")?),
        "parenthesized_expression" => root_name(&node.child(1)?),
        _ => None,
    }
}

/// The functions and classes at the top of `module`, each with whether a decorator wraps it.
pub(crate) fn top_level_definitions<'t>(module: &Node<'t>) -> Vec<(Node<'t>, bool)> {
    module
        .children()
        .filter_map(|item| {
            let decorated = item.kind() == "decorated_definition";
            let definition = if decorated {
                item.field("definition")?
            } else {
                item
            };
            matches!(
                definition.kind().as_ref(),
                "function_definition" | "class_definition"
            )
            .then_some((definition, decorated))
        })
        .collect()
}

/// The last name of what `call` calls, as `groupby` in `frame.groupby(keys)`.
pub(crate) fn called_method_name(call: &Node<'_>) -> Option<String> {
    let function = call.field("function")?;
    match function.kind().as_ref() {
        "identifier" => Some(function.text().into_owned()),
        "attribute" => Some(function.field("attribute")?.text().into_owned()),
        _ => None,
    }
}

/// The names that the import statement `import` binds, each with the module or the name of a
/// module that it stands for. A relative import is left out.
pub(crate) fn imports_of(import: &Node<'_>) -> Vec<(String, String)> {
    let dotted = |name: &Node<'_>| -> Option<(String, Option<String>)> {
        match name.kind().as_ref() {
            "dotted_name" => Some((name.text().into_owned(), None)),
            "aliased_import" => Some((
                name.field("name")?.text().into_owned(),
                Some(name.field("alias")?.text().into_owned()),
            )),
            _ => None,
        }
    };
    let names = import
        .field_children("name")
        .filter_map(|name| dotted(&name));
    match import.kind().as_ref() {
        "import_statement" => names
            .map(|(module, alias)| {
                let package = module.split('.').next().unwrap_or(&module).to_owned();
                alias.map_or_else(
                    || (package.clone(), package.clone()),
                    |alias| (alias, module.clone()),
                )
            })
            .collect(),
        "import_from_statement" => {
            let module = (import.field("module_name"))
                .filter(|module| module.kind() == "dotted_name")
                .map(|module| module.text().into_owned());
            let Some(module) = module else {
                return Vec::new();
            };
            names
                .map(|(name, alias)| {
                    let full = format!("{module}.{name}");
                    (alias.unwrap_or(name), full)
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Whether `node` introduces a Python lexical scope (module, function, class, or lambda).
pub(crate) fn is_python_scope(node: &Node<'_>) -> bool {
    matches!(
        node.kind().as_ref(),
        "module" | "function_definition" | "class_definition" | "lambda"
    )
}

/// Nearest enclosing Python lexical scope ancestor of `node`.
pub(crate) fn enclosing_scope<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    node.ancestors().find(is_python_scope)
}

/// Iterates over syntax nodes belonging directly to the lexical scope `root`.
pub(crate) fn local_nodes<'t>(root: &Node<'t>) -> impl Iterator<Item = Node<'t>> {
    let id = root.node_id();
    root.dfs()
        .filter(move |node| enclosing_scope(node).is_some_and(|scope| scope.node_id() == id))
}

/// Builds a dotted scope-qualified name for `name` using its enclosing function and class scopes.
pub(crate) fn scope_qualified_name(node: &Node<'_>, name: &str) -> String {
    let mut names: Vec<_> = node
        .ancestors()
        .filter(is_python_scope)
        .filter_map(|parent| parent.field("name").map(|name| name.text().into_owned()))
        .collect();
    names.reverse();
    names.push(name.to_owned());
    names.join(".")
}

/// Extracts `(left, right)` from an `assignment` node.
pub(crate) fn assignments<'t>(node: &Node<'t>) -> Option<(Node<'t>, Node<'t>)> {
    (node.kind() == "assignment").then_some(())?;
    Some((node.field("left")?, node.field("right")?))
}

/// Returns the identifier text when `node` is an `identifier`.
pub(crate) fn identifier(node: &Node<'_>) -> Option<String> {
    (node.kind() == "identifier").then(|| node.text().into_owned())
}

fn dotted(node: &Node<'_>) -> Option<String> {
    match node.kind().as_ref() {
        "identifier" => identifier(node),
        "attribute" => Some(format!(
            "{}.{}",
            dotted(&node.field("object")?)?,
            node.field("attribute")?.text()
        )),
        _ => None,
    }
}

fn enclosing_import(node: &Node<'_>, name: &str) -> Option<String> {
    for scope in node.ancestors().filter(is_python_scope) {
        let mut imports = local_nodes(&scope)
            .filter(|item| {
                matches!(
                    item.kind().as_ref(),
                    "import_statement" | "import_from_statement"
                )
            })
            .flat_map(|item| {
                imports_of(&item)
                    .into_iter()
                    .map(move |binding| (item.clone(), binding))
            })
            .filter(|(_, (bound, _))| bound == name);
        let imported = imports.next();
        if imports.next().is_some() {
            return None;
        }
        let rebound = local_nodes(&scope).any(|item| {
            let defines_name = matches!(
                item.kind().as_ref(),
                "function_definition" | "class_definition"
            ) && item
                .field("name")
                .is_some_and(|defined| defined.text() == name);
            defines_name
                || (item.kind() == "identifier"
                    && item.text() == name
                    && (is_bound(&item) || is_parameter_name(&item))
                    && imported.as_ref().is_none_or(|(statement, _)| {
                        !item
                            .ancestors()
                            .any(|ancestor| ancestor.node_id() == statement.node_id())
                    }))
        });
        if rebound {
            return None;
        }
        if let Some((statement, (_, full))) = imported {
            return (statement.range().start < node.range().start).then_some(full);
        }
    }
    None
}

/// Resolves the imported target of a `call` node's function expression.
pub(crate) fn resolved_call(node: &Node<'_>) -> Option<String> {
    resolved_name(&node.field("function")?)
}

/// Resolves an identifier or dotted attribute expression against enclosing imports.
pub(crate) fn resolved_name(node: &Node<'_>) -> Option<String> {
    let written = dotted(node)?;
    let (head, tail) = written.split_once('.').unwrap_or((&written, ""));
    let mut imported = enclosing_import(node, head)?;
    if !tail.is_empty() {
        imported.push('.');
        imported.push_str(tail);
    }
    Some(imported)
}

/// Returns the positional (non-keyword, non-comment) arguments of a Python `call`.
pub(crate) fn positional_call_args<'t>(call: &Node<'t>) -> Vec<Node<'t>> {
    call.field("arguments")
        .into_iter()
        .flat_map(|args| {
            args.named_children()
                .filter(|arg| !matches!(arg.kind().as_ref(), "keyword_argument" | "comment"))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Returns the receiver object of an attribute method `call`, as `model` in `model.fit(X)`.
pub(crate) fn call_receiver<'t>(call: &Node<'t>) -> Option<Node<'t>> {
    call.field("function")?.field("object")
}
