use std::collections::HashMap;
use std::path::Path;

use crate::ledger::python_ast::imports_of;
use crate::source;
use crate::syntax::{Node, Root};

use super::Kind;

pub(super) type Local = (Kind, String, usize);

/// The fixed wait or the retry that `node` is, with its evidence and the strength of it.
fn wait(node: &Node<'_>) -> Option<(Kind, String, usize)> {
    let evidence = match node.kind().as_ref() {
        "call" if sleeps(node) => "This waits a fixed time for something else to finish.",
        "decorator" => {
            let written = node.text().to_lowercase();
            (written.contains("retry") || written.contains("backoff"))
                .then_some("This runs the function again when it fails.")?
        }
        _ => return None,
    };
    Some((Kind::SleepOrRetry, evidence.to_owned(), 1))
}

/// The special case that `node` is, with its evidence and the strength of it.
fn special_case(node: &Node<'_>) -> Option<(Kind, String, usize)> {
    match node.kind().as_ref() {
        "except_clause" if swallows(node) => Some((
            Kind::SwallowedException,
            "This catches an exception and does nothing with it.".to_owned(),
            1,
        )),
        "with_item" => {
            let suppresses = node.field("value").is_some_and(|value| {
                value.kind() == "call"
                    && value.field("function").is_some_and(|function| {
                        matches!(function.text().as_ref(), "suppress" | "contextlib.suppress")
                    })
            });
            suppresses.then(|| {
                (
                    Kind::SwallowedException,
                    "This suppresses an exception and does nothing with it.".to_owned(),
                    1,
                )
            })
        }
        _ => None,
    }
}
/// Whether `call` is `time.sleep(...)` or `asyncio.sleep(...)`, under either name.
fn sleeps(call: &Node<'_>) -> bool {
    call.field("function").is_some_and(|function| {
        matches!(
            function.text().as_ref(),
            "time.sleep" | "sleep" | "asyncio.sleep"
        )
    })
}

/// Whether the `except` clause only passes, continues or holds `...`.
fn swallows(clause: &Node<'_>) -> bool {
    let Some(block) = clause.children().find(|child| child.kind() == "block") else {
        return false;
    };
    let mut statements = block
        .children()
        .filter(|child| child.is_named() && child.kind() != "comment")
        .peekable();
    statements.peek().is_some()
        && statements.all(|statement| match statement.kind().as_ref() {
            "pass_statement" | "continue_statement" => true,
            "expression_statement" => statement.text() == "...",
            _ => false,
        })
}

pub(super) fn rule_pattern(rule: &str) -> Option<&'static str> {
    match rule {
        "python/test-sleep" => Some("fixed wait"),
        "python/test-rerun" => Some("rerun"),
        "python/test-suppressed-health-check" => Some("suppressed health check"),
        "python/mock-import" => Some("mock import"),
        _ => None,
    }
}

pub(super) fn test_scope(root: &Root, line: usize) -> usize {
    root.root()
        .dfs()
        .filter(|node| {
            matches!(
                node.kind().as_ref(),
                "function_definition" | "decorated_definition"
            )
        })
        .map(|node| (node.start_pos().line() + 1, node.end_pos().line() + 1))
        .filter(|(start, end)| (*start..=*end).contains(&line))
        .max_by_key(|(start, _)| *start)
        .map_or(line, |(start, _)| start)
}

pub(super) fn findings(root: &Root, path: &Path) -> Vec<(usize, Local, &'static str)> {
    let in_test = source::is_test(path);
    let imports: HashMap<_, _> = if in_test {
        root.root()
            .dfs()
            .flat_map(|node| imports_of(&node))
            .collect()
    } else {
        HashMap::new()
    };
    let qualified = |text: &str| {
        let (root, rest) = text.split_once('.').unwrap_or((text, ""));
        imports.get(root).map_or_else(
            || text.to_owned(),
            |module| {
                if rest.is_empty() {
                    module.clone()
                } else {
                    format!("{module}.{rest}")
                }
            },
        )
    };
    let seeded = in_test
        && root.root().dfs().any(|node| {
            node.kind() == "call"
                && node.field("function").is_some_and(|callee| {
                    let name = qualified(&callee.text());
                    matches!(
                        name.as_str(),
                        "random.seed" | "numpy.random.seed" | "hypothesis.seed"
                    )
                })
        });
    let mut findings = Vec::new();
    for node in root.root().dfs() {
        let found = if in_test {
            special_case(&node).map(|local| (local, "")).or_else(|| {
                let pattern = match node.kind().as_ref() {
                    "decorator" => {
                        let written = node.text();
                        let name = written.trim_start_matches('@').split('(').next()?.trim();
                        let name = name.rsplit('.').next()?;
                        matches!(name, "flaky" | "rerun" | "reruns" | "retry" | "backoff")
                            .then_some("rerun")
                    }
                    "call" if !seeded => {
                        let name = qualified(&node.field("function")?.text());
                        nondeterministic_call(&node, &name)
                    }
                    _ => None,
                }?;
                let evidence = match pattern {
                    "rerun" => "Rerun marker: this can run the test again after a failure.",
                    "clock read" => "Clock read: this test depends on the time of the run.",
                    _ => {
                        "Unseeded random draw: this test can receive different values on each run."
                    }
                };
                Some(((Kind::FlakyTest, evidence.to_owned(), 1), pattern))
            })
        } else {
            wait(&node)
                .or_else(|| special_case(&node))
                .map(|local| (local, ""))
        };
        if let Some((local, pattern)) = found {
            findings.push((node.start_pos().line() + 1, local, pattern));
        }
    }
    findings
}

fn nondeterministic_call(node: &Node<'_>, name: &str) -> Option<&'static str> {
    match name {
        "time.time"
        | "time.time_ns"
        | "time.monotonic"
        | "time.monotonic_ns"
        | "time.perf_counter"
        | "time.perf_counter_ns"
        | "datetime.now"
        | "datetime.today"
        | "datetime.utcnow"
        | "datetime.datetime.now"
        | "datetime.datetime.utcnow"
        | "datetime.datetime.today"
        | "datetime.date.today" => Some("clock read"),
        "random.Random" | "numpy.random.default_rng" | "numpy.random.RandomState" => {
            let seeded = node
                .field("arguments")?
                .named_children()
                .any(|arg| arg.field("value").unwrap_or(arg).kind() != "none");
            (!seeded).then_some("unseeded random draw")
        }
        _ => {
            let random = name
                .strip_prefix("random.")
                .or_else(|| name.strip_prefix("numpy.random."))?;
            matches!(
                random,
                "random"
                    | "rand"
                    | "randn"
                    | "randint"
                    | "randrange"
                    | "choice"
                    | "choices"
                    | "sample"
                    | "shuffle"
                    | "uniform"
                    | "normal"
                    | "integers"
                    | "getrandbits"
            )
            .then_some("unseeded random draw")
        }
    }
}
