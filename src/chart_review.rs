//! `xpd chart-review`: the questions of Edward Tufte's book "The Visual Display of Quantitative
//! Information" for each chart of a file, in plotly, Dash, matplotlib, seaborn or pandas.
//!
//! The questions and facts live in `rules/xpd/chart-review.yml`, which this module embeds. A
//! review finds each chart call of the file with a call table and gives it a kind, such as pie
//! or line. It prints the plotly rules that fire on the file, and every question whose kinds
//! hold `any` or a kind it found, with its page of the book and the plotly route. With
//! `--run <adapter.py>`, `py/chart_facts.py` draws the figures that the adapter hands over in
//! the project's Python, and each measured fact goes under the questions it answers.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::Filters;
use crate::check::FixMode;
use crate::fix_safety::Applicability;
use crate::output::Format;
use crate::source;
use crate::syntax::{self, Node, Root};
use crate::venv;
use crate::workspace::Workspace;

/// The questions and the facts, written from the ledger of the book.
const INVENTORY: &str = include_str!("../rules/xpd/chart-review.yml");
/// The script that measures the figures of an adapter in the project's Python.
const CHART_FACTS_PY: &str = include_str!("../py/chart_facts.py");
/// The title of the book that the questions cite by page.
const BOOK: &str =
    "Edward Tufte, \"The Visual Display of Quantitative Information\", second edition";
/// The kind of a question that applies to every chart.
const ANY: &str = "any";
/// Chart libraries whose calls xpd does not read, which still get the questions for any chart.
const OTHER_LIBRARIES: [&str; 7] = [
    "altair",
    "bokeh",
    "holoviews",
    "hvplot",
    "plotnine",
    "pyecharts",
    "pygal",
];

/// One measured fact of a drawn figure.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Fact {
    id: String,
    text: String,
}

/// One review question, with the page and line of the book it comes from.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Question {
    pub(crate) id: String,
    pub(crate) page: u32,
    line: u32,
    chapter: String,
    pub(crate) principle: String,
    pub(crate) text: String,
    look: String,
    kinds: Vec<String>,
    facts: Vec<String>,
    plotly: String,
    pub(crate) rules: Vec<String>,
    covers: Vec<u32>,
}

/// Every question and fact of the review.
#[derive(Debug, Deserialize)]
pub(crate) struct Inventory {
    facts: Vec<Fact>,
    pub(crate) questions: Vec<Question>,
}

/// The inventory that ships with xpd.
pub(crate) fn inventory() -> Result<Inventory> {
    serde_saphyr::from_str(INVENTORY).context("rules/xpd/chart-review.yml does not parse")
}

/// The arguments of `xpd chart-review`.
#[derive(Debug, clap::Args)]
pub(crate) struct ChartReviewArgs {
    /// The Python file that draws the charts. Without it, or for a file that is not Python,
    /// every question is printed.
    path: Option<PathBuf>,
    /// A Python file that defines `figures()`, which returns a dict of name to figure. It runs
    /// in the project's Python, and each figure's measured facts go under their questions.
    #[arg(long, value_name = "ADAPTER")]
    run: Option<PathBuf>,
    /// Print the whole inventory of questions and facts, with no file.
    #[arg(long, conflicts_with_all = ["path", "run"])]
    questions: bool,
    /// Output format. Default: text on a terminal, json otherwise.
    #[arg(long, value_enum)]
    format: Option<Format>,
}

impl ChartReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        let inventory = inventory()?;
        let format = self.format.unwrap_or(default);
        let review = if self.questions {
            json!({
                "facts": inventory.facts,
                "questions": inventory.questions.iter().map(|question| question_json(question, &[], &[], &[])).collect::<Vec<_>>(),
            })
        } else {
            review(&inventory, self.path.as_deref(), self.run.as_deref())?
        };
        match format {
            Format::Json => println!("{review}"),
            Format::Text => print!("{}", to_text(&review)),
        }
        Ok(ExitCode::SUCCESS)
    }
}

/// One chart call of a file.
#[derive(Debug, Clone, Serialize)]
struct Chart {
    line: usize,
    call: String,
    kind: &'static str,
    library: &'static str,
    plotly: &'static str,
}

/// The review of one file, or of no file.
fn review(inventory: &Inventory, path: Option<&Path>, adapter: Option<&Path>) -> Result<Value> {
    let python = path.filter(|path| source::is_python(path));
    let (libraries, charts, issues) = match python {
        Some(path) => {
            let text = fs::read_to_string(path).with_context(|| source::cannot_read(path))?;
            let (libraries, charts) = charts(&syntax::parse(&text));
            (libraries, charts, plotly_issues(path)?)
        }
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    let kinds: BTreeSet<&str> = charts.iter().map(|chart| chart.kind).collect();
    let facts = adapter.map(measure);
    let figures = facts
        .as_ref()
        .and_then(|facts| facts.get("figures"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let fired: Vec<String> = issues
        .iter()
        .filter_map(|issue| issue.get("rule").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    let questions: Vec<Value> = inventory
        .questions
        .iter()
        .filter(|question| {
            python.is_none()
                || question
                    .kinds
                    .iter()
                    .any(|kind| kind == ANY || kinds.contains(kind.as_str()))
        })
        .map(|question| {
            let lines: Vec<usize> = charts
                .iter()
                .filter(|chart| {
                    question
                        .kinds
                        .iter()
                        .any(|kind| kind == ANY || kind == chart.kind)
                })
                .map(|chart| chart.line)
                .collect();
            question_json(question, &lines, &fired, &figures)
        })
        .collect();
    Ok(json!({
        "file": path.map(|path| path.display().to_string()),
        "book": BOOK,
        "libraries": libraries,
        "charts": charts,
        "issues": issues,
        "questions": questions,
        "of": inventory.questions.len(),
        "facts": facts,
    }))
}

/// A question as the review prints it: its fields from the inventory, the chart lines it applies
/// to, the plotly rules of it that fired, and what the run measured for it.
fn question_json(
    question: &Question,
    lines: &[usize],
    fired: &[String],
    figures: &[Value],
) -> Value {
    let mut measured = Vec::new();
    for figure in figures {
        for fact in &question.facts {
            let Some(found) = figure.pointer(&format!("/facts/{fact}")) else {
                continue;
            };
            measured.push(json!({
                "figure": figure.get("name"),
                "fact": fact,
                "summary": found.get("summary"),
                "findings": found.get("findings"),
            }));
        }
    }
    let rules: Vec<String> = question
        .rules
        .iter()
        .map(|rule| format!("plotly/{rule}"))
        .collect();
    let reported: Vec<&String> = rules
        .iter()
        .filter(|rule| {
            fired
                .iter()
                .any(|fired| rule.ends_with(&format!("/{fired}")))
        })
        .collect();
    json!({
        "id": question.id,
        "page": question.page,
        "line": question.line,
        "chapter": question.chapter,
        "principle": question.principle,
        "text": question.text,
        "look": question.look,
        "kinds": question.kinds,
        "plotly": question.plotly,
        "rules": rules,
        "reported": reported,
        "facts": question.facts,
        "covers": question.covers,
        "charts": lines,
        "measured": measured,
    })
}

/// The issues of the plotly rules on `path`, as `xpd check --rule plotly` reports them.
fn plotly_issues(path: &Path) -> Result<Vec<Value>> {
    let workspace = Workspace::open(
        std::env::current_dir()?,
        None,
        &Filters {
            rules: vec!["plotly".to_owned()],
            severity: None,
        },
    )?;
    let report = workspace.check(&[path.to_path_buf()], FixMode::Apply(None))?;
    let json = report.to_json(Applicability::Safe);
    Ok(json
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// Runs the adapter in the project's Python and returns what `py/chart_facts.py` printed, or
/// the reason it could not run. What the adapter and the libraries print on stderr is kept only
/// for a failed run, whose error ends with its last line. A failed run leaves the questions in
/// place.
fn measure(adapter: &Path) -> Value {
    let shown = adapter.display().to_string();
    let output = Command::new(venv::project_python())
        .arg("-c")
        .arg(CHART_FACTS_PY)
        .arg(adapter)
        .stdin(Stdio::null())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return json!({"adapter": shown, "figures": [], "error": format!("could not start python: {error}")});
        }
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last = stderr.lines().rev().find(|line| !line.trim().is_empty());
    let mut found: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        let code = output
            .status
            .code()
            .map_or_else(|| "a signal".to_owned(), |code| code.to_string());
        json!({"figures": [], "error": format!("{shown} exited with {code} and printed no facts")})
    });
    if let (Some(error), Some(last)) = (found.get("error").and_then(Value::as_str), last) {
        found["error"] = json!(format!("{error}. Its last line on stderr: {}", last.trim()));
    }
    found["adapter"] = json!(shown);
    found
}

/// The chart libraries that `root` imports and its chart calls in the order of their lines.
fn charts(root: &Root) -> (Vec<&'static str>, Vec<Chart>) {
    let names = imported(root);
    let modules: BTreeSet<&str> = names
        .values()
        .map(|module| module.split('.').next().unwrap_or_default())
        .collect();
    let mut charts = Vec::new();
    for call in root.root().dfs().filter(|node| node.kind() == "call") {
        let Some(function) = call.field("function") else {
            continue;
        };
        let written: String = function
            .text()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let line = call.start_pos().line() + 1;
        let resolved = resolve(&written, &names);
        for (kind, library) in classify(&written, resolved.as_deref(), &call, &modules) {
            charts.push(Chart {
                line,
                call: written.clone(),
                kind,
                library,
                plotly: route(kind),
            });
        }
    }
    let mut libraries: BTreeSet<&'static str> = charts.iter().map(|chart| chart.library).collect();
    for library in ["plotly", "dash", "matplotlib", "seaborn"]
        .into_iter()
        .chain(OTHER_LIBRARIES)
    {
        if modules.contains(library) {
            libraries.insert(library);
        }
    }
    (libraries.into_iter().collect(), charts)
}

/// The dotted module or name that each imported name of `root` stands for.
fn imported(root: &Root) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for import in root.root().dfs() {
        match &*import.kind() {
            "import_statement" => {
                for name in import.field_children("name") {
                    if name.kind() == "aliased_import" {
                        if let (Some(module), Some(alias)) =
                            (name.field("name"), name.field("alias"))
                        {
                            names.insert(alias.text().to_string(), module.text().to_string());
                        }
                    } else {
                        let text = name.text().to_string();
                        let head = text.split('.').next().unwrap_or_default().to_owned();
                        names.insert(head.clone(), head);
                    }
                }
            }
            "import_from_statement" => {
                let Some(module) = import
                    .field("module_name")
                    .filter(|module| module.kind() == "dotted_name")
                else {
                    continue;
                };
                let module = module.text().to_string();
                for name in import.field_children("name") {
                    let (target, alias) = if name.kind() == "aliased_import" {
                        let (Some(target), Some(alias)) = (name.field("name"), name.field("alias"))
                        else {
                            continue;
                        };
                        (target.text().to_string(), alias.text().to_string())
                    } else {
                        (name.text().to_string(), name.text().to_string())
                    };
                    names.insert(alias, format!("{module}.{target}"));
                }
            }
            _ => {}
        }
    }
    names
}

/// The dotted name that `written` calls, when its first name is imported.
fn resolve(written: &str, names: &HashMap<String, String>) -> Option<String> {
    let (head, rest) = written.split_once('.').unwrap_or((written, ""));
    let module = names.get(head)?;
    Some(if rest.is_empty() {
        module.clone()
    } else {
        format!("{module}.{rest}")
    })
}

/// The kinds and library of a call. A call can be a chart and also make panels, as
/// `px.line(..., facet_col="region")` does.
fn classify(
    written: &str,
    resolved: Option<&str>,
    call: &Node<'_>,
    modules: &BTreeSet<&str>,
) -> Vec<(&'static str, &'static str)> {
    let method = written.rsplit('.').next().unwrap_or(written);
    let mut found = Vec::new();
    if let Some(name) = resolved {
        let last = name.rsplit('.').next().unwrap_or(name);
        if let Some(function) = name.strip_prefix("plotly.express.") {
            if let Some(kind) = express_kind(function) {
                found.push((kind, "plotly"));
            }
            if ["facet_col", "facet_row"]
                .iter()
                .any(|key| syntax::keyword_arg(call, key).is_some())
            {
                found.push(("panels", "plotly"));
            }
        } else if name.starts_with("plotly.graph_objects.")
            || name.starts_with("plotly.graph_objs.")
        {
            if let Some(kind) = trace_kind(&last.to_lowercase(), call) {
                found.push((kind, "plotly"));
            }
        } else if name == "plotly.subplots.make_subplots" {
            found.push(("panels", "plotly"));
        } else if let Some(function) = name.strip_prefix("seaborn.") {
            if let Some(kind) = seaborn_kind(function) {
                found.push((kind, "seaborn"));
            }
        } else if name.starts_with("matplotlib.")
            && let Some(kind) = matplotlib_kind(last, call)
        {
            found.push((kind, "matplotlib"));
        }
        return found;
    }
    if !written.contains('.') {
        return found;
    }
    if modules.contains("plotly")
        && let Some(kind) = method
            .strip_prefix("add_")
            .and_then(|trace| trace_kind(trace, call))
    {
        found.push((kind, "plotly"));
        return found;
    }
    if modules.contains("pandas")
        && (written.contains(".plot.")
            || (method == "plot"
                && !modules.contains("matplotlib")
                && !modules.contains("seaborn")))
    {
        let kind = if method == "plot" {
            syntax::keyword_arg(call, "kind")
                .and_then(|kind| syntax::string_literal(&kind))
                .unwrap_or_else(|| "line".to_owned())
        } else {
            method.to_owned()
        };
        if let Some(kind) = pandas_kind(&kind) {
            found.push((kind, "pandas"));
        }
        return found;
    }
    if (modules.contains("matplotlib") || modules.contains("seaborn"))
        && let Some(kind) = matplotlib_kind(method, call)
    {
        found.push((kind, "matplotlib"));
    }
    found
}

fn express_kind(function: &str) -> Option<&'static str> {
    Some(match function {
        "pie" | "sunburst" | "funnel_area" => "pie",
        "bar" | "bar_polar" | "timeline" => "bar",
        "histogram" => "histogram",
        "line" | "line_polar" | "line_3d" | "ecdf" => "line",
        "area" => "area",
        "scatter" | "scatter_3d" | "scatter_polar" | "scatter_matrix" | "strip" => "scatter",
        "box" | "violin" => "box",
        "imshow" | "density_heatmap" | "density_contour" => "heatmap",
        "choropleth" | "choropleth_map" | "choropleth_mapbox" | "scatter_geo" | "scatter_map"
        | "scatter_mapbox" | "line_geo" | "line_map" | "line_mapbox" | "density_map"
        | "density_mapbox" => "map",
        _ => return None,
    })
}

/// The kind of a graph objects trace, by its lowercase class name. A scatter trace whose mode
/// draws only markers is a scatter, and any other is a line.
fn trace_kind(trace: &str, call: &Node<'_>) -> Option<&'static str> {
    Some(match trace {
        "pie" | "sunburst" | "funnelarea" => "pie",
        "bar" | "barpolar" => "bar",
        "histogram" => "histogram",
        "scatter" | "scattergl" => {
            let mode =
                syntax::keyword_arg(call, "mode").and_then(|mode| syntax::string_literal(&mode));
            match mode {
                Some(mode) if mode.contains("markers") && !mode.contains("lines") => "scatter",
                _ => "line",
            }
        }
        "box" | "violin" => "box",
        "heatmap" | "contour" | "image" | "histogram2d" => "heatmap",
        "choropleth" | "choroplethmap" | "choroplethmapbox" | "scattergeo" | "scattermap"
        | "scattermapbox" | "densitymap" | "densitymapbox" => "map",
        _ => return None,
    })
}

fn seaborn_kind(function: &str) -> Option<&'static str> {
    Some(match function {
        "barplot" | "countplot" => "bar",
        "histplot" | "displot" => "histogram",
        "lineplot" | "kdeplot" | "ecdfplot" => "line",
        "scatterplot" | "regplot" | "lmplot" | "stripplot" | "swarmplot" | "jointplot" => "scatter",
        "boxplot" | "violinplot" | "boxenplot" => "box",
        "heatmap" | "clustermap" => "heatmap",
        "catplot" | "relplot" | "pairplot" | "FacetGrid" => "panels",
        _ => return None,
    })
}

/// The kind of a matplotlib Axes or pyplot call. `subplots` makes panels when it asks for more
/// than one.
fn matplotlib_kind(function: &str, call: &Node<'_>) -> Option<&'static str> {
    Some(match function {
        "pie" => "pie",
        "bar" | "barh" => "bar",
        "hist" => "histogram",
        "plot" | "step" | "stairs" | "errorbar" => "line",
        "fill_between" | "stackplot" => "area",
        "scatter" | "hexbin" => "scatter",
        "boxplot" | "violinplot" => "box",
        "imshow" | "pcolormesh" | "contourf" => "heatmap",
        "subplots" if panel_count(call) > 1 => "panels",
        "subplot_mosaic" => "panels",
        _ => return None,
    })
}

/// The number of panels that `subplots(nrows, ncols)` asks for, as written.
fn panel_count(call: &Node<'_>) -> u64 {
    let number = |node: Option<Node<'_>>| {
        node.filter(|node| node.kind() == "integer")
            .and_then(|node| node.text().parse::<u64>().ok())
    };
    let positional: Vec<Node<'_>> = call
        .field("arguments")
        .map(|list| {
            list.named_children()
                .filter(|argument| !matches!(&*argument.kind(), "keyword_argument" | "comment"))
                .collect()
        })
        .unwrap_or_default();
    let rows =
        number(syntax::keyword_arg(call, "nrows")).or_else(|| number(positional.first().cloned()));
    let cols =
        number(syntax::keyword_arg(call, "ncols")).or_else(|| number(positional.get(1).cloned()));
    rows.unwrap_or(1) * cols.unwrap_or(1)
}

fn pandas_kind(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "line" | "kde" | "density" => "line",
        "bar" | "barh" => "bar",
        "hist" => "histogram",
        "box" => "box",
        "area" => "area",
        "pie" => "pie",
        "scatter" | "hexbin" => "scatter",
        _ => return None,
    })
}

/// The plotly call that draws a chart of `kind`.
fn route(kind: &str) -> &'static str {
    match kind {
        "pie" => "go.Table, or px.bar sorted by value",
        "bar" => "px.bar",
        "histogram" => "px.histogram",
        "line" => "px.line",
        "area" => "px.area",
        "scatter" => "px.scatter",
        "box" => "px.box, or a quartile plot with go.Scatter",
        "heatmap" => "px.imshow",
        "map" => "px.choropleth",
        _ => "facet_col, or make_subplots",
    }
}

/// The text of a JSON string, or nothing.
fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

/// The items of a JSON array, or none.
fn list(value: &Value) -> Vec<Value> {
    value.as_array().cloned().unwrap_or_default()
}

/// The review as text, from the same JSON that `--format json` prints. Writing to a `String`
/// cannot fail, so the results of `writeln!` are dropped.
fn to_text(review: &Value) -> String {
    let mut out = String::new();
    if let Some(file) = review.get("file").and_then(Value::as_str) {
        file_text(&mut out, file, review);
    }
    if let Some(facts) = review.get("facts").filter(|facts| !facts.is_null()) {
        facts_text(&mut out, facts);
    }
    let questions = list(&review["questions"]);
    let of = review
        .get("of")
        .map_or_else(|| questions.len().to_string(), ToString::to_string);
    let _ = writeln!(out, "Questions from {BOOK}, {} of {of}:", questions.len());
    for question in &questions {
        question_text(&mut out, question);
    }
    out
}

/// The libraries, the charts and the plotly issues of a reviewed file.
fn file_text(out: &mut String, file: &str, review: &Value) {
    let _ = writeln!(out, "Chart review of {file}");
    let libraries: Vec<String> = list(&review["libraries"]).iter().map(text).collect();
    let named = if libraries.is_empty() {
        "none that xpd reads".to_owned()
    } else {
        libraries.join(", ")
    };
    let _ = writeln!(out, "Libraries: {named}");
    let charts = list(&review["charts"]);
    if charts.is_empty() {
        out.push_str(
            "Charts: no chart call that xpd reads, so the questions for any chart follow.\n",
        );
    } else {
        out.push_str("Charts:\n");
        for chart in &charts {
            let _ = writeln!(
                out,
                "  line {}: {} draws a {} chart in {}. Plotly route: {}",
                chart["line"],
                text(&chart["call"]),
                text(&chart["kind"]),
                text(&chart["library"]),
                text(&chart["plotly"]),
            );
        }
    }
    let others: Vec<&str> = libraries
        .iter()
        .map(String::as_str)
        .filter(|library| !matches!(*library, "plotly" | "dash"))
        .collect();
    if !others.is_empty() {
        let _ = writeln!(
            out,
            "The plotly rules do not read {}. The questions below apply to every library, and each chart's plotly route is the call that draws it in plotly, the house library.",
            others.join(" or ")
        );
    }
    let issues = list(&review["issues"]);
    if issues.is_empty() {
        out.push_str("Plotly rules: none fires.\n");
        return;
    }
    out.push_str("Plotly rules that fire:\n");
    for issue in &issues {
        let _ = writeln!(
            out,
            "  {}:{}:{} {} [plotly/{}] {}",
            text(&issue["file"]),
            issue["line"],
            issue["column"],
            text(&issue["severity"]),
            text(&issue["rule"]),
            text(&issue["message"]),
        );
    }
}

/// What the run measured or why it failed, or the definitions that `--questions` prints.
fn facts_text(out: &mut String, facts: &Value) {
    if let Some(error) = facts.get("error").and_then(Value::as_str) {
        let _ = writeln!(out, "The run measured nothing: {error}");
    } else if let Some(figures) = facts.get("figures").and_then(Value::as_array) {
        let names: Vec<String> = figures.iter().map(|figure| text(&figure["name"])).collect();
        let count = if names.len() == 1 {
            "1 figure".to_owned()
        } else {
            format!("{} figures", names.len())
        };
        let _ = writeln!(
            out,
            "The run of {} measured {count}: {}",
            text(&facts["adapter"]),
            names.join(", ")
        );
        for figure in figures {
            if let Some(reason) = figure.get("unresolved").and_then(Value::as_str) {
                let _ = writeln!(
                    out,
                    "  {} is measured as written, because plotly could not draw it: {reason}",
                    text(&figure["name"])
                );
            }
        }
    } else if let Some(defined) = facts.as_array() {
        out.push_str("Facts that --run measures:\n");
        for fact in defined {
            let _ = writeln!(out, "  {}: {}", text(&fact["id"]), text(&fact["text"]));
        }
    }
}

/// One question with the charts it applies to, its rules and what the run measured for it.
fn question_text(out: &mut String, question: &Value) {
    let _ = writeln!(
        out,
        "\n{} (page {}): {}\n  {}\n  Look at: {}\n  In plotly: {}",
        text(&question["id"]),
        question["page"],
        text(&question["principle"]),
        text(&question["text"]),
        text(&question["look"]),
        text(&question["plotly"]),
    );
    let charts: Vec<String> = list(&question["charts"])
        .iter()
        .map(ToString::to_string)
        .collect();
    if !charts.is_empty() {
        let _ = writeln!(out, "  Charts at lines: {}", charts.join(", "));
    }
    let reported: Vec<String> = list(&question["reported"]).iter().map(text).collect();
    let rules: Vec<String> = list(&question["rules"])
        .iter()
        .map(text)
        .map(|rule| {
            if reported.contains(&rule) {
                format!("{rule} (fires)")
            } else {
                rule
            }
        })
        .collect();
    if !rules.is_empty() {
        let _ = writeln!(out, "  Rules: {}", rules.join(", "));
    }
    for measured in list(&question["measured"]) {
        let findings: Vec<String> = list(&measured["findings"]).iter().map(text).collect();
        let named = if findings.is_empty() {
            String::new()
        } else {
            format!(". {}", findings.join("; "))
        };
        let _ = writeln!(
            out,
            "  Measured in {} ({}): {}{named}",
            text(&measured["figure"]),
            text(&measured["fact"]),
            text(&measured["summary"]),
        );
    }
}
