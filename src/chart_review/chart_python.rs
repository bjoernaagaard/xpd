//! Static figure detection and Tufte checks on Python ASTs for Plotly (including `go.Table`),
//! Dash (`dcc.Graph`, `dash_ag_grid.AgGrid`), Matplotlib, Seaborn, and pandas.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::ledger::{self, FindingPlace};
use crate::source;
use crate::syntax::{self, Node, Root};
use crate::workspace::Workspace;

use super::{
    Figure, Finding, Kind, is_all_caps, is_rainbow_scale, looks_like_identifier, route_for,
};

/// Chart libraries whose calls xpd does not parse statically, still listed when imported.
const OTHER_LIBRARIES: [&str; 7] = [
    "altair",
    "bokeh",
    "holoviews",
    "hvplot",
    "plotnine",
    "pyecharts",
    "pygal",
];

/// What static Python inspection extracts from one file.
#[derive(Debug, Default)]
pub(crate) struct PythonSurvey {
    pub(crate) libraries: Vec<&'static str>,
    pub(crate) figures: Vec<Figure>,
    pub(crate) findings: Vec<Finding>,
}

/// Surveys one Python file for figures, plotly pack rule issues, and cross-library static findings.
pub(crate) fn survey_file(
    path: &Path,
    shown: &str,
    tracked: &str,
    text: &str,
    root: &Root,
    workspace: Option<&Workspace>,
) -> PythonSurvey {
    let names = imported(root);
    let modules: BTreeSet<&str> = names
        .values()
        .map(|module| module.split('.').next().unwrap_or_default())
        .collect();

    let mut figures = Vec::new();
    let mut findings = Vec::new();
    let mut symbol_ordinals: HashMap<(String, &'static str), usize> = HashMap::new();

    for call in root.root().dfs().filter(|node| node.kind() == "call") {
        let Some(function) = call.field("function") else {
            continue;
        };
        let written: String = function
            .text()
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect();
        let line = call.start_pos().line() + 1;
        let resolved = resolve(&written, &names);
        let symbol = enclosing_symbol(&call);

        for (kind, library) in classify(&written, resolved.as_deref(), &call, &modules) {
            let nth = symbol_ordinals
                .entry((symbol.clone(), kind))
                .and_modify(|count| *count += 1)
                .or_insert(1);
            let fig_key = format!("{tracked}\n{symbol}\n{kind}");
            let id = ledger::finding_id("figure", &fig_key, *nth);
            figures.push(Figure {
                id,
                file: shown.to_owned(),
                tracked: tracked.to_owned(),
                line,
                symbol: symbol.clone(),
                call: written.clone(),
                kind,
                library,
                route: route_for(kind),
            });
        }

        inspect_call(
            &written,
            resolved.as_deref(),
            &call,
            &modules,
            shown,
            tracked,
            line,
            &mut findings,
        );
    }

    if let Some(workspace) = workspace {
        let linted = workspace.analyze(source::File::on_disk(path), text);
        for found in &linted.found {
            let issue = found.issue();
            let rule_id = issue.id.to_string();
            let Some(rule_name) = rule_id.strip_prefix("plotly/") else {
                continue;
            };
            let Some(kind) = Kind::of_plotly_rule(rule_name) else {
                continue;
            };
            let line = issue.position.map_or(1, |pos| pos.line);
            findings.push(Finding {
                kind,
                key: format!("{tracked}\nplotly/{rule_name}\n{}", issue.message),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!("`plotly/{rule_name}`: {}", issue.message),
                weight: 3,
            });
        }
    }

    let mut libraries: BTreeSet<&'static str> = figures.iter().map(|fig| fig.library).collect();
    for library in ["plotly", "dash", "matplotlib", "seaborn"]
        .into_iter()
        .chain(OTHER_LIBRARIES)
    {
        if modules.contains(library) {
            libraries.insert(library);
        }
    }
    if modules.contains("dash_ag_grid") {
        libraries.insert("ag-grid");
    }

    PythonSurvey {
        libraries: libraries.into_iter().collect(),
        figures,
        findings,
    }
}

/// Name of the nearest enclosing Python function or class, or `<module>` at top level.
fn enclosing_symbol(node: &Node<'_>) -> String {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind().as_ref(),
            "function_definition" | "class_definition"
        ) && let Some(name) = parent.field("name")
        {
            return name.text().to_string();
        }
        current = parent.parent();
    }
    "<module>".to_owned()
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

/// The figure kinds and library of a Python call.
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
        } else if name == "dash_ag_grid.AgGrid" {
            found.push(("table", "ag-grid"));
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

/// Dispatches static Tufte checks on one Python call.
#[expect(
    clippy::too_many_arguments,
    reason = "internal per-call AST inspector sharing parsed file metadata"
)]
fn inspect_call(
    written: &str,
    resolved: Option<&str>,
    call: &Node<'_>,
    modules: &BTreeSet<&str>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    let method = written.rsplit('.').next().unwrap_or(written);
    if resolved.is_some_and(|name| {
        name == "plotly.graph_objects.Table" || name == "plotly.graph_objs.Table"
    }) || (modules.contains("plotly") && method == "add_table")
    {
        inspect_plotly_table(call, shown, tracked, line, findings);
        return;
    }
    if resolved == Some("dash_ag_grid.AgGrid") {
        inspect_dash_ag_grid(call, shown, tracked, line, findings);
        return;
    }
    if !(modules.contains("matplotlib")
        || modules.contains("seaborn")
        || modules.contains("pandas"))
    {
        return;
    }
    let is_mpl_or_pd = resolved.is_none_or(|name| {
        name.starts_with("matplotlib.")
            || name.starts_with("seaborn.")
            || name.starts_with("pandas.")
    });
    if is_mpl_or_pd {
        inspect_mpl_encoding(written, method, call, shown, tracked, line, findings);
        inspect_mpl_layout(written, method, call, shown, tracked, line, findings);
    }
}

/// Checks Matplotlib, Seaborn, and pandas calls for pie, dual-axis, 3D, rainbow, and hatch faults.
fn inspect_mpl_encoding(
    written: &str,
    method: &str,
    call: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if method == "pie"
        || (method == "plot"
            && syntax::keyword_arg(call, "kind")
                .and_then(|value| syntax::string_literal(&value))
                .as_deref()
                == Some("pie"))
    {
        findings.push(Finding {
            kind: Kind::PieChart,
            key: format!("{tracked}\nmpl-pie\n{written}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`{written}` draws a pie chart (VDQI p. 178). Replace with a sorted bar chart or a table."
            ),
            weight: 3,
        });
    }
    if matches!(method, "twinx" | "twiny") {
        findings.push(Finding {
            kind: Kind::DualAxis,
            key: format!("{tracked}\nmpl-dual-axis\n{written}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`{written}` adds a second value axis (VDQI p. 61). Use small multiples with shared scales instead."
            ),
            weight: 3,
        });
    }
    if syntax::keyword_arg(call, "projection")
        .and_then(|value| syntax::string_literal(&value))
        .is_some_and(|proj| proj.eq_ignore_ascii_case("3d"))
    {
        findings.push(Finding {
            kind: Kind::ExtraDimensions,
            key: format!("{tracked}\nmpl-3d\n{written}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`{written}` uses a 3D projection (VDQI p. 71). Keep depicted dimensions equal to data dimensions."
            ),
            weight: 3,
        });
    }
    if let Some(cmap) = syntax::keyword_arg(call, "cmap").and_then(|v| syntax::string_literal(&v))
        && is_rainbow_scale(&cmap)
    {
        findings.push(Finding {
            kind: Kind::RainbowColorscale,
            key: format!("{tracked}\nmpl-rainbow\n{cmap}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`cmap=\"{cmap}\"` uses an unordered rainbow palette (VDQI p. 154). Use an ordered lightness scale."
            ),
            weight: 2,
        });
    }
    if let Some(hatch) = syntax::keyword_arg(call, "hatch").and_then(|v| syntax::string_literal(&v))
        && !hatch.is_empty()
    {
        findings.push(Finding {
            kind: Kind::PatternFill,
            key: format!("{tracked}\nmpl-hatch\n{hatch}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`hatch=\"{hatch}\"` introduces moire vibration (VDQI p. 108). Use flat muted tints or direct labels."
            ),
            weight: 2,
        });
    }
}

/// Checks Matplotlib and Seaborn calls for rotated ticks, all-caps titles, vertical figsize, and thick lines.
fn inspect_mpl_layout(
    written: &str,
    method: &str,
    call: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if matches!(method, "xticks" | "tick_params" | "set_xticklabels") {
        let angle_node = syntax::keyword_arg(call, "rotation")
            .or_else(|| syntax::keyword_arg(call, "labelrotation"));
        if let Some(angle) = angle_node.and_then(|n| parse_number(&n))
            && angle.abs() > 0.0
            && (angle.abs() % 180.0) > 0.0
        {
            findings.push(Finding {
                kind: Kind::RotatedTicks,
                key: format!("{tracked}\nmpl-rotated-ticks\n{written}"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!(
                    "`{written}` rotates tick labels by {angle} degrees (VDQI p. 183). Keep words running left to right."
                ),
                weight: 2,
            });
        }
    }
    if matches!(method, "set_title" | "suptitle" | "title")
        && let Some(first_arg) = first_positional_arg(call)
        && let Some(title) = syntax::string_literal(&first_arg)
        && is_all_caps(&title)
    {
        findings.push(Finding {
            kind: Kind::AllCapsText,
            key: format!("{tracked}\nmpl-all-caps\n{title}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "Title `{title}` is written in all capitals (VDQI p. 183). Use sentence case."
            ),
            weight: 2,
        });
    }
    if matches!(method, "subplots" | "figure")
        && panel_count(call) == 1
        && let Some(figsize) = syntax::keyword_arg(call, "figsize")
        && let Some((width, height)) = parse_number_pair(&figsize)
        && height > width
    {
        findings.push(Finding {
            kind: Kind::VerticalAspectRatio,
            key: format!("{tracked}\nmpl-vertical-figsize\n{written}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "`figsize=({width}, {height})` is taller than wide for a single panel (VDQI p. 186). Default to wider than tall."
            ),
            weight: 2,
        });
    }
    if matches!(method, "plot" | "lineplot") {
        let lw = syntax::keyword_arg(call, "linewidth")
            .or_else(|| syntax::keyword_arg(call, "lw"))
            .and_then(|n| parse_number(&n));
        if let Some(width) = lw
            && width >= 4.0
        {
            findings.push(Finding {
                kind: Kind::ThickLine,
                key: format!("{tracked}\nmpl-thick-line\n{written}"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!(
                    "`linewidth={width}` draws a heavy line (VDQI p. 185). Keep data lines thin and delicate."
                ),
                weight: 1,
            });
        }
    }
}

/// Static checks on `go.Table(header=..., cells=...)`.
fn inspect_plotly_table(
    call: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(header) = syntax::keyword_arg(call, "header")
        && let Some(values) = py_mapping_value(&header, "values")
    {
        for raw in py_string_list(&values) {
            if is_all_caps(&raw) || looks_like_identifier(&raw) {
                findings.push(Finding {
                    kind: Kind::TableRawHeader,
                    key: format!("{tracked}\ngo-table-header\n{raw}"),
                    places: vec![FindingPlace::line(shown, line)],
                    evidence: format!(
                        "`go.Table` header `{raw}` looks like a raw identifier or all-caps label (VDQI p. 183). Spell out headers in sentence case with units."
                    ),
                    weight: 2,
                });
            }
        }
    }

    if let Some(cells) = syntax::keyword_arg(call, "cells") {
        if let Some(fill) = py_mapping_value(&cells, "fill_color").or_else(|| {
            py_mapping_value(&cells, "fill").and_then(|f| py_mapping_value(&f, "color"))
        }) && has_nested_alternating_colors(&fill)
        {
            findings.push(Finding {
                kind: Kind::TableZebraStriping,
                key: format!("{tracked}\ngo-table-zebra"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: "`go.Table` applies alternating row fill colors (VDQI p. 179). Remove zebra striping and separate topical row groups with thin horizontal rules.".to_owned(),
                weight: 2,
            });
        }

        let line_width = py_mapping_value(&cells, "line_width")
            .or_else(|| {
                py_mapping_value(&cells, "line").and_then(|l| py_mapping_value(&l, "width"))
            })
            .and_then(|n| parse_number(&n));
        if let Some(width) = line_width
            && width >= 2.0
        {
            findings.push(Finding {
                kind: Kind::TableVerticalRules,
                key: format!("{tracked}\ngo-table-borders"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!(
                    "`go.Table` sets heavy cell borders (`line.width={width}`, VDQI p. 179). Avoid vertical cages and heavy cell boxes."
                ),
                weight: 2,
            });
        }
    }
}

/// Static checks on `dash_ag_grid.AgGrid(columnDefs=..., dashGridOptions=...)`.
fn inspect_dash_ag_grid(
    call: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(col_defs) = syntax::keyword_arg(call, "columnDefs")
        && col_defs.kind() == "list"
    {
        inspect_dash_ag_grid_cols(&col_defs, shown, tracked, findings);
    }
    if let Some(options) = syntax::keyword_arg(call, "dashGridOptions") {
        let text = options.text();
        if text.contains("columnBorder") && text.contains("True") {
            findings.push(Finding {
                kind: Kind::TableVerticalRules,
                key: format!("{tracked}\nag-grid-col-border"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: "AG Grid enables vertical `columnBorder` lines (VDQI p. 179). Remove vertical gridlines and rely on alignment and horizontal group dividers.".to_owned(),
                weight: 2,
            });
        }
        if text.contains("oddRowBackgroundColor") {
            findings.push(Finding {
                kind: Kind::TableZebraStriping,
                key: format!("{tracked}\nag-grid-zebra"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: "AG Grid sets `oddRowBackgroundColor` zebra striping (VDQI p. 179). Use plain backgrounds with subtle horizontal topical dividers.".to_owned(),
                weight: 2,
            });
        }
    }
}

/// Checks each literal dict inside a Python AG Grid `columnDefs` list.
fn inspect_dash_ag_grid_cols(
    col_defs: &Node<'_>,
    shown: &str,
    tracked: &str,
    findings: &mut Vec<Finding>,
) {
    let mut text_sorted = Vec::new();
    let mut any_numeric_sorted = false;
    let mut has_numeric = false;

    for item in col_defs.named_children() {
        let field = py_mapping_value(&item, "field").and_then(|n| syntax::string_literal(&n));
        let header = py_mapping_value(&item, "headerName").and_then(|n| syntax::string_literal(&n));
        let col_type = py_mapping_value(&item, "type").and_then(|n| syntax::string_literal(&n));
        let data_type =
            py_mapping_value(&item, "cellDataType").and_then(|n| syntax::string_literal(&n));
        let has_formatter = py_mapping_value(&item, "valueFormatter").is_some();
        let sort = py_mapping_value(&item, "sort").and_then(|n| syntax::string_literal(&n));
        let col_line = item.start_pos().line() + 1;

        if let Some(ref hdr) = header {
            if is_all_caps(hdr) || looks_like_identifier(hdr) {
                findings.push(Finding {
                    kind: Kind::TableRawHeader,
                    key: format!("{tracked}\nag-grid-header\n{hdr}"),
                    places: vec![FindingPlace::line(shown, col_line)],
                    evidence: format!(
                        "AG Grid `headerName=\"{hdr}\"` is a raw identifier or all-caps label (VDQI p. 183). Spell out headers in sentence case with units."
                    ),
                    weight: 2,
                });
            }
        } else if let Some(ref fld) = field
            && looks_like_identifier(fld)
        {
            findings.push(Finding {
                kind: Kind::TableRawHeader,
                key: format!("{tracked}\nag-grid-field-no-header\n{fld}"),
                places: vec![FindingPlace::line(shown, col_line)],
                evidence: format!(
                    "AG Grid column `field=\"{fld}\"` has no `headerName` (VDQI p. 183). Spell out the column header with units."
                ),
                weight: 2,
            });
        }

        let is_right_aligned = col_type
            .as_deref()
            .is_some_and(|t| t.contains("numericColumn") || t.contains("rightAligned"))
            || py_mapping_value(&item, "cellStyle")
                .is_some_and(|style| style.text().contains("right"));
        let is_numeric = is_right_aligned || data_type.as_deref() == Some("number");

        if is_numeric {
            has_numeric = true;
            if sort.is_some() {
                any_numeric_sorted = true;
            }
            let label = header.as_deref().or(field.as_deref()).unwrap_or("column");
            if !has_formatter {
                findings.push(Finding {
                    kind: Kind::TableUnformattedNumbers,
                    key: format!("{tracked}\nag-grid-unformatted\n{label}"),
                    places: vec![FindingPlace::line(shown, col_line)],
                    evidence: format!(
                        "AG Grid numeric column `{label}` has no `valueFormatter` (VDQI p. 178). Format numbers to consistent decimal precision."
                    ),
                    weight: 2,
                });
            }
            if !is_right_aligned {
                findings.push(Finding {
                    kind: Kind::TableNumberAlignment,
                    key: format!("{tracked}\nag-grid-align\n{label}"),
                    places: vec![FindingPlace::line(shown, col_line)],
                    evidence: format!(
                        "AG Grid numeric column `{label}` is not right-aligned (VDQI p. 178). Set `type=\"numericColumn\"` or `type=\"rightAligned\"`."
                    ),
                    weight: 2,
                });
            }
        } else if sort.as_deref() == Some("asc")
            && let Some(label) = header.or(field)
        {
            text_sorted.push((label, col_line));
        }
    }

    if has_numeric && !any_numeric_sorted {
        for (label, col_line) in text_sorted {
            findings.push(Finding {
                kind: Kind::TableAlphabeticalOrder,
                key: format!("{tracked}\nag-grid-alpha-sort\n{label}"),
                places: vec![FindingPlace::line(shown, col_line)],
                evidence: format!(
                    "AG Grid sorts text column `{label}` ascending while numeric columns are unsorted (VDQI p. 178). Order rows by content or value rather than alphabetically."
                ),
                weight: 2,
            });
        }
    }
}

/// Reads key `name` from a Python dict literal `{"k": v}` or `dict(k=v)` call.
fn py_mapping_value<'t>(node: &Node<'t>, name: &str) -> Option<Node<'t>> {
    if node.kind() == "call" {
        return syntax::keyword_arg(node, name);
    }
    if node.kind() == "dictionary" {
        for pair in node.named_children().filter(|c| c.kind() == "pair") {
            if let Some(key) = pair.field("key").and_then(|k| syntax::string_literal(&k))
                && key == name
            {
                return pair.field("value");
            }
        }
    }
    None
}

/// Extracts all string literals directly inside a Python list or tuple node.
fn py_string_list(node: &Node<'_>) -> Vec<String> {
    if !matches!(node.kind().as_ref(), "list" | "tuple") {
        return Vec::new();
    }
    node.named_children()
        .filter_map(|child| syntax::string_literal(&child))
        .collect()
}

/// Checks whether a Python list literal looks like alternating zebra fill colors (`[["white", "lightgray"]]`).
fn has_nested_alternating_colors(node: &Node<'_>) -> bool {
    if node.kind() != "list" {
        return false;
    }
    let children: Vec<Node<'_>> = node.named_children().collect();
    if children.len() == 1 && children[0].kind() == "list" {
        let inner = py_string_list(&children[0]);
        return inner.len() >= 2 && inner.iter().collect::<BTreeSet<_>>().len() >= 2;
    }
    let direct = py_string_list(node);
    direct.len() >= 2 && direct.iter().collect::<BTreeSet<_>>().len() >= 2
}

/// First positional argument of a Python call node.
fn first_positional_arg<'t>(call: &Node<'t>) -> Option<Node<'t>> {
    call.field("arguments")?
        .named_children()
        .find(|arg| !matches!(arg.kind().as_ref(), "keyword_argument" | "comment"))
}

/// Parses a Python integer or float literal (including unary minus).
fn parse_number(node: &Node<'_>) -> Option<f64> {
    node.text().trim().parse::<f64>().ok()
}

/// Parses a 2-element numeric tuple or list such as `(4, 9)`.
fn parse_number_pair(node: &Node<'_>) -> Option<(f64, f64)> {
    if !matches!(node.kind().as_ref(), "tuple" | "list") {
        return None;
    }
    let nums: Vec<f64> = node
        .named_children()
        .filter_map(|child| parse_number(&child))
        .collect();
    (nums.len() == 2).then_some((nums[0], nums[1]))
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

/// The kind of a graph objects trace, by its lowercase class name.
fn trace_kind(trace: &str, call: &Node<'_>) -> Option<&'static str> {
    Some(match trace {
        "table" => "table",
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
