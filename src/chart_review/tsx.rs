//! Static figure detection and Tufte checks on JavaScript, TypeScript, JSX, and TSX ASTs for
//! Plotly (`react-plotly.js`, `plotly.js`) and AG Grid (`ag-grid-react`, `ag-grid-community`).

use std::collections::{BTreeSet, HashMap};

use crate::ledger::{self, FindingPlace};
use crate::syntax::{Node, Root};

use super::{
    Figure, Finding, Kind, is_all_caps, is_rainbow_scale, looks_like_identifier, route_for,
};

/// What static JS/TS/JSX/TSX inspection extracts from one file.
#[derive(Debug, Default)]
pub(crate) struct TsxSurvey {
    pub(crate) libraries: Vec<&'static str>,
    pub(crate) figures: Vec<Figure>,
    pub(crate) findings: Vec<Finding>,
}

/// Surveys one JS/TS/JSX/TSX file for Plotly and AG Grid figures and static Tufte findings.
pub(crate) fn survey_file(shown: &str, tracked: &str, root: &Root) -> TsxSurvey {
    let imports = imported_sources(root);
    let has_plotly = imports
        .iter()
        .any(|src| src == "react-plotly.js" || src.starts_with("plotly.js") || src == "plotly");
    let has_ag_grid = imports
        .iter()
        .any(|src| src.starts_with("ag-grid-") || src == "@ag-grid-community/react");
    if !has_plotly && !has_ag_grid {
        return TsxSurvey::default();
    }

    let bindings = const_bindings(root);
    let mut state = TsxState {
        shown,
        tracked,
        has_plotly,
        has_ag_grid,
        bindings,
        figures: Vec::new(),
        findings: Vec::new(),
        symbol_ordinals: HashMap::new(),
    };

    for node in root.root().dfs() {
        match node.kind().as_ref() {
            "jsx_self_closing_element" | "jsx_opening_element" => state.visit_jsx(&node),
            "call_expression" => state.visit_call(&node),
            _ => {}
        }
    }

    let mut libraries = Vec::new();
    if has_ag_grid {
        libraries.push("ag-grid");
    }
    if has_plotly {
        libraries.push("plotly");
    }
    TsxSurvey {
        libraries,
        figures: state.figures,
        findings: state.findings,
    }
}

struct TsxState<'t> {
    shown: &'t str,
    tracked: &'t str,
    has_plotly: bool,
    has_ag_grid: bool,
    bindings: HashMap<String, Node<'t>>,
    figures: Vec<Figure>,
    findings: Vec<Finding>,
    symbol_ordinals: HashMap<(String, &'static str), usize>,
}

impl<'t> TsxState<'t> {
    fn visit_jsx(&mut self, node: &Node<'t>) {
        let Some(name_node) = node.field("name") else {
            return;
        };
        let tag = name_node.text().to_string();
        let line = node.start_pos().line() + 1;
        let symbol = enclosing_symbol(node);

        if self.has_plotly && (tag == "Plot" || tag.ends_with(".Plot")) {
            let data_expr =
                jsx_attr_expr(node, "data").map(|expr| resolve_binding(expr, &self.bindings));
            let layout_expr =
                jsx_attr_expr(node, "layout").map(|expr| resolve_binding(expr, &self.bindings));
            for kind in plotly_data_kinds(data_expr.as_ref()) {
                push_figure(
                    &mut self.figures,
                    &mut self.symbol_ordinals,
                    self.shown,
                    self.tracked,
                    line,
                    &symbol,
                    &format!("<{tag}>"),
                    kind,
                    "plotly",
                );
            }
            inspect_plotly_spec(
                data_expr.as_ref(),
                layout_expr.as_ref(),
                self.shown,
                self.tracked,
                line,
                &mut self.findings,
            );
        } else if self.has_ag_grid && (tag == "AgGridReact" || tag.ends_with(".AgGridReact")) {
            push_figure(
                &mut self.figures,
                &mut self.symbol_ordinals,
                self.shown,
                self.tracked,
                line,
                &symbol,
                &format!("<{tag}>"),
                "table",
                "ag-grid",
            );
            let cols =
                jsx_attr_expr(node, "columnDefs").map(|expr| resolve_binding(expr, &self.bindings));
            let grid_opts = jsx_attr_expr(node, "gridOptions")
                .map(|expr| resolve_binding(expr, &self.bindings));
            let theme_expr =
                jsx_attr_expr(node, "theme").map(|expr| resolve_binding(expr, &self.bindings));
            inspect_ag_grid_props(
                cols.as_ref(),
                grid_opts.as_ref(),
                theme_expr.as_ref(),
                self.shown,
                self.tracked,
                line,
                &mut self.findings,
            );
        }
    }

    fn visit_call(&mut self, node: &Node<'t>) {
        let Some(func) = node.field("function") else {
            return;
        };
        let written: String = func
            .text()
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect();
        let line = node.start_pos().line() + 1;
        let symbol = enclosing_symbol(node);

        if self.has_plotly
            && matches!(
                written.as_str(),
                "Plotly.newPlot" | "Plotly.react" | "newPlot" | "react"
            )
        {
            let args = call_args(node);
            let data_expr = args
                .get(1)
                .cloned()
                .map(|expr| resolve_binding(expr, &self.bindings));
            let layout_expr = args
                .get(2)
                .cloned()
                .map(|expr| resolve_binding(expr, &self.bindings));
            for kind in plotly_data_kinds(data_expr.as_ref()) {
                push_figure(
                    &mut self.figures,
                    &mut self.symbol_ordinals,
                    self.shown,
                    self.tracked,
                    line,
                    &symbol,
                    &written,
                    kind,
                    "plotly",
                );
            }
            inspect_plotly_spec(
                data_expr.as_ref(),
                layout_expr.as_ref(),
                self.shown,
                self.tracked,
                line,
                &mut self.findings,
            );
        } else if self.has_ag_grid && (written == "createGrid" || written.ends_with(".createGrid"))
        {
            push_figure(
                &mut self.figures,
                &mut self.symbol_ordinals,
                self.shown,
                self.tracked,
                line,
                &symbol,
                &written,
                "table",
                "ag-grid",
            );
            let args = call_args(node);
            let opts = args
                .get(1)
                .cloned()
                .map(|expr| resolve_binding(expr, &self.bindings));
            let cols = opts
                .as_ref()
                .and_then(|o| js_object_prop(o, "columnDefs"))
                .map(|expr| resolve_binding(expr, &self.bindings));
            let theme_expr = opts
                .as_ref()
                .and_then(|o| js_object_prop(o, "theme"))
                .map(|expr| resolve_binding(expr, &self.bindings));
            inspect_ag_grid_props(
                cols.as_ref(),
                opts.as_ref(),
                theme_expr.as_ref(),
                self.shown,
                self.tracked,
                line,
                &mut self.findings,
            );
        } else if self.has_ag_grid && written.ends_with(".withParams") {
            let args = call_args(node);
            if let Some(params) = args.first() {
                inspect_ag_grid_theme_params(
                    params,
                    self.shown,
                    self.tracked,
                    line,
                    &mut self.findings,
                );
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "builder helper avoiding duplicate figure construction across JSX and call sites"
)]
fn push_figure(
    figures: &mut Vec<Figure>,
    symbol_ordinals: &mut HashMap<(String, &'static str), usize>,
    shown: &str,
    tracked: &str,
    line: usize,
    symbol: &str,
    call: &str,
    kind: &'static str,
    library: &'static str,
) {
    let nth = symbol_ordinals
        .entry((symbol.to_owned(), kind))
        .and_modify(|count| *count += 1)
        .or_insert(1);
    let fig_key = format!("{tracked}\n{symbol}\n{kind}");
    let id = ledger::finding_id("figure", &fig_key, *nth);
    figures.push(Figure {
        id,
        file: shown.to_owned(),
        tracked: tracked.to_owned(),
        line,
        symbol: symbol.to_owned(),
        call: call.to_owned(),
        kind,
        library,
        route: route_for(kind),
    });
}

/// Module specifiers from ES `import ... from "..."` statements.
fn imported_sources(root: &Root) -> Vec<String> {
    let mut sources = Vec::new();
    for node in root.root().dfs() {
        if node.kind() == "import_statement"
            && let Some(source) = node.field("source")
            && let Some(text) = js_string(&source)
        {
            sources.push(text);
        }
    }
    sources
}

/// Map of variable name to its initializer AST node for resolving JSX/call references in-file.
fn const_bindings(root: &Root) -> HashMap<String, Node<'_>> {
    let mut map = HashMap::new();
    for node in root.root().dfs() {
        if node.kind() == "variable_declarator"
            && let (Some(name), Some(value)) = (node.field("name"), node.field("value"))
            && name.kind() == "identifier"
        {
            map.insert(name.text().to_string(), value);
        }
    }
    map
}

/// Unwraps TypeScript `as` / `satisfies` / parenthesized expressions and resolves simple
/// same-file identifier bindings one hop.
fn resolve_binding<'t>(node: Node<'t>, bindings: &HashMap<String, Node<'t>>) -> Node<'t> {
    let unwrapped = unwrap_expr(node);
    if unwrapped.kind() == "identifier"
        && let Some(target) = bindings.get(unwrapped.text().as_ref())
    {
        return unwrap_expr(target.clone());
    }
    unwrapped
}

fn unwrap_expr(mut node: Node<'_>) -> Node<'_> {
    loop {
        match node.kind().as_ref() {
            "parenthesized_expression"
            | "as_expression"
            | "satisfies_expression"
            | "type_assertion"
            | "jsx_expression" => {
                let next = node.named_children().next();
                if let Some(inner) = next {
                    node = inner;
                    continue;
                }
                return node;
            }
            _ => return node,
        }
    }
}

/// Nearest enclosing TS/JS component, function, method, or class name.
fn enclosing_symbol(node: &Node<'_>) -> String {
    let mut current = node.parent();
    while let Some(parent) = current {
        match parent.kind().as_ref() {
            "function_declaration" | "class_declaration" | "method_definition" => {
                if let Some(name) = parent.field("name") {
                    return name.text().to_string();
                }
            }
            "variable_declarator" => {
                if let Some(name) = parent.field("name")
                    && name.kind() == "identifier"
                {
                    return name.text().to_string();
                }
            }
            _ => {}
        }
        current = parent.parent();
    }
    "<module>".to_owned()
}

/// Extracts the expression inside a JSX attribute `name={expr}` or `name="str"`.
fn jsx_attr_expr<'t>(element: &Node<'t>, attr_name: &str) -> Option<Node<'t>> {
    for child in element.named_children() {
        if child.kind() != "jsx_attribute" {
            continue;
        }
        let mut parts = child.named_children();
        let Some(name) = parts.next() else {
            continue;
        };
        if name.text() != attr_name {
            continue;
        }
        if let Some(value) = parts.next() {
            return Some(unwrap_expr(value));
        }
    }
    None
}

/// Positional argument nodes of a JS/TS `call_expression`.
fn call_args<'t>(call: &Node<'t>) -> Vec<Node<'t>> {
    call.field("arguments")
        .map(|args| args.named_children().collect())
        .unwrap_or_default()
}

/// Extracts property `key` from a JS/TS `object` literal.
fn js_object_prop<'t>(obj: &Node<'t>, key: &str) -> Option<Node<'t>> {
    if obj.kind() != "object" {
        return None;
    }
    for prop in obj.named_children() {
        if prop.kind() != "pair" {
            continue;
        }
        let Some(k) = prop.field("key") else {
            continue;
        };
        let key_text = js_string(&k).unwrap_or_else(|| k.text().to_string());
        if key_text == key {
            return prop.field("value").map(unwrap_expr);
        }
    }
    None
}

/// Extracts a JS/TS string literal's inner text.
fn js_string(node: &Node<'_>) -> Option<String> {
    match node.kind().as_ref() {
        "string" => {
            let raw = node.text();
            let trimmed = raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| raw.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))?;
            Some(trimmed.to_owned())
        }
        "template_string" => {
            let raw = node.text();
            if raw.contains("${") {
                return None;
            }
            raw.strip_prefix('`')
                .and_then(|s| s.strip_suffix('`'))
                .map(str::to_owned)
        }
        _ => None,
    }
}

fn js_number(node: &Node<'_>) -> Option<f64> {
    node.text().trim().parse::<f64>().ok()
}

fn js_string_array(node: &Node<'_>) -> Vec<String> {
    if node.kind() != "array" {
        return Vec::new();
    }
    node.named_children()
        .filter_map(|child| js_string(&unwrap_expr(child)))
        .collect()
}

/// Classifies Plotly trace kinds from a JS/TS `data` array expression.
fn plotly_data_kinds(data: Option<&Node<'_>>) -> Vec<&'static str> {
    let mut kinds = BTreeSet::new();
    if let Some(array) = data.filter(|node| node.kind() == "array") {
        for trace in array.named_children() {
            let trace = unwrap_expr(trace);
            let kind = js_object_prop(&trace, "type")
                .and_then(|v| js_string(&v))
                .map(|t| t.to_ascii_lowercase());
            let mapped = match kind.as_deref() {
                Some("table") => "table",
                Some("pie" | "sunburst" | "funnelarea") => "pie",
                Some("bar" | "barpolar") => "bar",
                Some("histogram") => "histogram",
                Some("box" | "violin") => "box",
                Some("heatmap" | "contour" | "image" | "histogram2d") => "heatmap",
                Some(
                    "choropleth" | "choroplethmap" | "choroplethmapbox" | "scattergeo"
                    | "scattermap" | "scattermapbox" | "densitymap" | "densitymapbox",
                ) => "map",
                _ => {
                    let mode = js_object_prop(&trace, "mode").and_then(|v| js_string(&v));
                    if mode
                        .as_deref()
                        .is_some_and(|m| m.contains("markers") && !m.contains("lines"))
                    {
                        "scatter"
                    } else {
                        "line"
                    }
                }
            };
            kinds.insert(mapped);
        }
    }
    if kinds.is_empty() {
        vec!["line"]
    } else {
        kinds.into_iter().collect()
    }
}

/// Static Tufte checks on JS/TS Plotly `data` and `layout` expressions.
fn inspect_plotly_spec(
    data: Option<&Node<'_>>,
    layout: Option<&Node<'_>>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    let (trace_count, has_direct_text) = data.map_or((0, false), |node| {
        inspect_plotly_traces(node, shown, tracked, findings)
    });
    if let Some(layout_obj) = layout.filter(|node| node.kind() == "object") {
        inspect_plotly_layout(
            layout_obj,
            trace_count,
            has_direct_text,
            shown,
            tracked,
            line,
            findings,
        );
    }
}

/// Inspects a JS/TS Plotly `data` array expression, returning `(trace_count, has_direct_text)`.
fn inspect_plotly_traces(
    data: &Node<'_>,
    shown: &str,
    tracked: &str,
    findings: &mut Vec<Finding>,
) -> (usize, bool) {
    if data.kind() != "array" {
        return (0, false);
    }
    let mut trace_count = 0usize;
    let mut has_direct_text = false;
    for trace in data.named_children() {
        let trace = unwrap_expr(trace);
        if trace.kind() != "object" {
            continue;
        }
        trace_count += 1;
        let trace_line = trace.start_pos().line() + 1;
        let trace_type = js_object_prop(&trace, "type")
            .and_then(|v| js_string(&v))
            .map_or_else(|| "scatter".to_owned(), |s| s.to_ascii_lowercase());
        let mode = js_object_prop(&trace, "mode")
            .and_then(|v| js_string(&v))
            .unwrap_or_default();

        if mode.contains("text") || js_object_prop(&trace, "texttemplate").is_some() {
            has_direct_text = true;
        }
        inspect_plotly_trace_fields(
            &trace,
            &trace_type,
            &mode,
            shown,
            tracked,
            trace_line,
            findings,
        );
    }
    (trace_count, has_direct_text)
}

/// Checks one JS/TS Plotly trace object literal for Tufte violations.
fn inspect_plotly_trace_fields(
    trace: &Node<'_>,
    trace_type: &str,
    mode: &str,
    shown: &str,
    tracked: &str,
    trace_line: usize,
    findings: &mut Vec<Finding>,
) {
    if matches!(trace_type, "pie" | "sunburst" | "funnelarea") {
        findings.push(Finding {
            kind: Kind::PieChart,
            key: format!("{tracked}\nts-plotly-pie\n{trace_type}"),
            places: vec![FindingPlace::line(shown, trace_line)],
            evidence: format!(
                "Plotly trace `type: '{trace_type}'` compares angles instead of a common scale (VDQI p. 178). Use a sorted bar chart or a table."
            ),
            weight: 3,
        });
    }
    if trace_type == "indicator" && mode.contains("gauge") {
        findings.push(Finding {
            kind: Kind::DecorativeGauge,
            key: format!("{tracked}\nts-plotly-gauge"),
            places: vec![FindingPlace::line(shown, trace_line)],
            evidence: "Plotly gauge indicator uses heavy non-data chrome for a single value (VDQI p. 116). Show a number, text-table, or sparkline.".to_owned(),
            weight: 3,
        });
    }
    if matches!(trace_type, "scatter3d" | "surface" | "mesh3d" | "bar3d") {
        findings.push(Finding {
            kind: Kind::ExtraDimensions,
            key: format!("{tracked}\nts-plotly-3d\n{trace_type}"),
            places: vec![FindingPlace::line(shown, trace_line)],
            evidence: format!(
                "Plotly trace `type: '{trace_type}'` depicts extra 3D dimensions (VDQI p. 71). Use a 2D plot or small multiples."
            ),
            weight: 3,
        });
    }
    if js_object_prop(trace, "yaxis")
        .and_then(|v| js_string(&v))
        .is_some_and(|y| y == "y2")
    {
        findings.push(Finding {
            kind: Kind::DualAxis,
            key: format!("{tracked}\nts-plotly-dual-y"),
            places: vec![FindingPlace::line(shown, trace_line)],
            evidence: "Plotly trace assigns `yaxis: 'y2'` creating a dual y-axis (VDQI p. 61). Use small multiples with aligned x-axes.".to_owned(),
            weight: 3,
        });
    }
    if let Some(scale) = js_object_prop(trace, "colorscale").and_then(|v| js_string(&v))
        && is_rainbow_scale(&scale)
    {
        findings.push(Finding {
            kind: Kind::RainbowColorscale,
            key: format!("{tracked}\nts-plotly-rainbow\n{scale}"),
            places: vec![FindingPlace::line(shown, trace_line)],
            evidence: format!(
                "Plotly `colorscale: '{scale}'` has no natural lightness order (VDQI p. 154). Use an ordered sequential scale."
            ),
            weight: 2,
        });
    }
    inspect_plotly_trace_style(trace, shown, tracked, trace_line, findings);
    if trace_type == "table" {
        inspect_js_plotly_table(trace, shown, tracked, trace_line, findings);
    }
}

/// Checks `line` and `marker` style sub-objects on a JS/TS Plotly trace literal.
fn inspect_plotly_trace_style(
    trace: &Node<'_>,
    shown: &str,
    tracked: &str,
    trace_line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(line_obj) = js_object_prop(trace, "line") {
        if js_object_prop(&line_obj, "shape")
            .and_then(|v| js_string(&v))
            .as_deref()
            == Some("spline")
        {
            findings.push(Finding {
                kind: Kind::SmoothedData,
                key: format!("{tracked}\nts-plotly-spline"),
                places: vec![FindingPlace::line(shown, trace_line)],
                evidence: "Plotly `line.shape: 'spline'` invents curvature between observations (VDQI p. 13). Connect points linearly.".to_owned(),
                weight: 2,
            });
        }
        if let Some(width) = js_object_prop(&line_obj, "width").and_then(|v| js_number(&v))
            && width >= 4.0
        {
            findings.push(Finding {
                kind: Kind::ThickLine,
                key: format!("{tracked}\nts-plotly-thick-line"),
                places: vec![FindingPlace::line(shown, trace_line)],
                evidence: format!(
                    "Plotly `line.width: {width}` draws a heavy stroke (VDQI p. 185). Keep data lines thin (1 to 2 px)."
                ),
                weight: 1,
            });
        }
    }
    if let Some(marker_obj) = js_object_prop(trace, "marker") {
        if js_object_prop(&marker_obj, "sizemode")
            .and_then(|v| js_string(&v))
            .as_deref()
            == Some("diameter")
        {
            findings.push(Finding {
                kind: Kind::DistortedSize,
                key: format!("{tracked}\nts-plotly-diameter"),
                places: vec![FindingPlace::line(shown, trace_line)],
                evidence: "Plotly `marker.sizemode: 'diameter'` squares area relative to value (VDQI p. 57). Use `sizemode: 'area'`.".to_owned(),
                weight: 3,
            });
        }
        if js_object_prop(&marker_obj, "pattern").is_some() {
            findings.push(Finding {
                kind: Kind::PatternFill,
                key: format!("{tracked}\nts-plotly-pattern"),
                places: vec![FindingPlace::line(shown, trace_line)],
                evidence: "Plotly `marker.pattern` introduces moire vibration (VDQI p. 108). Use flat muted colors or direct labels.".to_owned(),
                weight: 2,
            });
        }
    }
}

/// Checks a JS/TS Plotly `layout` object literal for Tufte violations.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "private helper invoked once from inspect_plotly_spec"
)]
fn inspect_plotly_layout(
    layout_obj: &Node<'_>,
    trace_count: usize,
    has_direct_text: bool,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if js_object_prop(layout_obj, "yaxis2").is_some() {
        findings.push(Finding {
            kind: Kind::DualAxis,
            key: format!("{tracked}\nts-plotly-layout-yaxis2"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: "Plotly layout defines `yaxis2` creating a second vertical scale (VDQI p. 61). Prefer small multiples.".to_owned(),
            weight: 3,
        });
    }
    if let Some(title_node) = js_object_prop(layout_obj, "title") {
        let title_text = js_string(&title_node)
            .or_else(|| js_object_prop(&title_node, "text").and_then(|v| js_string(&v)));
        if let Some(title) = title_text
            && is_all_caps(&title)
        {
            findings.push(Finding {
                kind: Kind::AllCapsText,
                key: format!("{tracked}\nts-plotly-caps\n{title}"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!(
                    "Plotly title `{title}` is written in all capitals (VDQI p. 183). Use sentence case."
                ),
                weight: 2,
            });
        }
    }
    for axis_key in ["xaxis", "yaxis"] {
        if let Some(axis_obj) = js_object_prop(layout_obj, axis_key) {
            inspect_plotly_axis(&axis_obj, axis_key, shown, tracked, line, findings);
        }
    }
    if let (Some(w), Some(h)) = (
        js_object_prop(layout_obj, "width").and_then(|v| js_number(&v)),
        js_object_prop(layout_obj, "height").and_then(|v| js_number(&v)),
    ) && h > w
    {
        findings.push(Finding {
            kind: Kind::VerticalAspectRatio,
            key: format!("{tracked}\nts-plotly-aspect"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "Plotly layout `{w}x{h}` is taller than wide (VDQI p. 186). Prefer a horizontal aspect ratio."
            ),
            weight: 2,
        });
    }
    if trace_count >= 2
        && !has_direct_text
        && js_object_prop(layout_obj, "showlegend").is_some_and(|v| v.text().trim() == "true")
    {
        findings.push(Finding {
            kind: Kind::LegendInsteadOfLabels,
            key: format!("{tracked}\nts-plotly-legend"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: "Multi-series Plotly chart relies on a legend with no direct labels (VDQI p. 183). Label series directly and set `showlegend: false`.".to_owned(),
            weight: 2,
        });
    }
}

/// Checks one `xaxis` or `yaxis` object in a JS/TS Plotly layout.
fn inspect_plotly_axis(
    axis_obj: &Node<'_>,
    axis_key: &str,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(angle) = js_object_prop(axis_obj, "tickangle").and_then(|v| js_number(&v))
        && angle.abs() > 0.0
        && (angle.abs() % 180.0) > 0.0
    {
        findings.push(Finding {
            kind: Kind::RotatedTicks,
            key: format!("{tracked}\nts-plotly-tickangle\n{axis_key}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "Plotly `{axis_key}.tickangle: {angle}` rotates labels (VDQI p. 183). Keep words horizontal."
            ),
            weight: 2,
        });
    }
    if js_object_prop(axis_obj, "showgrid").is_some_and(|v| v.text().trim() == "true") {
        findings.push(Finding {
            kind: Kind::HeavyGrid,
            key: format!("{tracked}\nts-plotly-grid\n{axis_key}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "Plotly `{axis_key}.showgrid: true` draws an explicit grid (VDQI p. 112). Erase the grid or mute it faintly."
            ),
            weight: 1,
        });
    }
    if js_object_prop(axis_obj, "mirror")
        .and_then(|v| js_string(&v))
        .is_some_and(|m| m == "true" || m == "all" || m == "ticks")
        || js_object_prop(axis_obj, "mirror").is_some_and(|v| v.text().trim() == "true")
    {
        findings.push(Finding {
            kind: Kind::BoxedFrame,
            key: format!("{tracked}\nts-plotly-mirror\n{axis_key}"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: format!(
                "Plotly `{axis_key}.mirror` encloses the chart in a non-data box frame (VDQI p. 96)."
            ),
            weight: 1,
        });
    }
}

/// Static checks on a JS/TS Plotly `{ type: 'table', header: ..., cells: ... }` trace.
fn inspect_js_plotly_table(
    trace: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(header) = js_object_prop(trace, "header")
        && let Some(values) = js_object_prop(&header, "values")
    {
        for raw in js_string_array(&values) {
            if is_all_caps(&raw) || looks_like_identifier(&raw) {
                findings.push(Finding {
                    kind: Kind::TableRawHeader,
                    key: format!("{tracked}\nts-table-header\n{raw}"),
                    places: vec![FindingPlace::line(shown, line)],
                    evidence: format!(
                        "Plotly table header `{raw}` is a raw identifier or all-caps label (VDQI p. 183)."
                    ),
                    weight: 2,
                });
            }
        }
    }
    if let Some(cells) = js_object_prop(trace, "cells") {
        if let Some(fill) = js_object_prop(&cells, "fill")
            && let Some(color) = js_object_prop(&fill, "color")
            && color.kind() == "array"
        {
            let first = color.named_children().next().map(unwrap_expr);
            let colors = first
                .as_ref()
                .filter(|n| n.kind() == "array")
                .map_or_else(|| js_string_array(&color), js_string_array);
            if colors.len() >= 2 && colors.iter().collect::<BTreeSet<_>>().len() >= 2 {
                findings.push(Finding {
                    kind: Kind::TableZebraStriping,
                    key: format!("{tracked}\nts-table-zebra"),
                    places: vec![FindingPlace::line(shown, line)],
                    evidence: "Plotly table applies alternating row fill colors (VDQI p. 179). Remove zebra striping.".to_owned(),
                    weight: 2,
                });
            }
        }
        if let Some(line_obj) = js_object_prop(&cells, "line")
            && let Some(width) = js_object_prop(&line_obj, "width").and_then(|v| js_number(&v))
            && width >= 2.0
        {
            findings.push(Finding {
                kind: Kind::TableVerticalRules,
                key: format!("{tracked}\nts-table-borders"),
                places: vec![FindingPlace::line(shown, line)],
                evidence: format!(
                    "Plotly table sets heavy cell borders (`line.width: {width}`, VDQI p. 179)."
                ),
                weight: 2,
            });
        }
    }
}

/// Static checks on React/TS AG Grid `columnDefs`, `gridOptions`, and `theme`.
fn inspect_ag_grid_props(
    col_defs: Option<&Node<'_>>,
    grid_opts: Option<&Node<'_>>,
    theme: Option<&Node<'_>>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    if let Some(array) = col_defs.filter(|node| node.kind() == "array") {
        inspect_ag_grid_cols(array, shown, tracked, findings);
    }
    if let Some(opts) = grid_opts {
        inspect_ag_grid_theme_params(opts, shown, tracked, line, findings);
    }
    if let Some(theme_node) = theme {
        inspect_ag_grid_theme_params(theme_node, shown, tracked, line, findings);
    }
}

/// Checks each object literal inside a JS/TS AG Grid `columnDefs` array.
fn inspect_ag_grid_cols(array: &Node<'_>, shown: &str, tracked: &str, findings: &mut Vec<Finding>) {
    let mut text_sorted = Vec::new();
    let mut any_numeric_sorted = false;
    let mut has_numeric = false;

    for item in array.named_children() {
        let item = unwrap_expr(item);
        if item.kind() != "object" {
            continue;
        }
        let col_line = item.start_pos().line() + 1;
        let field = js_object_prop(&item, "field").and_then(|v| js_string(&v));
        let header = js_object_prop(&item, "headerName").and_then(|v| js_string(&v));
        let col_type = js_object_prop(&item, "type").and_then(|v| js_string(&v));
        let data_type = js_object_prop(&item, "cellDataType").and_then(|v| js_string(&v));
        let has_formatter = js_object_prop(&item, "valueFormatter").is_some();
        let sort = js_object_prop(&item, "sort").and_then(|v| js_string(&v));

        if let Some(ref hdr) = header {
            if is_all_caps(hdr) || looks_like_identifier(hdr) {
                findings.push(Finding {
                    kind: Kind::TableRawHeader,
                    key: format!("{tracked}\nts-ag-header\n{hdr}"),
                    places: vec![FindingPlace::line(shown, col_line)],
                    evidence: format!(
                        "AG Grid `headerName: '{hdr}'` is a raw identifier or all-caps label (VDQI p. 183). Spell out headers in sentence case with units."
                    ),
                    weight: 2,
                });
            }
        } else if let Some(ref fld) = field
            && looks_like_identifier(fld)
        {
            findings.push(Finding {
                kind: Kind::TableRawHeader,
                key: format!("{tracked}\nts-ag-field-no-header\n{fld}"),
                places: vec![FindingPlace::line(shown, col_line)],
                evidence: format!(
                    "AG Grid column `field: '{fld}'` has no `headerName` (VDQI p. 183). Spell out the column header with units."
                ),
                weight: 2,
            });
        }

        let is_right_aligned = col_type
            .as_deref()
            .is_some_and(|t| t.contains("numericColumn") || t.contains("rightAligned"))
            || js_object_prop(&item, "cellStyle")
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
                    key: format!("{tracked}\nts-ag-unformatted\n{label}"),
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
                    key: format!("{tracked}\nts-ag-align\n{label}"),
                    places: vec![FindingPlace::line(shown, col_line)],
                    evidence: format!(
                        "AG Grid numeric column `{label}` is not right-aligned (VDQI p. 178). Set `type: 'numericColumn'` or `type: 'rightAligned'`."
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
                key: format!("{tracked}\nts-ag-alpha-sort\n{label}"),
                places: vec![FindingPlace::line(shown, col_line)],
                evidence: format!(
                    "AG Grid sorts text column `{label}` ascending while numeric columns are unsorted (VDQI p. 178). Order rows by content or value."
                ),
                weight: 2,
            });
        }
    }
}

/// Checks an AG Grid options or `theme.withParams(...)` node for vertical borders or zebra striping.
fn inspect_ag_grid_theme_params(
    node: &Node<'_>,
    shown: &str,
    tracked: &str,
    line: usize,
    findings: &mut Vec<Finding>,
) {
    let text = node.text();
    if text.contains("columnBorder") && text.contains("true") {
        findings.push(Finding {
            kind: Kind::TableVerticalRules,
            key: format!("{tracked}\nts-ag-col-border"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: "AG Grid enables vertical `columnBorder` lines (VDQI p. 179). Remove vertical gridlines and rely on alignment and horizontal topical dividers.".to_owned(),
            weight: 2,
        });
    }
    if text.contains("oddRowBackgroundColor") {
        findings.push(Finding {
            kind: Kind::TableZebraStriping,
            key: format!("{tracked}\nts-ag-zebra"),
            places: vec![FindingPlace::line(shown, line)],
            evidence: "AG Grid sets `oddRowBackgroundColor` zebra striping (VDQI p. 179). Use plain backgrounds with subtle horizontal dividers.".to_owned(),
            weight: 2,
        });
    }
}
