//! What a variable of a Python file holds at one place, as far as the file itself proves it.
//!
//! ast-grep matches the shape of code and does not follow a name to its value. A rule about a
//! method, such as `fig.update_xaxes(...)`, must know that `fig` is a plotly figure. This module
//! follows the name back to the nearest earlier assignment, and from a parameter out to the
//! calls of its function.

use std::collections::HashSet;

use ast_grep_core::Matcher;

use crate::scan::Node;

/// What gives a name its value at the place of one use.
struct Origin<'t> {
    /// The annotation that the binding gives the name, as in `fig: go.Figure`.
    annotation: Option<Node<'t>>,
    binding: Binding<'t>,
}

/// The statement or the parameter that binds a name.
enum Binding<'t> {
    /// The nearest earlier statement that binds the name is `name = value`. The value is `None`
    /// for `name: T`, which declares the name and assigns nothing.
    Assigned(Option<Node<'t>>),
    /// The name is a parameter of the function around the use, and nothing binds it between the
    /// start of the function and the use.
    Parameter(Parameter<'t>),
}

/// One parameter of a `def`.
struct Parameter<'t> {
    function: Node<'t>,
    name: String,
    /// The place of the parameter among those a call fills by position, from 0. `None` for a
    /// parameter that only a keyword fills.
    position: Option<usize>,
}

/// Whether the variable at `usage` holds what `proof` matches, by every value the file shows
/// reaching it.
///
/// A value reaches a variable through its nearest earlier assignment, through another variable
/// assigned to it, or, for a parameter, through the argument of each call of its function. An
/// annotation that `proof` matches, as in `fig: go.Figure`, stands for the value. The variable
/// is proven when `proof` matches each of them, and there is at least one.
pub(crate) fn holds<M: Matcher>(usage: &Node<'_>, proof: &M) -> bool {
    let mut proven = false;
    // A function that passes its own parameter on to itself adds no value, so a variable that
    // was followed once is not followed again.
    let mut followed = HashSet::new();
    let mut open = vec![usage.clone()];
    while let Some(value) = open.pop() {
        if value.matches(proof) {
            proven = true;
            continue;
        }
        if value.kind() != "identifier" {
            return false;
        }
        if !followed.insert(value.node_id()) {
            continue;
        }
        let Some(origin) = origin(&value) else {
            return false;
        };
        if origin
            .annotation
            .is_some_and(|annotation| annotation.matches(proof))
        {
            proven = true;
            continue;
        }
        let values = match origin.binding {
            Binding::Assigned(value) => value.map(|value| vec![value]),
            Binding::Parameter(parameter) => passed(&parameter),
        };
        let Some(values) = values else {
            return false;
        };
        open.extend(values);
    }
    proven
}

/// What gives the name at `usage` its value there. `None` when something other than a plain
/// assignment or a parameter binds the name on the way back, such as a loop or an import, and
/// when nothing in the function or the module binds it before the use.
///
/// The search walks back through the statements before the use, then through those before each
/// statement that holds it, and stops at the start of the function. A statement that holds the
/// use and binds the name anywhere inside ends the search, because a loop can run the binding
/// before the use.
fn origin<'t>(usage: &Node<'t>) -> Option<Origin<'t>> {
    let name = usage.text();
    let mut statement = statement_around(usage.clone())?;
    loop {
        if binds(&statement, &name) {
            return None;
        }
        if let Some(binder) = statement.prev_all().find(|earlier| binds(earlier, &name)) {
            return assigned(&binder, &name);
        }
        let owner = statement.parent()?.parent()?;
        if owner.kind() == "function_definition" {
            return parameter(&owner, &name);
        }
        statement = statement_around(owner)?;
    }
}

/// `node` itself or the node around it that is a statement of a block or of the module.
fn statement_around(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        let parent = node.parent()?;
        if matches!(&*parent.kind(), "block" | "module") {
            return Some(node);
        }
        node = parent;
    }
}

/// Whether `statement`, or anything inside it, binds `name`.
fn binds(statement: &Node<'_>, name: &str) -> bool {
    statement.text().contains(name)
        && statement
            .dfs()
            .any(|node| node.kind() == "identifier" && node.text() == name && is_bound(&node))
}

/// Whether the identifier stands where Python binds a name: a target of an assignment, of a
/// loop or of `as`, a name an import, a `def` or a `class` makes, a parameter of a lambda, a
/// capture of a `case`, or a name of `global`, `nonlocal` or `del`. A parameter of a `def` is
/// not counted, because it belongs to the function and not to the code around it.
fn is_bound(identifier: &Node<'_>) -> bool {
    // A target can stand inside a tuple or a list of targets, as in `fig, ax = ...`.
    let mut target = identifier.clone();
    let parent = loop {
        let Some(parent) = target.parent() else {
            return false;
        };
        let holds_targets = matches!(
            &*parent.kind(),
            "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern"
        );
        if !holds_targets {
            break parent;
        }
        target = parent;
    };
    let directly = match &*parent.kind() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
            field_is(&parent, "left", &target)
        }
        "named_expression" | "function_definition" | "class_definition" => {
            field_is(&parent, "name", &target)
        }
        "aliased_import" => field_is(&parent, "alias", &target),
        "global_statement" | "nonlocal_statement" | "delete_statement" | "lambda_parameters" => {
            true
        }
        "default_parameter" => {
            field_is(&parent, "name", &target)
                && parent
                    .parent()
                    .is_some_and(|p| p.kind() == "lambda_parameters")
        }
        "expression_list" => parent
            .parent()
            .is_some_and(|p| p.kind() == "delete_statement"),
        // `import fig` and `from module import fig`. In `import a.b`, only `a` is bound.
        "dotted_name" => {
            parent
                .child(0)
                .is_some_and(|first| first.node_id() == target.node_id())
                && parent.parent().is_some_and(|import| {
                    matches!(
                        &*import.kind(),
                        "import_statement" | "import_from_statement"
                    ) && !field_is(&import, "module_name", &parent)
                })
        }
        _ => false,
    };
    // In `with ... as (a, b)` and in a `case` pattern the name can stand at any depth.
    directly
        || identifier
            .ancestors()
            .take_while(|around| !matches!(&*around.kind(), "block" | "module"))
            .any(|around| matches!(&*around.kind(), "as_pattern_target" | "case_pattern"))
}

/// Whether `child` is what `parent` holds in `field`.
fn field_is(parent: &Node<'_>, field: &str, child: &Node<'_>) -> bool {
    parent
        .field_children(field)
        .any(|held| held.node_id() == child.node_id())
}

/// The origin that the statement `binder` gives `name`, when the statement is one plain
/// assignment to the name.
fn assigned<'t>(binder: &Node<'t>, name: &str) -> Option<Origin<'t>> {
    let assignment = binder
        .child(0)
        .filter(|_| binder.kind() == "expression_statement")?;
    let left = assignment
        .field("left")
        .filter(|_| assignment.kind() == "assignment")?;
    (left.kind() == "identifier" && left.text() == name).then(|| Origin {
        annotation: assignment.field("type"),
        binding: Binding::Assigned(assignment.field("right")),
    })
}

/// The origin of `name` as a parameter of `function`. `None` for `*args` and `**kwargs`, and
/// for a name the function takes from the code around it.
fn parameter<'t>(function: &Node<'t>, name: &str) -> Option<Origin<'t>> {
    let mut position = Some(0);
    for declared in function.field("parameters")?.children() {
        let (identifier, annotation) = match &*declared.kind() {
            "identifier" => (Some(declared.clone()), None),
            "default_parameter" | "typed_default_parameter" => {
                (declared.field("name"), declared.field("type"))
            }
            "typed_parameter" => (declared.child(0), declared.field("type")),
            // After `*` or `*args`, a call fills a parameter by keyword only.
            "list_splat_pattern" | "keyword_separator" => (None, None),
            _ => continue,
        };
        match identifier.filter(|identifier| identifier.kind() == "identifier") {
            Some(identifier) if identifier.text() == name => {
                return Some(Origin {
                    annotation,
                    binding: Binding::Parameter(Parameter {
                        function: function.clone(),
                        name: name.to_owned(),
                        position,
                    }),
                });
            }
            Some(_) => position = position.map(|position| position + 1),
            None => position = None,
        }
    }
    None
}

/// The argument that each call of the parameter's function passes for it.
///
/// `None` when the file does not show every call: the function is a method or has a decorator,
/// its name is used for something other than a call, or a call fills the parameter in a way
/// that cannot be read, such as `**options`. A function that is never called gives no arguments.
fn passed<'t>(parameter: &Parameter<'t>) -> Option<Vec<Node<'t>>> {
    let function = &parameter.function;
    let around = function.parent()?;
    let is_method = around.kind() == "block"
        && around
            .parent()
            .is_some_and(|owner| owner.kind() == "class_definition");
    if is_method || around.kind() == "decorated_definition" {
        return None;
    }
    let defined = function.field("name")?;
    let name = defined.text();
    let mut arguments = Vec::new();
    for node in function.root().root().dfs() {
        if node.kind() != "identifier" || node.text() != name || node.node_id() == defined.node_id()
        {
            continue;
        }
        let parent = node.parent()?;
        match &*parent.kind() {
            "call" if field_is(&parent, "function", &node) => {
                arguments.push(argument(&parent, parameter)?);
            }
            // An attribute or a keyword of the same name is not the function.
            "attribute" if field_is(&parent, "attribute", &node) => {}
            "keyword_argument" if field_is(&parent, "name", &node) => {}
            _ => return None,
        }
    }
    Some(arguments)
}

/// What `call` passes for `parameter`. `None` when the call leaves it to the default, or holds
/// `*args` or `**kwargs`, which can fill any parameter.
fn argument<'t>(call: &Node<'t>, parameter: &Parameter<'t>) -> Option<Node<'t>> {
    let arguments = call
        .field("arguments")
        .filter(|list| list.kind() == "argument_list")?;
    let mut by_position = Vec::new();
    let mut by_keyword = None;
    for argument in arguments.named_children() {
        match &*argument.kind() {
            "comment" => {}
            "list_splat" | "dictionary_splat" => return None,
            "keyword_argument" => {
                if argument.field("name")?.text() == parameter.name {
                    by_keyword = argument.field("value");
                }
            }
            _ => by_position.push(argument),
        }
    }
    by_keyword.or_else(|| by_position.into_iter().nth(parameter.position?))
}
