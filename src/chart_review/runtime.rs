//! Runtime adapter execution and unified Rust-native fact analysis for Plotly charts,
//! Plotly `go.Table`, AG Grid specs (Python `dash_ag_grid` and React/TS `ag-grid`), and
//! Matplotlib/Seaborn figures.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use regex::Regex;
use serde_json::{Value, json};

use crate::ledger::adapter_runner::invoke_adapter;
use crate::ledger::{AdapterPolicy, FindingPlace};

use super::{Finding, Kind, is_all_caps, looks_like_identifier};

/// The Python extractor embedded in xpd for `.py` adapters.
const FIGURE_FACTS_PY: &str = include_str!("../../py/figure_facts.py");

/// CSS pixels per inch for Plotly canvases.
const CSS_DPI: f64 = 96.0;
/// Maximum number of drawn values before Tufte recommends a table or sentence (VDQI p. 33).
const SMALL_DATA: usize = 20;
/// Maximum rows in a flat table before Tufte recommends topical row groups (VDQI p. 179).
const UNGROUPED_TABLE_ROWS: usize = 12;
/// Minimum rows or bars required to diagnose alphabetical sorting against unsorted values.
const ALPHA_MIN_ITEMS: usize = 4;
/// Minimum line segments required to evaluate 45-degree banking slope.
const BANKING_MIN_SEGMENTS: usize = 6;

static MONEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(€|\$|£|¥|\beur\b|\beuros?\b|\busd\b|\bdollars?\b|\bdkk\b|\bkroner\b|\bgbp\b|\bprice|\bcost|\brevenue|\bincome|\bwage|\bsalar)")
        .expect("valid money regex")
});

static BASIS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(nominal|real|constant|deflated|inflation|adjusted|current prices|\d{4} (euro|dollar|prices))")
        .expect("valid basis regex")
});

static HTML_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid html tag regex"));

static YEAR_STR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d{4})(-\d{2}(-\d{2})?)?([ T].*)?$").expect("valid year regex")
});

/// Outcome of running an adapter and analyzing its figures.
#[derive(Debug)]
pub(crate) struct RuntimeResult {
    pub(crate) summary: Value,
    pub(crate) findings: Vec<Finding>,
}

/// Resolved style metadata for one plotted series.
struct TraceStyle {
    name: String,
    color: Option<String>,
    dash: Option<String>,
    markers: bool,
    direct_labels: bool,
}

/// Context passed into runtime analyzers for one named figure.
struct FigureCtx<'a> {
    name: &'a str,
    adapter: &'a str,
}

impl FigureCtx<'_> {
    fn push(
        &self,
        findings: &mut Vec<Finding>,
        kind: Kind,
        detail_key: &str,
        evidence: &str,
        weight: usize,
    ) {
        findings.push(Finding {
            kind,
            key: format!("runtime\n{}\n{detail_key}", self.name),
            places: vec![FindingPlace::file(self.adapter)],
            evidence: format!("[{}] {evidence}", self.name),
            weight,
        });
    }
}

/// Runs `adapter` in `work_dir` and evaluates every returned figure into facts and findings.
pub(crate) fn measure(
    adapter: &str,
    work_dir: &Path,
    policy: AdapterPolicy,
) -> Result<RuntimeResult> {
    let payload = match invoke_adapter(adapter, work_dir, FIGURE_FACTS_PY) {
        Ok(payload) => payload,
        Err(error) => {
            if policy == AdapterPolicy::Required {
                return Err(error);
            }
            return Ok(RuntimeResult {
                summary: json!({"adapter": adapter, "figures": [], "error": error.to_string()}),
                findings: Vec::new(),
            });
        }
    };

    if let Some(error) = payload.get("error").and_then(Value::as_str) {
        if policy == AdapterPolicy::Required {
            bail!("adapter `{adapter}` failed: {error}");
        }
        return Ok(RuntimeResult {
            summary: json!({"adapter": adapter, "figures": [], "error": error}),
            findings: Vec::new(),
        });
    }

    let entries = normalize_entries(&payload)?;
    let mut analyzed_figures = Vec::new();
    let mut findings = Vec::new();

    for entry in entries {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("figure")
            .to_owned();
        let spec = entry.get("spec").unwrap_or(&Value::Null);
        let raw_library = entry
            .get("library")
            .or_else(|| entry.get("kind"))
            .and_then(Value::as_str);
        let library = match raw_library {
            Some("ag-grid" | "ag_grid" | "aggrid") => "ag-grid".to_owned(),
            Some(other) => other.to_owned(),
            None if spec.get("columnDefs").is_some() || spec.get("rowData").is_some() => {
                "ag-grid".to_owned()
            }
            None => "plotly".to_owned(),
        };
        let unresolved = entry
            .get("unresolved")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let ctx = FigureCtx {
            name: &name,
            adapter,
        };

        let facts = match library.as_str() {
            "ag-grid" => analyze_ag_grid(&ctx, spec, &mut findings),
            "matplotlib" => analyze_matplotlib(
                &ctx,
                entry.get("raw").unwrap_or(&Value::Null),
                &mut findings,
            ),
            _ => analyze_plotly(&ctx, spec, &mut findings),
        };

        analyzed_figures.push(json!({
            "name": name,
            "library": library,
            "unresolved": unresolved,
            "facts": facts,
        }));
    }

    Ok(RuntimeResult {
        summary: json!({
            "adapter": adapter,
            "figures": analyzed_figures,
        }),
        findings,
    })
}

/// Normalizes either `{"figures": [...]}` or `{"figures": {"name": {...}}}` into a sorted list.
fn normalize_entries(payload: &Value) -> Result<Vec<Value>> {
    let Some(figures) = payload.get("figures") else {
        bail!("adapter JSON must hold a `figures` array or object");
    };
    if let Some(array) = figures.as_array() {
        return Ok(array.clone());
    }
    if let Some(map) = figures.as_object() {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            let val = &map[key];
            if val.get("spec").is_some() || val.get("raw").is_some() {
                let mut obj = val.clone();
                obj["name"] = json!(key);
                out.push(obj);
            } else if val.get("columnDefs").is_some() || val.get("rowData").is_some() {
                out.push(json!({"name": key, "library": "ag-grid", "spec": val}));
            } else {
                out.push(json!({"name": key, "library": "plotly", "spec": val}));
            }
        }
        return Ok(out);
    }
    bail!("adapter `figures` field must be an array or object")
}

fn plain_text(value: Option<&Value>) -> String {
    let Some(text) = value.and_then(Value::as_str) else {
        return String::new();
    };
    if text.starts_with("Click to enter") {
        return String::new();
    }
    HTML_TAG_RE.replace_all(text, "").trim().to_owned()
}

fn usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

fn year_of(value: &Value) -> Option<i32> {
    if let Some(num) = value.as_i64()
        && (1800..=2200).contains(&num)
    {
        return i32::try_from(num).ok();
    }
    if let Some(num) = value.as_f64()
        && num.fract() == 0.0
        && (1800.0..=2200.0).contains(&num)
    {
        return format!("{num:.0}").parse::<i32>().ok();
    }
    if let Some(text) = value.as_str()
        && let Some(caps) = YEAR_STR_RE.captures(text.trim())
        && let Ok(year) = caps[1].parse::<i32>()
        && (1800..=2200).contains(&year)
    {
        return Some(year);
    }
    None
}

fn parse_rgb(color: &str) -> Option<(f64, f64, f64)> {
    let trimmed = color.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "red" | "crimson" | "firebrick" | "darkred" => return Some((0.86, 0.08, 0.08)),
        "green" | "lime" | "forestgreen" | "darkgreen" | "seagreen" => {
            return Some((0.08, 0.55, 0.08));
        }
        _ => {}
    }
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some((
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
            ));
        }
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some((
                f64::from(r) / 255.0,
                f64::from(g) / 255.0,
                f64::from(b) / 255.0,
            ));
        }
    }
    if let Some(inner) = trimmed
        .strip_prefix("rgb(")
        .or_else(|| trimmed.strip_prefix("rgba("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<f64> = inner
            .split(',')
            .take(3)
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .collect();
        if parts.len() == 3 {
            return Some((parts[0] / 255.0, parts[1] / 255.0, parts[2] / 255.0));
        }
    }
    None
}

fn red_or_green(color: Option<&str>) -> Option<&'static str> {
    let (r, g, b) = parse_rgb(color?)?;
    if r >= 0.6 && g <= 0.35 && b <= 0.35 {
        Some("red")
    } else if g >= 0.4 && r <= 0.35 && b <= 0.35 {
        Some("green")
    } else {
        None
    }
}

fn is_alpha_with_unsorted_values(labels: &[String], values: &[f64]) -> bool {
    if labels.len() < ALPHA_MIN_ITEMS || labels.len() != values.len() {
        return false;
    }
    let lowered: Vec<String> = labels.iter().map(|s| s.to_ascii_lowercase()).collect();
    if !lowered.windows(2).all(|w| w[0] <= w[1]) {
        return false;
    }
    let val_asc = values.windows(2).all(|w| w[0] <= w[1]);
    let val_desc = values.windows(2).all(|w| w[0] >= w[1]);
    !val_asc && !val_desc
}

fn mixes_zero_and_missing(values: &[&Value]) -> bool {
    let mut has_zero = false;
    let mut has_missing = false;
    for val in values {
        match val {
            Value::Null => has_missing = true,
            Value::String(s) if s.trim().is_empty() || s.eq_ignore_ascii_case("nan") => {
                has_missing = true;
            }
            Value::Number(n) if n.as_f64().is_some_and(|v| v == 0.0) => has_zero = true,
            _ => {}
        }
    }
    has_zero && has_missing
}

fn decimal_digits(val: f64) -> usize {
    if !val.is_finite() || val.fract() == 0.0 {
        return 0;
    }
    let formatted = format!("{val}");
    formatted
        .split_once('.')
        .map_or(0, |(_, frac)| frac.trim_end_matches('0').len())
}

/// Accumulated facts across all Plotly traces in one figure.
#[derive(Default)]
struct PlotlyAcc {
    panels: BTreeSet<(String, String)>,
    total_values: usize,
    pie_slices: usize,
    marks: Vec<Value>,
    styles: Vec<TraceStyle>,
    x_summaries: Vec<Value>,
    any_years_span: Option<i32>,
    has_bar_or_area: bool,
    max_trace_points: usize,
    two_point_year_traces: Vec<String>,
    y_bounds: BTreeMap<String, (f64, f64)>,
}

/// Analyzes a Plotly `{ data, layout }` spec (both charts and `go.Table` traces).
fn analyze_plotly(ctx: &FigureCtx<'_>, spec: &Value, findings: &mut Vec<Finding>) -> Value {
    let layout = spec.get("layout").unwrap_or(&Value::Null);
    let traces: Vec<&Value> = spec
        .get("data")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|t| t.get("visible").and_then(Value::as_bool).unwrap_or(true))
                .collect()
        })
        .unwrap_or_default();

    let table_traces: Vec<&Value> = traces
        .iter()
        .copied()
        .filter(|t| t.get("type").and_then(Value::as_str) == Some("table"))
        .collect();
    if !table_traces.is_empty() && table_traces.len() == traces.len() {
        let mut first_facts = Value::Null;
        for (idx, table) in table_traces.iter().enumerate() {
            let facts = if idx == 0 {
                analyze_plotly_table(ctx, table, layout, findings)
            } else {
                let sub_name = format!("{}:table {idx}", ctx.name);
                let sub_ctx = FigureCtx {
                    name: &sub_name,
                    adapter: ctx.adapter,
                };
                analyze_plotly_table(&sub_ctx, table, layout, findings)
            };
            if idx == 0 {
                first_facts = facts;
            }
        }
        return first_facts;
    }

    let canvas = (
        layout.get("width").and_then(Value::as_f64),
        layout.get("height").and_then(Value::as_f64),
    );
    let mut acc = PlotlyAcc::default();
    for (idx, trace) in traces.iter().enumerate() {
        collect_plotly_trace(ctx, idx, trace, layout, canvas, &mut acc, findings);
    }
    if acc.max_trace_points <= 2 {
        for trace_name in &acc.two_point_year_traces {
            ctx.push(
                findings,
                Kind::MissingContext,
                &format!("two-points-{trace_name}"),
                &format!(
                    "Trace `{trace_name}` plots only two x values with no baseline context (VDQI p. 74)."
                ),
                2,
            );
        }
    }
    check_plotly_canvas_and_colors(ctx, layout, canvas, &acc, findings);
    let (title, axis_titles, annotations) = check_plotly_layout_axes(ctx, layout, &acc, findings);
    check_plotly_annotations(ctx, layout, &acc, findings);

    let panel_count = acc.panels.len().max(1);
    let density = match canvas {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => {
            let area = (w * h) / (CSS_DPI * CSS_DPI);
            Some(((usize_to_f64(acc.total_values) / area) * 100.0).round() / 100.0)
        }
        _ => None,
    };

    json!({
        "canvas": {"width": canvas.0, "height": canvas.1, "panels": panel_count},
        "values": {"total": acc.total_values, "per_square_inch": density, "marks": acc.marks},
        "x-values": acc.x_summaries,
        "text": {"title": title, "axis_titles": axis_titles, "annotations": annotations},
    })
}

fn collect_plotly_trace(
    ctx: &FigureCtx<'_>,
    idx: usize,
    trace: &Value,
    layout: &Value,
    canvas: (Option<f64>, Option<f64>),
    acc: &mut PlotlyAcc,
    findings: &mut Vec<Finding>,
) {
    let trace_type = trace
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("scatter");
    let trace_name = trace
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("trace {idx}"), str::to_owned);
    if trace_type == "table" {
        let sub_name = format!("{}:{trace_name}", ctx.name);
        let sub_ctx = FigureCtx {
            name: &sub_name,
            adapter: ctx.adapter,
        };
        analyze_plotly_table(&sub_ctx, trace, layout, findings);
        return;
    }
    let mode = trace.get("mode").and_then(Value::as_str).unwrap_or("");
    let is_line = trace_type == "scatter" && (mode.is_empty() || mode.contains("lines"));
    let count = plotly_trace_count(trace, trace_type);
    acc.total_values += count;
    if matches!(trace_type, "pie" | "sunburst") {
        acc.pie_slices += count;
    }
    if matches!(trace_type, "bar" | "histogram")
        || trace
            .get("fill")
            .and_then(Value::as_str)
            .is_some_and(|f| f != "none")
    {
        acc.has_bar_or_area = true;
    }
    acc.marks
        .push(json!({"trace": trace_name, "mark": trace_type, "values": count}));
    if matches!(
        trace_type,
        "pie" | "sunburst" | "treemap" | "funnelarea" | "indicator"
    ) {
        acc.panels.insert(("domain".to_owned(), format!("{idx}")));
    } else {
        let xa = trace.get("xaxis").and_then(Value::as_str).unwrap_or("x");
        let ya = trace.get("yaxis").and_then(Value::as_str).unwrap_or("y");
        acc.panels.insert((xa.to_owned(), ya.to_owned()));
    }

    let color = if is_line {
        trace.pointer("/line/color")
    } else {
        trace.pointer("/marker/color")
    }
    .and_then(Value::as_str)
    .map(str::to_owned);
    acc.styles.push(TraceStyle {
        name: trace_name.clone(),
        color,
        dash: trace
            .pointer("/line/dash")
            .and_then(Value::as_str)
            .map(str::to_owned),
        markers: mode.contains("markers"),
        direct_labels: mode.contains("text")
            || trace.get("texttemplate").and_then(Value::as_str).is_some(),
    });

    if let Some(lw) = trace.pointer("/line/width").and_then(Value::as_f64)
        && lw >= 4.0
    {
        ctx.push(
            findings,
            Kind::ThickLine,
            &format!("thick-line-{trace_name}"),
            &format!("Trace `{trace_name}` has line width {lw} px (VDQI p. 185)."),
            1,
        );
    }
    let panel_canvas = panel_dimensions(trace, layout, canvas);
    check_plotly_trace_data(
        ctx,
        trace,
        trace_type,
        &trace_name,
        is_line,
        panel_canvas,
        acc,
        findings,
    );
}

fn axis_layout_key(short: &str, prefix: &str) -> String {
    match short.strip_prefix(prefix) {
        Some("") | None => format!("{prefix}axis"),
        Some(suffix) => format!("{prefix}axis{suffix}"),
    }
}

fn domain_span(layout: &Value, key: &str) -> f64 {
    layout
        .get(key)
        .and_then(|axis| axis.get("domain"))
        .and_then(Value::as_array)
        .filter(|d| d.len() == 2)
        .and_then(|d| Some((d[0].as_f64()?, d[1].as_f64()?)))
        .map_or(1.0, |(lo, hi)| (hi - lo).clamp(0.05, 1.0))
}

fn panel_dimensions(
    trace: &Value,
    layout: &Value,
    canvas: (Option<f64>, Option<f64>),
) -> (Option<f64>, Option<f64>) {
    let x_key = axis_layout_key(
        trace.get("xaxis").and_then(Value::as_str).unwrap_or("x"),
        "x",
    );
    let y_key = axis_layout_key(
        trace.get("yaxis").and_then(Value::as_str).unwrap_or("y"),
        "y",
    );
    (
        canvas.0.map(|w| w * domain_span(layout, &x_key)),
        canvas.1.map(|h| h * domain_span(layout, &y_key)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "internal trace data validator sharing accumulated Plotly figure state"
)]
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "private helper invoked once from collect_plotly_trace"
)]
fn check_plotly_trace_data(
    ctx: &FigureCtx<'_>,
    trace: &Value,
    trace_type: &str,
    trace_name: &str,
    is_line: bool,
    canvas: (Option<f64>, Option<f64>),
    acc: &mut PlotlyAcc,
    findings: &mut Vec<Finding>,
) {
    if let Some(xs) = trace.get("x").and_then(Value::as_array) {
        acc.max_trace_points = acc.max_trace_points.max(xs.len());
        let years: BTreeSet<i32> = xs.iter().filter_map(year_of).collect();
        if xs.len() == 2 && years.len() == 2 {
            acc.two_point_year_traces.push(trace_name.to_owned());
        }
        if years.len() >= 2
            && let (Some(first), Some(last)) = (years.first(), years.last())
        {
            let span = last - first + 1;
            acc.any_years_span = Some(acc.any_years_span.map_or(span, |prev| prev.max(span)));
        }
        acc.x_summaries
            .push(json!({"trace": trace_name, "count": xs.len()}));
        if trace_type == "bar"
            && let Some(ys) = trace.get("y").and_then(Value::as_array)
        {
            let labels: Vec<String> = xs
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            let nums: Vec<f64> = ys.iter().filter_map(Value::as_f64).collect();
            if is_alpha_with_unsorted_values(&labels, &nums) {
                ctx.push(
                    findings,
                    Kind::AlphabeticalOrder,
                    &format!("alpha-bar-{trace_name}"),
                    &format!(
                        "Bars in `{trace_name}` are ordered alphabetically while values are unsorted (VDQI p. 178)."
                    ),
                    2,
                );
            }
        }
    }
    if let Some(ys) = trace.get("y").and_then(Value::as_array) {
        let y_axis = trace
            .get("yaxis")
            .and_then(Value::as_str)
            .unwrap_or("y")
            .to_owned();
        for num in ys
            .iter()
            .filter_map(Value::as_f64)
            .filter(|v| v.is_finite())
        {
            acc.y_bounds
                .entry(y_axis.clone())
                .and_modify(|(lo, hi)| {
                    *lo = lo.min(num);
                    *hi = hi.max(num);
                })
                .or_insert((num, num));
        }
        let refs: Vec<&Value> = ys.iter().collect();
        if mixes_zero_and_missing(&refs) {
            ctx.push(
                findings,
                Kind::ZeroMissingAmbiguity,
                &format!("zero-missing-{trace_name}"),
                &format!(
                    "Trace `{trace_name}` mixes exact zeros and missing values (VDQI p. 145)."
                ),
                1,
            );
        }
    }
    if is_line
        && let (Some(w), Some(h)) = canvas
        && let (Some(xs), Some(ys)) = (
            trace.get("x").and_then(Value::as_array),
            trace.get("y").and_then(Value::as_array),
        )
        && let Some(angle) = median_banking_angle(xs, ys, w, h)
        && !(15.0..=75.0).contains(&angle)
    {
        ctx.push(
            findings,
            Kind::PoorBanking,
            &format!("banking-{trace_name}"),
            &format!(
                "Line `{trace_name}` has median absolute slope angle {angle:.0} degrees (target near 45 degrees, VDQI pp. 188 to 190)."
            ),
            1,
        );
    }
}

fn check_plotly_canvas_and_colors(
    ctx: &FigureCtx<'_>,
    layout: &Value,
    canvas: (Option<f64>, Option<f64>),
    acc: &PlotlyAcc,
    findings: &mut Vec<Finding>,
) {
    let panel_count = acc.panels.len().max(1);
    if let (Some(w), Some(h)) = canvas
        && panel_count == 1
        && h > w
    {
        ctx.push(
            findings,
            Kind::VerticalAspectRatio,
            "vertical-canvas",
            &format!("Single-panel canvas {w:.0} by {h:.0} px is taller than wide (VDQI p. 186)."),
            2,
        );
    }
    if acc.total_values > 0 && acc.total_values <= SMALL_DATA {
        ctx.push(
            findings,
            Kind::SmallDataChart,
            "small-data",
            &format!(
                "Chart draws only {} values ({SMALL_DATA} or fewer), which a table or sentence shows better (VDQI p. 33).",
                acc.total_values
            ),
            2,
        );
    }
    if acc.pie_slices > 0 {
        ctx.push(
            findings,
            Kind::PieChart,
            "pie-slices",
            &format!(
                "Pie draws {} slices (VDQI p. 178). Replace with a sorted bar chart or a table.",
                acc.pie_slices
            ),
            3,
        );
    }
    let has_annotations = layout
        .get("annotations")
        .and_then(Value::as_array)
        .is_some_and(|arr| arr.iter().any(|a| !plain_text(a.get("text")).is_empty()));
    let any_direct = has_annotations || acc.styles.iter().any(|s| s.direct_labels);
    for (idx, first) in acc.styles.iter().enumerate() {
        for second in &acc.styles[idx + 1..] {
            let same_shape = first.dash == second.dash && first.markers == second.markers;
            let pair = (
                red_or_green(first.color.as_deref()),
                red_or_green(second.color.as_deref()),
            );
            if matches!(
                pair,
                (Some("red"), Some("green")) | (Some("green"), Some("red"))
            ) && same_shape
                && !any_direct
            {
                ctx.push(
                    findings,
                    Kind::RedGreenOnly,
                    &format!("rg-{}-{}", first.name, second.name),
                    &format!(
                        "`{}` and `{}` differ only by red against green with no direct labels (VDQI p. 183).",
                        first.name, second.name
                    ),
                    3,
                );
            }
        }
    }
}

fn check_plotly_layout_axes(
    ctx: &FigureCtx<'_>,
    layout: &Value,
    acc: &PlotlyAcc,
    findings: &mut Vec<Finding>,
) -> (String, Vec<String>, Vec<String>) {
    let title = plain_text(
        layout
            .pointer("/title/text")
            .or_else(|| layout.get("title")),
    );
    if !title.is_empty() && is_all_caps(&title) {
        ctx.push(
            findings,
            Kind::AllCapsText,
            "caps-title",
            &format!("Title `{title}` is written in all capitals (VDQI p. 183)."),
            2,
        );
    }
    let annotations: Vec<String> = layout
        .get("annotations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|a| plain_text(a.get("text")))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut axis_titles = Vec::new();
    if let Some(layout_map) = layout.as_object() {
        for (key, axis) in layout_map {
            if key.starts_with("yaxis")
                && key != "yaxis"
                && axis
                    .get("overlaying")
                    .and_then(Value::as_str)
                    .is_some_and(|ov| ov.starts_with('y'))
            {
                ctx.push(
                    findings,
                    Kind::DualAxis,
                    key,
                    &format!(
                        "Figure defines an overlaying secondary vertical axis `{key}` (VDQI p. 61)."
                    ),
                    3,
                );
            }
            if !(key.starts_with("xaxis") || key.starts_with("yaxis")) {
                continue;
            }
            check_plotly_single_axis(ctx, key, axis, acc, &mut axis_titles, findings);
        }
    }
    let words = format!(
        "{title} {} {}",
        axis_titles.join(" "),
        annotations.join(" ")
    );
    if acc.any_years_span.is_some_and(|span| span >= 2)
        && let Some(matched) = MONEY_RE.find(&words)
        && !BASIS_RE.is_match(&words)
    {
        let span_note = acc
            .any_years_span
            .map(|span| format!(" over {span} years"))
            .unwrap_or_default();
        ctx.push(
            findings,
            Kind::UndeflatedMoney,
            "undeflated-money",
            &format!(
                "Figure shows money (`{}`){span_note} without stating nominal, real, or constant units (VDQI p. 68).",
                matched.as_str()
            ),
            2,
        );
    }
    (title, axis_titles, annotations)
}

fn check_plotly_annotations(
    ctx: &FigureCtx<'_>,
    layout: &Value,
    acc: &PlotlyAcc,
    findings: &mut Vec<Finding>,
) {
    let Some(items) = layout.get("annotations").and_then(Value::as_array) else {
        return;
    };
    let mut data_notes: Vec<(String, String, Option<&Value>, f64)> = Vec::new();
    for item in items {
        let text = plain_text(item.get("text"));
        if text.is_empty() {
            continue;
        }
        let yref = item.get("yref").and_then(Value::as_str).unwrap_or("y");
        if !yref.starts_with('y') {
            continue;
        }
        let Some(y_val) = item.get("y").and_then(Value::as_f64) else {
            continue;
        };
        let norm_ref = if yref == "y1" { "y" } else { yref };
        data_notes.push((text, norm_ref.to_owned(), item.get("x"), y_val));
    }

    for (text, yref, _, y_val) in &data_notes {
        let Some(&(lo, hi)) = acc.y_bounds.get(yref) else {
            continue;
        };
        let span = hi - lo;
        if span <= 0.0 {
            continue;
        }
        let axis_key = axis_layout_key(yref, "y");
        let has_range = layout
            .get(&axis_key)
            .and_then(|axis| axis.get("range"))
            .and_then(Value::as_array)
            .is_some_and(|r| r.len() == 2);
        let rel = (*y_val - lo) / span;
        if !has_range && (!(0.05..=0.95).contains(&rel)) {
            ctx.push(
                findings,
                Kind::LabelCollision,
                &format!("edge-label-{text}"),
                &format!(
                    "Annotation `{text}` sits on the `{axis_key}` edge without explicit range padding and risks colliding with the axis or ticks (VDQI p. 183)."
                ),
                2,
            );
        }
    }

    for (idx, (t1, yref1, x1, y1)) in data_notes.iter().enumerate() {
        for (t2, yref2, x2, y2) in &data_notes[idx + 1..] {
            if yref1 != yref2 || x1.is_none() || x1 != x2 {
                continue;
            }
            if let Some(&(lo, hi)) = acc.y_bounds.get(yref1)
                && hi > lo
                && (y1 - y2).abs() / (hi - lo) < 0.08
            {
                ctx.push(
                    findings,
                    Kind::LabelCollision,
                    &format!("overlap-label-{t1}-{t2}"),
                    &format!(
                        "Annotations `{t1}` and `{t2}` sit within 8% of the y-span at the same x coordinate and risk overlapping (VDQI p. 183)."
                    ),
                    2,
                );
            }
        }
    }
}

fn check_plotly_single_axis(
    ctx: &FigureCtx<'_>,
    key: &str,
    axis: &Value,
    acc: &PlotlyAcc,
    axis_titles: &mut Vec<String>,
    findings: &mut Vec<Finding>,
) {
    let axis_title = plain_text(axis.pointer("/title/text").or_else(|| axis.get("title")));
    if !axis_title.is_empty() {
        axis_titles.push(axis_title);
    }
    if let Some(angle) = axis.get("tickangle").and_then(Value::as_f64)
        && angle.abs() > 0.0
        && (angle.abs() % 180.0) > 0.0
    {
        ctx.push(
            findings,
            Kind::RotatedTicks,
            &format!("tickangle-{key}"),
            &format!("`{key}` tick labels are rotated by {angle:.0} degrees (VDQI p. 183)."),
            2,
        );
    }
    if key.starts_with("yaxis")
        && acc.has_bar_or_area
        && let Some(range) = axis.get("range").and_then(Value::as_array)
        && range.len() == 2
        && let (Some(lo), Some(hi)) = (range[0].as_f64(), range[1].as_f64())
        && lo > 0.0
        && hi > lo
    {
        ctx.push(
            findings,
            Kind::NonZeroBaseline,
            &format!("nonzero-{key}"),
            &format!("Bar or area `{key}` range starts at {lo} instead of zero (VDQI p. 54)."),
            3,
        );
    }
    if let Some(tickvals) = axis.get("tickvals").and_then(Value::as_array)
        && acc.total_values > 0
        && tickvals.len() > acc.total_values
    {
        ctx.push(
            findings,
            Kind::ExcessTicks,
            &format!("excess-ticks-{key}"),
            &format!(
                "`{key}` draws {} explicit ticks for only {} data values (VDQI p. 120).",
                tickvals.len(),
                acc.total_values
            ),
            2,
        );
    }
}

fn plotly_trace_count(trace: &Value, trace_type: &str) -> usize {
    if matches!(
        trace_type,
        "pie" | "sunburst" | "treemap" | "funnelarea" | "indicator"
    ) {
        return trace
            .get("values")
            .or_else(|| trace.get("labels"))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
    }
    if trace_type == "bar" {
        let key = if trace.get("orientation").and_then(Value::as_str) == Some("h") {
            "x"
        } else {
            "y"
        };
        return trace.get(key).and_then(Value::as_array).map_or(0, Vec::len);
    }
    if matches!(trace_type, "heatmap" | "contour" | "surface") {
        return trace.get("z").and_then(Value::as_array).map_or(0, |rows| {
            rows.iter()
                .map(|row| row.as_array().map_or(1, Vec::len))
                .sum()
        });
    }
    trace
        .get("x")
        .or_else(|| trace.get("y"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn median_banking_angle(xs: &[Value], ys: &[Value], width: f64, height: f64) -> Option<f64> {
    let pairs: Vec<(f64, f64)> = xs
        .iter()
        .zip(ys.iter())
        .enumerate()
        .filter_map(|(idx, (x, y))| {
            let xv = x.as_f64().unwrap_or_else(|| usize_to_f64(idx));
            let yv = y.as_f64()?;
            (xv.is_finite() && yv.is_finite()).then_some((xv, yv))
        })
        .collect();
    if pairs.len() <= BANKING_MIN_SEGMENTS {
        return None;
    }
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(x, y) in &pairs {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let span_x = max_x - min_x;
    let span_y = max_y - min_y;
    if span_x <= 0.0 || span_y <= 0.0 {
        return None;
    }
    let mut angles: Vec<f64> = pairs
        .windows(2)
        .filter_map(|seg| {
            let dx = ((seg[1].0 - seg[0].0) / span_x) * width;
            let dy = ((seg[1].1 - seg[0].1) / span_y) * height;
            (dx.abs() > 1e-9 || dy.abs() > 1e-9).then(|| dy.abs().atan2(dx.abs()).to_degrees())
        })
        .collect();
    if angles.len() < BANKING_MIN_SEGMENTS {
        return None;
    }
    angles.sort_by(f64::total_cmp);
    Some(angles[angles.len() / 2])
}

/// Analyzes a Plotly `go.Table` trace against Tufte's tabular presentation rules (VDQI pp. 178 to 181).
fn analyze_plotly_table(
    ctx: &FigureCtx<'_>,
    trace: &Value,
    layout: &Value,
    findings: &mut Vec<Finding>,
) -> Value {
    let headers: Vec<String> = trace
        .pointer("/header/values")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|v| plain_text(Some(v))).collect())
        .unwrap_or_default();
    let columns: Vec<Vec<&Value>> = trace
        .pointer("/cells/values")
        .and_then(Value::as_array)
        .map(|cols| {
            cols.iter()
                .map(|col| col.as_array().map_or_else(Vec::new, |a| a.iter().collect()))
                .collect()
        })
        .unwrap_or_default();
    let row_count = columns.first().map_or(0, Vec::len);

    check_plotly_table_chrome(ctx, trace, &headers, row_count, findings);
    check_plotly_table_columns(ctx, trace, &headers, &columns, row_count, findings);

    let title = plain_text(
        layout
            .pointer("/title/text")
            .or_else(|| layout.get("title")),
    );
    if !title.is_empty() && is_all_caps(&title) {
        ctx.push(
            findings,
            Kind::AllCapsText,
            "table-caps-title",
            &format!("Table title `{title}` is written in all capitals (VDQI p. 183)."),
            2,
        );
    }

    json!({
        "table-structure": {"rows": row_count, "columns": headers.len(), "headers": headers},
        "text": {"title": title},
    })
}

fn check_plotly_table_chrome(
    ctx: &FigureCtx<'_>,
    trace: &Value,
    headers: &[String],
    row_count: usize,
    findings: &mut Vec<Finding>,
) {
    for header in headers {
        if is_all_caps(header) || looks_like_identifier(header) {
            ctx.push(
                findings,
                Kind::TableRawHeader,
                &format!("table-header-{header}"),
                &format!(
                    "Table header `{header}` is a raw identifier or all-caps label (VDQI p. 183)."
                ),
                2,
            );
        }
    }
    if let Some(fill) = trace.pointer("/cells/fill/color").and_then(Value::as_array) {
        let palette: BTreeSet<String> = fill
            .iter()
            .flat_map(|item| {
                item.as_array().map_or_else(
                    || {
                        item.as_str()
                            .map(|s| vec![s.to_owned()])
                            .unwrap_or_default()
                    },
                    |inner| {
                        inner
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    },
                )
            })
            .collect();
        if palette.len() >= 2 {
            ctx.push(
                findings,
                Kind::TableZebraStriping,
                "table-zebra",
                "Table applies multiple alternating cell fill colors (VDQI p. 179). Remove zebra striping.",
                2,
            );
        }
    }
    if let Some(width) = trace.pointer("/cells/line/width").and_then(Value::as_f64)
        && width >= 2.0
    {
        ctx.push(
            findings,
            Kind::TableVerticalRules,
            "table-borders",
            &format!("Table draws heavy cell borders (`line.width={width}`, VDQI p. 179)."),
            2,
        );
    }
    if trace.pointer("/header/fill").is_none() && trace.pointer("/cells/fill").is_none() {
        ctx.push(
            findings,
            Kind::TableVerticalRules,
            "table-default-chrome",
            "Table uses default Plotly gray cell fills and white gridlines (VDQI p. 179). Set flat `header.fill`, `cells.fill`, and `cells.line.width=0`.",
            2,
        );
    }
    if row_count > UNGROUPED_TABLE_ROWS {
        ctx.push(
            findings,
            Kind::TableUngroupedRows,
            "table-ungrouped",
            &format!(
                "Table has {row_count} flat rows with no topical grouping (VDQI p. 179). Group into clusters of 3 to 7 rows."
            ),
            2,
        );
    }
}

fn align_at(aligns: Option<&Value>, idx: usize) -> Option<&str> {
    aligns.and_then(|a| {
        a.as_str().or_else(|| {
            a.as_array()
                .and_then(|arr| arr.get(idx))
                .and_then(Value::as_str)
        })
    })
}

fn check_plotly_table_columns(
    ctx: &FigureCtx<'_>,
    trace: &Value,
    headers: &[String],
    columns: &[Vec<&Value>],
    row_count: usize,
    findings: &mut Vec<Finding>,
) {
    let aligns = trace.pointer("/cells/align");
    let header_aligns = trace.pointer("/header/align");
    let has_format = trace.pointer("/cells/format").is_some_and(|v| !v.is_null());
    let mut first_text_col: Option<Vec<String>> = None;
    let mut first_num_col: Option<Vec<f64>> = None;

    for (idx, col) in columns.iter().enumerate() {
        let label = headers
            .get(idx)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("column {idx}"));
        let nums: Vec<f64> = col.iter().filter_map(|v| v.as_f64()).collect();
        let non_null = col.iter().filter(|v| !v.is_null()).count();
        let is_numeric = non_null > 0 && nums.len() * 2 >= non_null;
        let col_align = align_at(aligns, idx);
        let hdr_align = align_at(header_aligns, idx);

        if is_numeric {
            if first_num_col.is_none() && nums.len() == row_count {
                first_num_col = Some(nums.clone());
            }
            if !has_format {
                check_numeric_precision(ctx, "table-unfmt", &label, &nums, findings);
            }
            if col_align != Some("right") {
                ctx.push(
                    findings,
                    Kind::TableNumberAlignment,
                    &format!("table-align-{label}"),
                    &format!(
                        "Numeric column `{label}` is not right-aligned (VDQI p. 178). Set `cells.align='right'`."
                    ),
                    2,
                );
            }
            if hdr_align != Some("right") {
                ctx.push(
                    findings,
                    Kind::TableNumberAlignment,
                    &format!("table-header-align-{label}"),
                    &format!(
                        "Numeric column header `{label}` is not right-aligned to match its cells (VDQI p. 178). Set `header.align` to match `cells.align`."
                    ),
                    2,
                );
            }
            if mixes_zero_and_missing(col) {
                ctx.push(
                    findings,
                    Kind::TableMissingZeroAmbiguity,
                    &format!("table-missing-zero-{label}"),
                    &format!(
                        "Column `{label}` mixes exact zeros and missing values without explicit missing notation (VDQI p. 145)."
                    ),
                    2,
                );
            }
        } else {
            if let Some(cell_align) = col_align
                && hdr_align != Some(cell_align)
            {
                ctx.push(
                    findings,
                    Kind::TableNumberAlignment,
                    &format!("table-header-align-{label}"),
                    &format!(
                        "Table header `{label}` alignment does not match its column cells (VDQI p. 178). Set `header.align` to match `cells.align`."
                    ),
                    2,
                );
            }
            if first_text_col.is_none() {
                let texts: Vec<String> = col
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect();
                if texts.len() == row_count {
                    first_text_col = Some(texts);
                }
            }
        }
    }
    if let (Some(labels), Some(nums)) = (first_text_col, first_num_col)
        && is_alpha_with_unsorted_values(&labels, &nums)
    {
        ctx.push(
            findings,
            Kind::TableAlphabeticalOrder,
            "table-alpha-order",
            "Table rows are ordered alphabetically while numeric values are unsorted (VDQI p. 178). Order rows by content or value.",
            2,
        );
    }
}

fn check_numeric_precision(
    ctx: &FigureCtx<'_>,
    prefix: &str,
    label: &str,
    nums: &[f64],
    findings: &mut Vec<Finding>,
) {
    let decimals: BTreeSet<usize> = nums.iter().map(|v| decimal_digits(*v)).collect();
    let max_dec = decimals.iter().copied().max().unwrap_or(0);
    if max_dec > 4 || (decimals.len() >= 3 && max_dec >= 2) {
        ctx.push(
            findings,
            Kind::TableUnformattedNumbers,
            &format!("{prefix}-{label}"),
            &format!(
                "Numeric column `{label}` shows unformatted or inconsistent decimals (up to {max_dec} places, VDQI p. 178)."
            ),
            2,
        );
    }
}

/// Analyzes an AG Grid `{ columnDefs, rowData, defaultColDef, dashGridOptions/gridOptions }` spec.
fn analyze_ag_grid(ctx: &FigureCtx<'_>, spec: &Value, findings: &mut Vec<Finding>) -> Value {
    let col_defs: Vec<&Value> = spec
        .get("columnDefs")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().collect())
        .unwrap_or_default();
    let rows: Vec<&Value> = spec
        .get("rowData")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().collect())
        .unwrap_or_default();
    let default_col = spec.get("defaultColDef").unwrap_or(&Value::Null);
    let grid_opts = spec
        .get("dashGridOptions")
        .or_else(|| spec.get("gridOptions"))
        .unwrap_or(&Value::Null);

    let has_row_group = check_ag_grid_chrome(ctx, spec, grid_opts, &col_defs, rows.len(), findings);
    let headers = check_ag_grid_columns(ctx, &col_defs, default_col, &rows, findings);

    json!({
        "table-structure": {
            "rows": rows.len(),
            "columns": col_defs.len(),
            "grouped": has_row_group,
            "headers": headers,
        }
    })
}

fn check_ag_grid_chrome(
    ctx: &FigureCtx<'_>,
    spec: &Value,
    grid_opts: &Value,
    col_defs: &[&Value],
    row_count: usize,
    findings: &mut Vec<Finding>,
) -> bool {
    let has_row_group = col_defs
        .iter()
        .any(|c| c.get("rowGroup").and_then(Value::as_bool) == Some(true));
    if row_count > UNGROUPED_TABLE_ROWS && !has_row_group {
        ctx.push(
            findings,
            Kind::TableUngroupedRows,
            "ag-ungrouped",
            &format!(
                "AG Grid displays {row_count} rows without topical `rowGroup` sections (VDQI p. 179). Group dense tables into clusters of 3 to 7 rows."
            ),
            2,
        );
    }
    if grid_opts.get("columnBorder").and_then(Value::as_bool) == Some(true)
        || spec.pointer("/theme/columnBorder").and_then(Value::as_bool) == Some(true)
    {
        ctx.push(
            findings,
            Kind::TableVerticalRules,
            "ag-column-border",
            "AG Grid enables vertical `columnBorder` lines (VDQI p. 179). Remove vertical gridlines.",
            2,
        );
    }
    let has_zebra = grid_opts
        .get("oddRowBackgroundColor")
        .and_then(Value::as_str)
        .is_some_and(|c| !c.is_empty() && c != "transparent")
        || spec
            .pointer("/theme/oddRowBackgroundColor")
            .and_then(Value::as_str)
            .is_some_and(|c| !c.is_empty() && c != "transparent");
    if has_zebra {
        ctx.push(
            findings,
            Kind::TableZebraStriping,
            "ag-zebra",
            "AG Grid sets `oddRowBackgroundColor` zebra striping (VDQI p. 179). Use plain backgrounds with horizontal topical dividers.",
            2,
        );
    }
    has_row_group
}

fn check_ag_grid_columns(
    ctx: &FigureCtx<'_>,
    col_defs: &[&Value],
    default_col: &Value,
    rows: &[&Value],
    findings: &mut Vec<Finding>,
) -> Vec<String> {
    let mut headers = Vec::new();
    let mut first_text_col: Option<Vec<String>> = None;
    let mut first_num_col: Option<Vec<f64>> = None;

    for col in col_defs {
        let field = col.get("field").and_then(Value::as_str);
        let header = col.get("headerName").and_then(Value::as_str);
        let label = header.or(field).unwrap_or("column").to_owned();
        headers.push(label.clone());

        if let Some(hdr) = header {
            if is_all_caps(hdr) || looks_like_identifier(hdr) {
                ctx.push(
                    findings,
                    Kind::TableRawHeader,
                    &format!("ag-header-{hdr}"),
                    &format!("AG Grid header `{hdr}` is a raw identifier or all-caps label (VDQI p. 183)."),
                    2,
                );
            }
        } else if let Some(fld) = field
            && looks_like_identifier(fld)
        {
            ctx.push(
                findings,
                Kind::TableRawHeader,
                &format!("ag-field-{fld}"),
                &format!(
                    "AG Grid column `field: '{fld}'` has no spelled-out `headerName` (VDQI p. 183)."
                ),
                2,
            );
        }

        let Some(fld) = field else {
            continue;
        };
        let col_values: Vec<&Value> = rows
            .iter()
            .map(|row| row.get(fld).unwrap_or(&Value::Null))
            .collect();
        let nums: Vec<f64> = col_values.iter().filter_map(|v| v.as_f64()).collect();
        let non_null = col_values.iter().filter(|v| !v.is_null()).count();
        let is_numeric = (non_null > 0 && nums.len() * 2 >= non_null)
            || col.get("cellDataType").and_then(Value::as_str) == Some("number");
        let has_formatter = col.get("valueFormatter").is_some_and(|v| !v.is_null())
            || default_col
                .get("valueFormatter")
                .is_some_and(|v| !v.is_null());

        if is_numeric {
            if first_num_col.is_none() && nums.len() == rows.len() {
                first_num_col = Some(nums.clone());
            }
            let right_aligned = col_type_is_right(col)
                || col_type_is_right(default_col)
                || col.pointer("/cellStyle/textAlign").and_then(Value::as_str) == Some("right");
            if !right_aligned {
                ctx.push(
                    findings,
                    Kind::TableNumberAlignment,
                    &format!("ag-align-{label}"),
                    &format!(
                        "AG Grid numeric column `{label}` is not right-aligned (VDQI p. 178). Set `type: 'numericColumn'` or `'rightAligned'`."
                    ),
                    2,
                );
            }
            if !has_formatter {
                check_numeric_precision(ctx, "ag-unfmt", &label, &nums, findings);
                if mixes_zero_and_missing(&col_values) {
                    ctx.push(
                        findings,
                        Kind::TableMissingZeroAmbiguity,
                        &format!("ag-missing-zero-{label}"),
                        &format!(
                            "AG Grid column `{label}` mixes exact zeros and missing values with no `valueFormatter` to distinguish them (VDQI p. 145)."
                        ),
                        2,
                    );
                }
            }
        } else if first_text_col.is_none() {
            let texts: Vec<String> = col_values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            if texts.len() == rows.len() {
                first_text_col = Some(texts);
            }
        }
    }

    if let (Some(labels), Some(nums)) = (first_text_col, first_num_col)
        && is_alpha_with_unsorted_values(&labels, &nums)
    {
        ctx.push(
            findings,
            Kind::TableAlphabeticalOrder,
            "ag-alpha-order",
            "AG Grid rows are ordered alphabetically while numeric values are unsorted (VDQI p. 178). Sort by content or value.",
            2,
        );
    }
    headers
}

fn col_type_is_right(col: &Value) -> bool {
    match col.get("type") {
        Some(Value::String(s)) => s.contains("numericColumn") || s.contains("rightAligned"),
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(Value::as_str)
            .any(|s| s == "numericColumn" || s == "rightAligned"),
        _ => false,
    }
}

/// Analyzes raw Matplotlib/Seaborn measurements extracted by `py/figure_facts.py`.
fn analyze_matplotlib(ctx: &FigureCtx<'_>, raw: &Value, findings: &mut Vec<Finding>) -> Value {
    let marks = raw
        .get("marks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total_values = check_mpl_marks_and_canvas(ctx, raw, &marks, findings);
    let any_span = check_mpl_colors_and_axes(ctx, raw, &marks, findings);
    check_mpl_texts(ctx, raw, any_span, findings);

    let mut summary = BTreeMap::new();
    summary.insert("canvas", raw.get("canvas").cloned().unwrap_or(Value::Null));
    summary.insert("values", json!({"total": total_values, "marks": marks}));
    summary.insert("text", raw.get("texts").cloned().unwrap_or(Value::Null));
    json!(summary)
}

fn check_mpl_marks_and_canvas(
    ctx: &FigureCtx<'_>,
    raw: &Value,
    marks: &[Value],
    findings: &mut Vec<Finding>,
) -> usize {
    let width = raw.pointer("/canvas/width").and_then(Value::as_f64);
    let height = raw.pointer("/canvas/height").and_then(Value::as_f64);
    let panels = raw
        .pointer("/canvas/panels")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if let (Some(w), Some(h)) = (width, height)
        && panels == 1
        && h > w
    {
        ctx.push(
            findings,
            Kind::VerticalAspectRatio,
            "mpl-vertical",
            &format!("Single-panel canvas {w:.0} by {h:.0} px is taller than wide (VDQI p. 186)."),
            2,
        );
    }
    let total_values: usize = marks
        .iter()
        .filter_map(|m| m.get("values").and_then(Value::as_u64))
        .filter_map(|v| usize::try_from(v).ok())
        .sum();
    if total_values > 0 && total_values <= SMALL_DATA {
        ctx.push(
            findings,
            Kind::SmallDataChart,
            "mpl-small-data",
            &format!(
                "Figure draws only {total_values} values ({SMALL_DATA} or fewer), which a table or sentence shows better (VDQI p. 33)."
            ),
            2,
        );
    }
    let pie_slices: usize = marks
        .iter()
        .filter(|m| m.get("mark").and_then(Value::as_str) == Some("pie"))
        .filter_map(|m| m.get("values").and_then(Value::as_u64))
        .filter_map(|v| usize::try_from(v).ok())
        .sum();
    if pie_slices > 0 {
        ctx.push(
            findings,
            Kind::PieChart,
            "mpl-pie",
            &format!("Pie draws {pie_slices} slices (VDQI p. 178)."),
            3,
        );
    }
    total_values
}

fn check_mpl_colors_and_axes(
    ctx: &FigureCtx<'_>,
    raw: &Value,
    marks: &[Value],
    findings: &mut Vec<Finding>,
) -> Option<i32> {
    let styles = raw
        .get("styles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let annotations_nonempty = raw
        .pointer("/texts/annotations")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty());
    for (idx, first) in styles.iter().enumerate() {
        for second in &styles[idx + 1..] {
            let same = first.get("dash") == second.get("dash")
                && first.get("markers") == second.get("markers");
            let pair = (
                red_or_green(first.get("color").and_then(Value::as_str)),
                red_or_green(second.get("color").and_then(Value::as_str)),
            );
            if matches!(
                pair,
                (Some("red"), Some("green")) | (Some("green"), Some("red"))
            ) && same
                && !annotations_nonempty
            {
                let a = first.get("trace").and_then(Value::as_str).unwrap_or("a");
                let b = second.get("trace").and_then(Value::as_str).unwrap_or("b");
                ctx.push(
                    findings,
                    Kind::RedGreenOnly,
                    &format!("mpl-rg-{a}-{b}"),
                    &format!(
                        "`{a}` and `{b}` differ only by red against green with no direct labels (VDQI p. 183)."
                    ),
                    3,
                );
            }
        }
    }
    if raw.get("has_twin").and_then(Value::as_bool) == Some(true) {
        ctx.push(
            findings,
            Kind::DualAxis,
            "mpl-twin",
            "Figure overlays twin axes on the same panel (VDQI p. 61).",
            3,
        );
    }
    let has_bar = marks
        .iter()
        .any(|m| matches!(m.get("mark").and_then(Value::as_str), Some("bar" | "area")));
    if has_bar
        && let Some(ranges) = raw.get("y_ranges").and_then(Value::as_array)
        && ranges.iter().any(|r| {
            r.as_array()
                .is_some_and(|pair| pair.len() == 2 && pair[0].as_f64().is_some_and(|lo| lo > 0.0))
        })
    {
        ctx.push(
            findings,
            Kind::NonZeroBaseline,
            "mpl-nonzero",
            "Bar or area y-axis starts above zero (VDQI p. 54).",
            3,
        );
    }
    let mut any_span: Option<i32> = None;
    if let Some(xs_list) = raw.get("xs").and_then(Value::as_array) {
        for trace in xs_list {
            let tname = trace
                .get("trace")
                .and_then(Value::as_str)
                .unwrap_or("trace");
            if let Some(xs) = trace.get("x").and_then(Value::as_array) {
                let years: BTreeSet<i32> = xs.iter().filter_map(year_of).collect();
                if xs.len() == 2 {
                    ctx.push(
                        findings,
                        Kind::MissingContext,
                        &format!("mpl-two-points-{tname}"),
                        &format!(
                            "`{tname}` has only two x values with no baseline context (VDQI p. 74)."
                        ),
                        2,
                    );
                } else if years.len() > 2
                    && let (Some(first), Some(last)) = (years.first(), years.last())
                {
                    let span = last - first + 1;
                    any_span = Some(any_span.map_or(span, |p| p.max(span)));
                }
            }
        }
    }
    any_span
}

fn check_mpl_texts(
    ctx: &FigureCtx<'_>,
    raw: &Value,
    any_span: Option<i32>,
    findings: &mut Vec<Finding>,
) {
    let title = raw
        .pointer("/texts/title")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !title.is_empty() && is_all_caps(title) {
        ctx.push(
            findings,
            Kind::AllCapsText,
            "mpl-caps",
            &format!("Title `{title}` is written in all capitals (VDQI p. 183)."),
            2,
        );
    }
    if let Some(angles) = raw.pointer("/texts/tick_angles").and_then(Value::as_array)
        && let Some(angle) = angles
            .iter()
            .filter_map(Value::as_f64)
            .find(|a| a.abs() > 0.0 && (a.abs() % 180.0) > 0.0)
    {
        ctx.push(
            findings,
            Kind::RotatedTicks,
            "mpl-rotated",
            &format!("Tick labels are rotated by {angle:.0} degrees (VDQI p. 183)."),
            2,
        );
    }
    let axis_titles: Vec<&str> = raw
        .pointer("/texts/axis_titles")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let words = format!("{title} {}", axis_titles.join(" "));
    if let Some(matched) = MONEY_RE.find(&words)
        && !BASIS_RE.is_match(&words)
    {
        let span_note = any_span
            .map(|span| format!(" over {span} years"))
            .unwrap_or_default();
        ctx.push(
            findings,
            Kind::UndeflatedMoney,
            "mpl-money",
            &format!(
                "Figure shows money (`{}`){span_note} without nominal, real, or constant basis (VDQI p. 68).",
                matched.as_str()
            ),
            2,
        );
    }
}
