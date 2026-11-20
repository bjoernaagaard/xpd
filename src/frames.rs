//! Which names of a Python file hold a pandas frame, as far as the file itself proves it.
//!
//! A rule about the shape of frame code, such as `df["x"] = ...`, matches code that a dict, a
//! list and an array share. `metadata.frame` keeps such a match only when this module
//! proves that the name holds a frame, and `metadata.frameScript` only for a module that builds
//! its frames in statements at its top.
//!
//! A name holds a frame when its nearest earlier assignment gives it one, or when it is a
//! parameter annotated `pd.DataFrame`. A value is a frame when it is one of the following:
//!
//! - a call of pandas that returns a frame, such as `pd.read_parquet(...)` or `pd.DataFrame(...)`
//! - a method of a frame that returns a frame, such as `df.dropna()`, without `inplace=`
//! - a selection of rows or of a list of columns from a frame, such as `df[df["a"] > 0]`
//! - a name that holds a frame
//!
//! Anything else is not proven, and so is a name that a loop, a branch or an import binds.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::names::{self, Binders, Binding, LibraryNames};
use crate::syntax::{Node, Root};

/// The name pandas is imported by. A constructor or an annotation must start at such an import.
const PANDAS: &str = "pandas";

/// Functions of pandas that read a file or a table into a frame.
const READERS: [&str; 17] = [
    "read_clipboard",
    "read_csv",
    "read_excel",
    "read_feather",
    "read_fwf",
    "read_gbq",
    "read_json",
    "read_orc",
    "read_parquet",
    "read_sas",
    "read_spss",
    "read_sql",
    "read_sql_query",
    "read_sql_table",
    "read_stata",
    "read_table",
    "read_xml",
];

/// Arguments that make a reader return something other than one frame: an iterator of chunks,
/// a dict of sheets or a Series.
const NOT_ONE_FRAME: [&str; 5] = ["chunksize", "iterator", "sheet_name", "squeeze", "typ"];

/// Other functions of pandas that always return a frame.
const MAKERS: [&str; 13] = [
    "DataFrame",
    "DataFrame.from_dict",
    "DataFrame.from_records",
    "crosstab",
    "get_dummies",
    "json_normalize",
    "melt",
    "merge",
    "merge_asof",
    "merge_ordered",
    "pivot",
    "pivot_table",
    "wide_to_long",
];

/// The function of pandas that joins frames and Series, which makes a frame from some parts.
const CONCAT: &str = "concat";

/// Methods of a frame that return a frame, unless the call passes `inplace=`.
const FRAME_METHODS: [&str; 55] = [
    "abs",
    "add_prefix",
    "add_suffix",
    "asfreq",
    "assign",
    "astype",
    "bfill",
    "clip",
    "convert_dtypes",
    "copy",
    "cummax",
    "cummin",
    "cumprod",
    "cumsum",
    "diff",
    "drop",
    "drop_duplicates",
    "droplevel",
    "dropna",
    "explode",
    "ffill",
    "fillna",
    "filter",
    "head",
    "infer_objects",
    "interpolate",
    "join",
    "mask",
    "melt",
    "merge",
    "nlargest",
    "nsmallest",
    "pct_change",
    "pivot",
    "pivot_table",
    "query",
    "rank",
    "reindex",
    "rename",
    "rename_axis",
    "reorder_levels",
    "replace",
    "reset_index",
    "round",
    "sample",
    "select_dtypes",
    "set_axis",
    "set_index",
    "shift",
    "sort_index",
    "sort_values",
    "swaplevel",
    "tail",
    "truncate",
    "where",
];

/// Methods of a column that return one True or False for each row.
const MASK_METHODS: [&str; 18] = [
    "between",
    "contains",
    "duplicated",
    "endswith",
    "eq",
    "fullmatch",
    "ge",
    "gt",
    "isin",
    "isna",
    "isnull",
    "le",
    "lt",
    "match",
    "ne",
    "notna",
    "notnull",
    "startswith",
];

/// Where a name that `metadata.frame` asks about must get its frame from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    /// From a parameter or from an assignment.
    Any,
    /// From a parameter of the function around the use, which nothing binds again before the
    /// use. The frame is then the one the caller passed.
    Parameter,
}

/// What the code of a value proves about it.
enum Value {
    Frame,
    /// A frame when one of these names holds a frame where the statement of the value starts.
    FrameIf(Vec<String>),
    Unknown,
}

/// The frame proofs of one file. A name asked about once at a statement is not followed again.
pub(crate) struct Frames<'t> {
    /// Whether a name holds a frame where a statement starts, by the statement's node and the
    /// name.
    known: RefCell<HashMap<(usize, String), bool>>,
    /// The names of the file that an import of pandas binds.
    pandas: LibraryNames<'t>,
    /// The statements of the file that bind a name.
    binders: Binders<'t>,
}

impl<'t> Frames<'t> {
    pub(crate) fn new() -> Self {
        Self {
            known: RefCell::new(HashMap::new()),
            pandas: LibraryNames::new(PANDAS),
            binders: Binders::default(),
        }
    }

    /// Whether the name at `usage` holds a frame, with the origin that `role` asks for, where
    /// the statement around it starts. The statement may assign the name again, as
    /// `df = df[df["a"] > 0]` does.
    pub(crate) fn holds(&self, usage: &Node<'t>, role: Role) -> bool {
        if usage.kind() != "identifier" {
            return false;
        }
        let Some(statement) = names::statement_around(usage.clone()) else {
            return false;
        };
        let name = usage.text();
        match role {
            Role::Any => self.is_frame(&statement, &name),
            Role::Parameter => {
                (self.binders.origin_before(&statement, &name)).is_some_and(|origin| {
                    matches!(origin.binding, Binding::Parameter(_))
                        && origin
                            .annotation
                            .is_some_and(|hint| self.is_frame_type(&hint))
                })
            }
        }
    }

    /// The first statement at the top of the module `root` that assigns or writes a frame, when
    /// the module has at least `least` such statements and no function that takes a frame. Such
    /// a module is a script that builds its frames line by line.
    pub(crate) fn script_start(&self, root: &'t Root, least: usize) -> Option<usize> {
        let module = root.root();
        let mut steps = module
            .children()
            .filter(|statement| self.is_frame_statement(statement));
        let first = steps.next()?;
        (steps.count() + 1 >= least && !self.has_frame_function(&module)).then(|| first.node_id())
    }

    /// Whether `name` holds a frame where `statement` starts. A value reaches the name through
    /// its nearest earlier assignment, and from there through the names that the assigned value
    /// is built from.
    fn is_frame(&self, statement: &Node<'t>, name: &str) -> bool {
        let asked = (statement.node_id(), name.to_owned());
        if let Some(known) = self.known.borrow().get(&asked) {
            return *known;
        }
        let mut proven = false;
        let mut followed = HashSet::new();
        let mut open = vec![(statement.clone(), name.to_owned())];
        while let Some((statement, name)) = open.pop() {
            let key = (statement.node_id(), name);
            let known = self.known.borrow().get(&key).copied();
            if known == Some(true) {
                proven = true;
                break;
            }
            if known == Some(false) || !followed.insert(key.clone()) {
                continue;
            }
            let Some(origin) = self.binders.origin_before(&statement, &key.1) else {
                continue;
            };
            if origin
                .annotation
                .is_some_and(|hint| self.is_frame_type(&hint))
            {
                proven = true;
                break;
            }
            let Binding::Assigned(Some(value)) = origin.binding else {
                continue;
            };
            let Some(binder) = names::statement_around(value.clone()) else {
                continue;
            };
            match self.value_of(&value, &binder) {
                Value::Frame => {
                    proven = true;
                    break;
                }
                Value::FrameIf(names) => {
                    open.extend(names.into_iter().map(|name| (binder.clone(), name)));
                }
                Value::Unknown => {}
            }
        }
        self.known.borrow_mut().insert(asked, proven);
        proven
    }

    /// Whether `value`, which `statement` holds, is a frame.
    fn evaluates_to_frame(&self, value: &Node<'t>, statement: &Node<'t>) -> bool {
        match self.value_of(value, statement) {
            Value::Frame => true,
            Value::FrameIf(names) => names.iter().any(|name| self.is_frame(statement, name)),
            Value::Unknown => false,
        }
    }

    /// Whether `statement` gives a name a frame, as `df = df.dropna()` does, or writes into a
    /// frame, as `df["x"] = 1` and `df.loc[rows, "x"] = 1` do.
    fn is_frame_statement(&self, statement: &Node<'t>) -> bool {
        let Some(assignment) = statement
            .child(0)
            .filter(|_| statement.kind() == "expression_statement")
        else {
            return false;
        };
        let plain = assignment.kind() == "assignment";
        if !plain && assignment.kind() != "augmented_assignment" {
            return false;
        }
        let (Some(left), Some(right)) = (assignment.field("left"), assignment.field("right"))
        else {
            return false;
        };
        match &*left.kind() {
            "identifier" if plain => {
                (assignment.field("type")).is_some_and(|hint| self.is_frame_type(&hint))
                    || self.evaluates_to_frame(&right, statement)
            }
            "subscript" => written_frame(&left).is_some_and(|name| self.is_frame(statement, &name)),
            _ => false,
        }
    }

    /// Whether a function of `module` takes a frame: a parameter is annotated as a frame, a
    /// call passes the function a frame, or a `.pipe(...)` call passes a frame through it.
    fn has_frame_function(&self, module: &Node<'t>) -> bool {
        let functions: Vec<_> = module
            .dfs()
            .filter(|node| node.kind() == "function_definition")
            .collect();
        let annotated = |function: &Node<'t>| {
            function.field("parameters").is_some_and(|parameters| {
                parameters
                    .children()
                    .filter_map(|parameter| parameter.field("type"))
                    .any(|hint| self.is_frame_type(&hint))
            })
        };
        if functions.iter().any(annotated) {
            return true;
        }
        let defined: HashSet<String> = functions
            .iter()
            .filter_map(|function| function.field("name"))
            .map(|name| name.text().into_owned())
            .collect();
        if defined.is_empty() {
            return false;
        }
        module
            .dfs()
            .filter(|node| node.kind() == "call")
            .any(|call| {
                let Some(function) = call.field("function") else {
                    return false;
                };
                let mut passed = arguments(&call).into_iter();
                match &*function.kind() {
                    "identifier" if defined.contains(&*function.text()) => {
                        names::statement_around(call.clone()).is_some_and(|statement| {
                            passed.any(|value| self.evaluates_to_frame(&value, &statement))
                        })
                    }
                    "attribute" if attribute_is(&function, &["pipe"]) => passed.any(|value| {
                        value.kind() == "identifier" && defined.contains(&*value.text())
                    }),
                    _ => false,
                }
            })
    }

    /// What the code of `value`, which `statement` holds, proves about it.
    fn value_of(&self, value: &Node<'t>, statement: &Node<'t>) -> Value {
        match &*value.kind() {
            "parenthesized_expression" => {
                inner(value).map_or(Value::Unknown, |inner| self.value_of(&inner, statement))
            }
            "identifier" => Value::FrameIf(vec![value.text().into_owned()]),
            "call" => self.call_value(value, statement),
            "subscript" => self.selection_value(value, statement),
            _ => Value::Unknown,
        }
    }

    /// What a call proves: a call of pandas that makes a frame, or a method of a frame that
    /// returns one.
    fn call_value(&self, call: &Node<'t>, statement: &Node<'t>) -> Value {
        let Some(function) = call.field("function") else {
            return Value::Unknown;
        };
        if let Some(name) = self.pandas_function(&function) {
            return self.made_value(&name, call, statement);
        }
        let returns_frame = function.kind() == "attribute"
            && attribute_is(&function, &FRAME_METHODS)
            && !passes(call, &["inplace"]);
        match function.field("object").filter(|_| returns_frame) {
            Some(receiver) => self.value_of(&receiver, statement),
            None => Value::Unknown,
        }
    }

    /// The name of the function of pandas that `function` is, as in `read_csv` for
    /// `pd.read_csv` and `DataFrame.from_dict` for `pd.DataFrame.from_dict`. `None` when the
    /// name does not start at an import of pandas, and for a function of pandas that makes no
    /// frame.
    fn pandas_function(&self, function: &Node<'t>) -> Option<String> {
        let start = dotted_start(function)?;
        let text = function.text();
        let name = match text.split_once('.') {
            Some((_, name)) => name,
            None => &*text,
        };
        let makes_frame = READERS.contains(&name) || MAKERS.contains(&name) || name == CONCAT;
        (makes_frame && self.pandas.binds(&start)).then(|| name.to_owned())
    }

    /// What a call of the pandas function `name` proves.
    fn made_value(&self, name: &str, call: &Node<'t>, statement: &Node<'t>) -> Value {
        if READERS.contains(&name) {
            return if passes(call, &NOT_ONE_FRAME) {
                Value::Unknown
            } else {
                Value::Frame
            };
        }
        if MAKERS.contains(&name) {
            return Value::Frame;
        }
        // Beside each other, the parts of a concat always make a frame. Below each other, they
        // make a frame when one of them is a frame.
        let axis = keyword(call, "axis").map(|axis| axis.text().into_owned());
        if matches!(axis.as_deref(), Some("1" | "\"columns\"" | "'columns'")) {
            return Value::Frame;
        }
        let parts = keyword(call, "objs").or_else(|| {
            (call.field("arguments")?.named_children())
                .find(|argument| !matches!(&*argument.kind(), "comment" | "keyword_argument"))
        });
        let Some(parts) = parts.filter(|parts| parts.kind() == "list") else {
            return Value::Unknown;
        };
        let mut names = Vec::new();
        for part in parts.named_children() {
            match self.value_of(&part, statement) {
                Value::Frame => return Value::Frame,
                Value::FrameIf(found) => names.extend(found),
                Value::Unknown => {}
            }
        }
        if names.is_empty() {
            Value::Unknown
        } else {
            Value::FrameIf(names)
        }
    }

    /// What a selection proves: rows or a list of columns of a frame are a frame, and one
    /// column or one row is not.
    fn selection_value(&self, selection: &Node<'t>, statement: &Node<'t>) -> Value {
        let Some(target) = selection.field("value") else {
            return Value::Unknown;
        };
        let indexes: Vec<_> = selection.field_children("subscript").collect();
        let whole_rows =
            |index: &Node<'t>| index.kind() == "list" || self.selects_rows(index, statement);
        let by_position = target.kind() == "attribute" && attribute_is(&target, &["loc", "iloc"]);
        let frame = if by_position {
            let kept = match indexes.as_slice() {
                [rows] => whole_rows(rows),
                [rows, columns] => whole_rows(rows) && matches!(&*columns.kind(), "list" | "slice"),
                _ => false,
            };
            target.field("object").filter(|_| kept)
        } else {
            match indexes.as_slice() {
                [index] if whole_rows(index) => Some(target),
                _ => None,
            }
        };
        frame.map_or(Value::Unknown, |frame| self.value_of(&frame, statement))
    }

    /// Whether `index`, which `statement` holds, selects rows by a condition or a slice, and
    /// not one column or one row by its label.
    fn selects_rows(&self, index: &Node<'t>, statement: &Node<'t>) -> bool {
        let operator = || index.field("operator").map(|operator| operator.text());
        match &*index.kind() {
            "slice" | "lambda" | "comparison_operator" => true,
            "parenthesized_expression" => {
                inner(index).is_some_and(|inner| self.selects_rows(&inner, statement))
            }
            "binary_operator" => operator().is_some_and(|sign| matches!(&*sign, "&" | "|" | "^")),
            "unary_operator" => operator().is_some_and(|sign| sign == "~"),
            "call" => (index.field("function"))
                .is_some_and(|function| attribute_is(&function, &MASK_METHODS)),
            // A mask with a name, as in `late = df["days"] > 30` before `df[late]`.
            "identifier" => {
                (self.binders.origin_before(statement, &index.text())).is_some_and(|origin| {
                    let Binding::Assigned(Some(value)) = origin.binding else {
                        return false;
                    };
                    value.kind() != "identifier"
                        && names::statement_around(value.clone())
                            .is_some_and(|binder| self.selects_rows(&value, &binder))
                })
            }
            _ => false,
        }
    }

    /// Whether `hint` is the annotation of a frame, as in `df: pd.DataFrame`.
    fn is_frame_type(&self, hint: &Node<'t>) -> bool {
        let Some(named) = hint
            .named_children()
            .next()
            .filter(|_| hint.kind() == "type")
        else {
            return false;
        };
        let text = named.text();
        (text == "DataFrame" || text.ends_with(".DataFrame"))
            && dotted_start(&named).is_some_and(|start| self.pandas.binds(&start))
    }
}

/// The first name of `dotted`, as `pd` in `pd.DataFrame.from_dict`. `None` when `dotted` is not
/// only names joined by dots, so that `pd.read_csv(path).dropna` is no function of pandas.
fn dotted_start<'t>(dotted: &Node<'t>) -> Option<Node<'t>> {
    let mut start = dotted.clone();
    while start.kind() == "attribute" {
        start = start.field("object")?;
    }
    (start.kind() == "identifier").then_some(start)
}

/// The name of the frame that an assignment to `target` writes into: `df` in `df["x"]` and in
/// `df.loc[rows, "x"]`.
fn written_frame(target: &Node<'_>) -> Option<String> {
    let written = target.field("value")?;
    let name = match &*written.kind() {
        "identifier" => written,
        "attribute" if attribute_is(&written, &["loc", "iloc", "at", "iat"]) => written
            .field("object")
            .filter(|object| object.kind() == "identifier")?,
        _ => return None,
    };
    Some(name.text().into_owned())
}

/// The expression inside a pair of parentheses.
fn inner<'t>(parenthesized: &Node<'t>) -> Option<Node<'t>> {
    parenthesized
        .named_children()
        .find(|child| child.kind() != "comment")
}

/// Whether `node` is `something.<name>` for one of `names`.
fn attribute_is(node: &Node<'_>, names: &[&str]) -> bool {
    node.kind() == "attribute"
        && node
            .field("attribute")
            .is_some_and(|attribute| names.contains(&&*attribute.text()))
}

/// The values that `call` passes, by position and by keyword.
fn arguments<'t>(call: &Node<'t>) -> Vec<Node<'t>> {
    let Some(list) = call
        .field("arguments")
        .filter(|list| list.kind() == "argument_list")
    else {
        return Vec::new();
    };
    list.named_children()
        .filter_map(|argument| match &*argument.kind() {
            "comment" => None,
            "keyword_argument" => argument.field("value"),
            _ => Some(argument),
        })
        .collect()
}

/// The value that `call` passes for the keyword `name`.
fn keyword<'t>(call: &Node<'t>, name: &str) -> Option<Node<'t>> {
    call.field("arguments")?
        .named_children()
        .filter(|argument| argument.kind() == "keyword_argument")
        .find(|argument| argument.field("name").is_some_and(|key| key.text() == name))?
        .field("value")
}

/// Whether `call` passes one of the keywords `names`, or `**options`, which can hold any of
/// them.
fn passes(call: &Node<'_>, names: &[&str]) -> bool {
    names.iter().any(|name| keyword(call, name).is_some())
        || call.field("arguments").is_some_and(|list| {
            list.named_children()
                .any(|argument| argument.kind() == "dictionary_splat")
        })
}
