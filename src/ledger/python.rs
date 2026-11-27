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

/// The last name of what `call` calls, as `groupby` in `frame.groupby(keys)`.
pub(crate) fn called(call: &Node<'_>) -> Option<String> {
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
