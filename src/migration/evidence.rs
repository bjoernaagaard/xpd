use std::collections::BTreeSet;

use serde::Serialize;

use crate::frames::{Frames, Role};
use crate::names::{Binders, Binding, LibraryNames, statement_around};
use crate::syntax::{Node, string_literal};

#[derive(Debug, Serialize)]
pub(super) struct Evidence {
    pub(super) line: usize,
    pub(super) code: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Default)]
pub(super) struct Observations {
    pub(super) observed: Vec<Evidence>,
    pub(super) proposed: Vec<Evidence>,
    pub(super) unresolved: Vec<Evidence>,
}

impl Observations {
    pub(super) fn note(&mut self, line: usize, code: &'static str, message: &str) {
        add(&mut self.observed, line, code, message);
    }

    pub(super) fn suggest(&mut self, line: usize, code: &'static str, message: &str) {
        add(&mut self.proposed, line, code, message);
    }

    pub(super) fn block(&mut self, line: usize, code: &'static str, message: &str) {
        add(&mut self.unresolved, line, code, message);
    }

    pub(super) fn function<'tree>(
        &mut self,
        definition: &Node<'tree>,
        stages: &[&str],
        pandas: &LibraryNames<'tree>,
        constants: &BTreeSet<&str>,
    ) {
        let Some(body) = definition.field("body") else {
            return;
        };
        let returned_table = body
            .children()
            .filter(|node| node.kind() == "return_statement")
            .filter_map(|statement| statement.children().find(Node::is_named))
            .any(|value| table_result(&value, stages));
        if !returned_table {
            self.block(
                definition.start_pos().line() + 1,
                "return-shape",
                "Show which returned value is the output table.",
            );
        }
        if definition.children().any(|node| node.kind() == "async") {
            self.block(
                definition.start_pos().line() + 1,
                "async-boundary",
                "Resolve the async caller and materialization boundary.",
            );
        }
        let mut calls = Vec::new();
        let binders = Binders::default();
        for node in body.dfs() {
            let line = node.start_pos().line() + 1;
            match node.kind().as_ref() {
                "call" => self.call(&node, line, stages, &mut calls, pandas),
                "identifier" if is_operand(&node, stages, pandas) => {
                    let name = node.text();
                    let bound = statement_around(node.clone())
                        .and_then(|statement| binders.origin_before(&statement, &name));
                    if !constants.contains(name.as_ref())
                        && bound.is_none_or(|origin| matches!(origin.binding, Binding::Assigned(None)))
                    {
                        self.block(line, "external-value", &format!("Establish the value and alignment behavior of {name:?}."));
                    }
                }
                "list" | "tuple" | "dictionary" | "set" if collection_operand(&node) => {
                    self.block(line, "positional-values", "Establish row count and positional or key alignment for this collection.");
                }
                "attribute" => {
                    if node.field("attribute").is_some_and(|attr| {
                        matches!(
                            attr.text().as_ref(),
                            "index" | "loc" | "iloc" | "at" | "iat"
                        )
                    }) {
                        self.block(line, "index", "Establish row labels, alignment and cardinality before using positional Polars expressions.");
                    }
                }
                "assignment" => {
                    if node.field("left").is_some_and(|left| {
                        matches!(left.kind().as_ref(), "subscript" | "attribute")
                    }) {
                        self.suggest(line, "column-update", "Compute named column expressions in with_columns. Check which writes callers can observe.");
                    }
                }
                "subscript" => self.subscript(&node, line),
                "comparison_operator" => self.block(line, "comparison-missing-values", "Resolve null and NaN comparisons before translating the predicate. In Polars NaN can sort above finite values."),
                "if_statement"
                | "for_statement"
                | "while_statement"
                | "try_statement"
                | "match_statement"
                | "with_statement"
                | "list_comprehension"
                | "dictionary_comprehension"
                | "set_comprehension"
                | "generator_expression"
                | "lambda"
                | "function_definition"
                | "await"
                | "yield"
                | "import_statement"
                | "import_from_statement"
                | "global_statement"
                | "nonlocal_statement"
                | "delete_statement"
                | "augmented_assignment"
                | "assert_statement"
                | "raise_statement" => self.block(
                    line,
                    "python-control-flow",
                    "Determine how this Python construct affects inputs, outputs or side effects.",
                ),
                _ => {}
            }
        }
    }

    fn call<'tree>(
        &mut self,
        call: &Node<'tree>,
        line: usize,
        stages: &[&str],
        calls: &mut Vec<String>,
        pandas: &LibraryNames<'tree>,
    ) {
        let Some(target) = call.field("function") else {
            return;
        };
        let args: Vec<_> = call
            .field("arguments")
            .into_iter()
            .flat_map(|args| args.children().filter(Node::is_named).collect::<Vec<_>>())
            .filter(|arg| arg.kind() != "comment")
            .collect();
        if target.kind() == "identifier" {
            let name = target.text();
            if stages.contains(&name.as_ref()) {
                self.stage(line, &name, calls);
            } else {
                self.unknown_call(line, &target);
            }
            return;
        }
        let Some(attribute) = target.field("attribute") else {
            self.unknown_call(line, &target);
            return;
        };
        let method = attribute.text();
        match method.as_ref() {
            "pipe" => {
                if let [step] = args.as_slice()
                    && step.kind() == "identifier"
                    && stages.contains(&step.text().as_ref())
                {
                    self.stage(line, &step.text(), calls);
                } else {
                    self.unknown_call(line, &target);
                }
            }
            "assign" if args.iter().all(|arg| arg.kind() == "keyword_argument") => {
                self.suggest(line, "expressions", "Use pl.col and native arithmetic in with_columns. Keep dependent expressions in later stages.");
                for arg in &args {
                    self.note(arg.start_pos().line() + 1, "assignment", &arg.text());
                }
            }
            "copy" if args.is_empty() => self.note(
                line,
                "copy",
                "The source explicitly copies before mutation.",
            ),
            "read_csv" | "read_parquet"
                if target.field("object").is_some_and(|object| {
                    object.kind() == "identifier" && pandas.binds(&object)
                }) =>
            {
                self.file_read(line, &method, &args);
            }
            "groupby" => {
                self.note(line, "groupby", &call.text());
                if args.len() != 1 || !group_key(&args[0]) {
                    self.block(line, "group-options", "Resolve the group key and pandas groupby options before choosing a Polars window or group_by.");
                }
            }
            "transform"
                if args.as_slice().first().and_then(string_literal).as_deref() == Some("sum")
                    && args.len() == 1
                    && grouped_transform(&target) =>
            {
                self.suggest(line, "group-window", "Use pl.col(value).sum().over(keys) for the row-preserving sum. Guard missing keys to match pandas dropna behavior and test missing values and zero denominators.");
            }
            "join" | "merge" | "set_index" | "reset_index" | "reindex" | "sort_index" => {
                self.block(line, "index-or-join", "Establish key cardinality, alignment, null matching and required order before choosing a Polars join or replacing an index operation.");
            }
            "apply" | "map" | "map_partitions" | "iterrows" | "itertuples" => {
                self.block(line, "python-rows", "Read the operation's full behavior and redesign it as native expressions, a join or a window. Resolve unsupported side effects explicitly.");
            }
            _ => self.unknown_call(line, &target),
        }
    }

    fn file_read(&mut self, line: usize, method: &str, args: &[Node<'_>]) {
        let scan = if method == "read_csv" {
            "pl.scan_csv"
        } else {
            "pl.scan_parquet"
        };
        self.suggest(
            line,
            "lazy-scan",
            &format!("Start with {scan} so Polars can push filters and projections into the scan."),
        );
        if args.len() != 1 || string_literal(&args[0]).is_none() {
            self.block(
                line,
                "read-options",
                "Establish the path, input types, missing-value parsing and pandas reader options.",
            );
        }
    }

    fn subscript(&mut self, node: &Node<'_>, line: usize) {
        if let Some(index) = node.field("subscript") {
            match index.kind().as_ref() {
                "comparison_operator" | "binary_operator" => self.suggest(line, "filter", "Use a native filter expression after checking mask alignment and missing-value behavior."),
                "list" if index.children().filter(Node::is_named).all(|item| string_literal(&item).is_some()) =>
                    self.suggest(line, "projection", "Use select with explicit columns in the declared output order."),
                _ => {}
            }
        }
    }

    fn stage(&mut self, line: usize, name: &str, calls: &mut Vec<String>) {
        if calls.iter().any(|previous| previous == name) {
            self.block(line, "repeated-step", "Establish each invocation and its input columns. The reported stage list shows the first invocation.");
        }
        calls.push(name.to_owned());
    }

    fn unknown_call(&mut self, line: usize, target: &Node<'_>) {
        self.block(
            line,
            "unresolved-call",
            &format!(
                "Inspect {} and its effects at this call before mapping it.",
                target.text()
            ),
        );
    }
}

fn is_operand<'tree>(value: &Node<'tree>, stages: &[&str], pandas: &LibraryNames<'tree>) -> bool {
    let Some(parent) = value.parent() else {
        return false;
    };
    let is_part = |name| {
        parent
            .field(name)
            .is_some_and(|part| part.range() == value.range())
    };
    match parent.kind().as_ref() {
        "attribute" => !is_part("attribute") && !pandas.binds(value),
        "assignment" | "named_expression" => !is_part("left") && !is_part("name"),
        "keyword_argument" => !is_part("name"),
        "call" => !is_part("function"),
        "argument_list" => !stages.contains(&value.text().as_ref()),
        _ => true,
    }
}

fn collection_operand(value: &Node<'_>) -> bool {
    let Some(parent) = value.parent() else {
        return true;
    };
    let selector = parent.kind() == "subscript"
        && parent
            .field("subscript")
            .is_some_and(|index| index.range() == value.range());
    let method = |call: Node<'_>| {
        call.field("function")
            .and_then(|function| function.field("attribute"))
            .is_some_and(|attribute| attribute.text() == "groupby")
    };
    let group_key = match parent.kind().as_ref() {
        "argument_list" => parent.parent().is_some_and(method),
        "keyword_argument" => parent
            .parent()
            .and_then(|args| args.parent())
            .is_some_and(method),
        _ => false,
    };
    !selector && !group_key
}

fn table_result(value: &Node<'_>, stages: &[&str]) -> bool {
    match value.kind().as_ref() {
        "identifier" => Frames::new().holds(value, Role::Any),
        "call" => value.field("function").is_some_and(|function| {
            if function.kind() == "identifier" {
                return stages.contains(&function.text().as_ref());
            }
            function.field("attribute").is_some_and(|method| {
                matches!(
                    method.text().as_ref(),
                    "assign" | "copy" | "pipe" | "read_csv" | "read_parquet" | "join" | "merge"
                )
            })
        }),
        "parenthesized_expression" => value
            .children()
            .find(Node::is_named)
            .is_some_and(|inner| table_result(&inner, stages)),
        "subscript" => value.field("subscript").is_some_and(|index| {
            matches!(
                index.kind().as_ref(),
                "list" | "comparison_operator" | "boolean_operator" | "slice"
            )
        }),
        _ => false,
    }
}

fn group_key(argument: &Node<'_>) -> bool {
    let key = match argument.kind().as_ref() {
        "keyword_argument"
            if argument
                .field("name")
                .is_some_and(|name| name.text() == "by") =>
        {
            argument.field("value")
        }
        "keyword_argument" => None,
        _ => Some(argument.clone()),
    };
    key.is_some_and(|key| {
        string_literal(&key).is_some()
            || (key.kind() == "list"
                && key
                    .children()
                    .filter(Node::is_named)
                    .all(|item| string_literal(&item).is_some()))
    })
}

fn grouped_transform(target: &Node<'_>) -> bool {
    target.field("object").is_some_and(|object| {
        object.kind() == "subscript"
            && object.field("value").is_some_and(|value| {
                value.kind() == "call"
                    && value
                        .field("function")
                        .and_then(|function| function.field("attribute"))
                        .is_some_and(|method| method.text() == "groupby")
            })
    })
}

fn add(evidence: &mut Vec<Evidence>, line: usize, code: &'static str, message: &str) {
    if !evidence
        .iter()
        .any(|row| row.line == line && row.code == code && row.message == message)
    {
        evidence.push(Evidence {
            line,
            code,
            message: message.to_owned(),
        });
    }
}
