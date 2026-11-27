//! `xpd chart-review`: the Duroxide ledger of Edward Tufte's "The Visual Display of Quantitative
//! Information" across charts and presentation tables in Python (Plotly, `go.Table`, Dash AG Grid,
//! Matplotlib, Seaborn, pandas) and React/TypeScript (`react-plotly.js`, `plotly.js`,
//! `ag-grid-react`, `ag-grid-community`).
//!
//! Mechanical findings (static AST rules plus optional `--adapter` runtime figure measurements)
//! gate objectively on removal, while remaining per-figure judgment questions form a durable
//! checklist that must be fixed or answered with a reason before `finish`.

mod inventory;
mod python;
pub(crate) mod runtime;
mod tsx;

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use ast_grep_language::SupportLang;
use clap::ValueEnum;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};

use crate::catalog::Filters;
use crate::ledger::gix_source::Listing;
use crate::ledger::{self, AdapterPolicy, ChecklistEntry, Ledger, Select, SurveyOutput, Tree};
use crate::output::Format;
use crate::style::Style;
use crate::syntax;
use crate::workspace::Workspace;

pub(crate) use inventory::{BOOK, Inventory, load as inventory};

pub(crate) type Finding = ledger::Finding<Kind>;

/// The five Tufte analytical pillars that group `xpd chart-review` findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Group {
    GraphicalIntegrity,
    DataInkAndChartjunk,
    EncodingAndForm,
    WordsAndLayout,
    TabularPresentation,
}

/// Mechanical Tufte finding kinds across charts and presentation tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Kind {
    /// Bar or area baseline starts above zero (VDQI p. 54).
    NonZeroBaseline,
    /// Two different value scales share one chart panel (VDQI p. 61).
    DualAxis,
    /// Chart depicts more varying dimensions than the data have (VDQI p. 71).
    ExtraDimensions,
    /// Mark area or radius distorts the underlying numerical ratio (VDQI p. 57).
    DistortedSize,
    /// Money shown over time without stating nominal, real, or constant units (VDQI p. 68).
    UndeflatedMoney,
    /// Two isolated points shown without surrounding trend or baseline context (VDQI p. 74).
    MissingContext,
    /// Default chart theme keeps heavy background tint and software chrome (VDQI p. 96).
    DefaultTemplate,
    /// Full box frame or mirrored spines enclose the plotting field without carrying data (VDQI p. 96).
    BoxedFrame,
    /// Explicit or dark grid competes with the data marks (VDQI p. 112).
    HeavyGrid,
    /// Color repeats a variable already encoded by position (VDQI p. 100).
    RedundantColor,
    /// Hatching or pattern fills produce moire vibration (VDQI p. 108).
    PatternFill,
    /// Decorative gauge or duck replaces plain quantitative marks (VDQI p. 116).
    DecorativeGauge,
    /// Standard box plot adds excess frame ink where a quartile plot fits (VDQI p. 124).
    HeavyBoxPlot,
    /// Heavy line strokes overwhelm the data (VDQI p. 185).
    ThickLine,
    /// Pie, donut, or sunburst forces angle comparison instead of a common scale (VDQI p. 178).
    PieChart,
    /// Twenty or fewer values drawn as a chart where a table or sentence works better (VDQI p. 33).
    SmallDataChart,
    /// Animation hides comparison across frames where small multiples belong (VDQI p. 170).
    AnimationInsteadOfMultiples,
    /// Facet panels use independent scales that break visual comparison (VDQI p. 170).
    FreeFacetScales,
    /// Unordered rainbow colormap distorts quantitative order (VDQI p. 154).
    RainbowColorscale,
    /// Series differ only by red against green with no secondary channel (VDQI p. 183).
    RedGreenOnly,
    /// Spline or heatmap smoothing invents values not in the data (VDQI p. 13).
    SmoothedData,
    /// More axis ticks are drawn than data points (VDQI p. 120).
    ExcessTicks,
    /// Categories are ordered alphabetically while values are unsorted (VDQI p. 178).
    AlphabeticalOrder,
    /// Exact zeros and missing values are mixed without visual distinction (VDQI p. 145).
    ZeroMissingAmbiguity,
    /// Legend forces eye travel where direct series labels belong (VDQI p. 183).
    LegendInsteadOfLabels,
    /// Axis or series label shows a raw column name instead of spelled-out words (VDQI p. 183).
    ColumnNameLabel,
    /// Tick labels are rotated instead of running left to right (VDQI p. 183).
    RotatedTicks,
    /// Titles or labels are set in all capitals (VDQI p. 183).
    AllCapsText,
    /// Single-panel chart is taller than wide (VDQI p. 186).
    VerticalAspectRatio,
    /// Line slope banking sits far from 45 degrees (VDQI p. 188).
    PoorBanking,
    /// Table draws vertical gridlines or heavy cell cages (VDQI p. 179).
    TableVerticalRules,
    /// Table uses alternating zebra row fills instead of subtle horizontal dividers (VDQI p. 179).
    TableZebraStriping,
    /// Table rows are ordered alphabetically while numeric columns are unsorted (VDQI p. 178).
    TableAlphabeticalOrder,
    /// Dense table exceeds 12 flat rows with no topical grouping (VDQI p. 179).
    TableUngroupedRows,
    /// Numeric table column renders raw or inconsistently rounded floats (VDQI p. 178).
    TableUnformattedNumbers,
    /// Numeric table column is not right-aligned (VDQI p. 178).
    TableNumberAlignment,
    /// Table header displays a raw field identifier or all-caps text (VDQI p. 183).
    TableRawHeader,
    /// Table numeric column mixes exact zeros and missing values without distinct formatting (VDQI p. 145).
    TableMissingZeroAmbiguity,
}

impl ledger::Kind for Kind {
    type Group = Group;

    const COMMAND: &'static str = "chart-review";

    const GROUPS: &'static [Group] = &[
        Group::GraphicalIntegrity,
        Group::DataInkAndChartjunk,
        Group::EncodingAndForm,
        Group::WordsAndLayout,
        Group::TabularPresentation,
    ];

    fn group(self) -> Group {
        match self {
            Self::NonZeroBaseline
            | Self::DualAxis
            | Self::ExtraDimensions
            | Self::DistortedSize
            | Self::UndeflatedMoney
            | Self::MissingContext => Group::GraphicalIntegrity,
            Self::DefaultTemplate
            | Self::BoxedFrame
            | Self::HeavyGrid
            | Self::RedundantColor
            | Self::PatternFill
            | Self::DecorativeGauge
            | Self::HeavyBoxPlot
            | Self::ThickLine => Group::DataInkAndChartjunk,
            Self::PieChart
            | Self::SmallDataChart
            | Self::AnimationInsteadOfMultiples
            | Self::FreeFacetScales
            | Self::RainbowColorscale
            | Self::RedGreenOnly
            | Self::SmoothedData
            | Self::ExcessTicks
            | Self::AlphabeticalOrder
            | Self::ZeroMissingAmbiguity => Group::EncodingAndForm,
            Self::LegendInsteadOfLabels
            | Self::ColumnNameLabel
            | Self::RotatedTicks
            | Self::AllCapsText
            | Self::VerticalAspectRatio
            | Self::PoorBanking => Group::WordsAndLayout,
            Self::TableVerticalRules
            | Self::TableZebraStriping
            | Self::TableAlphabeticalOrder
            | Self::TableUngroupedRows
            | Self::TableUnformattedNumbers
            | Self::TableNumberAlignment
            | Self::TableRawHeader
            | Self::TableMissingZeroAmbiguity => Group::TabularPresentation,
        }
    }

    fn heading(group: Group) -> &'static str {
        match group {
            Group::GraphicalIntegrity => {
                "Graphical integrity: distortions of scale, baseline, dimensions, money, or context (VDQI Ch. 2 to 3)"
            }
            Group::DataInkAndChartjunk => {
                "Data-ink and chartjunk: non-data frames, grids, patterns, and software chrome (VDQI Ch. 4 to 6)"
            }
            Group::EncodingAndForm => {
                "Encoding and form: weak chart types, unordered palettes, smoothing, and ordering (VDQI Ch. 1, 7, 8)"
            }
            Group::WordsAndLayout => {
                "Words and layout: legends, labels, typography, aspect ratio, and slope banking (VDQI Ch. 1, 7, 9)"
            }
            Group::TabularPresentation => {
                "Tabular presentation: borders, striping, alignment, formatting, grouping, and headers (VDQI pp. 33, 178 to 183)"
            }
        }
    }
}

impl Kind {
    /// Maps each rule in `rules/plotly/*.yml` onto its corresponding ledger finding kind.
    pub(crate) fn of_plotly_rule(rule: &str) -> Option<Self> {
        Some(match rule {
            "bar-axis-without-zero" | "area-axis-without-zero" => Self::NonZeroBaseline,
            "dual-y-axis" => Self::DualAxis,
            "three-d-for-two-d" => Self::ExtraDimensions,
            "marker-size-diameter" | "polar-bar-area" => Self::DistortedSize,
            "default-template" => Self::DefaultTemplate,
            "boxed-axis-frame" => Self::BoxedFrame,
            "explicit-grid" => Self::HeavyGrid,
            "redundant-color" => Self::RedundantColor,
            "pattern-fill" => Self::PatternFill,
            "gauge-indicator" => Self::DecorativeGauge,
            "box-plot-ink" => Self::HeavyBoxPlot,
            "pie-chart" | "sunburst-as-pie" => Self::PieChart,
            "table-for-small-data" => Self::SmallDataChart,
            "animation-frame" => Self::AnimationInsteadOfMultiples,
            "free-facet-scales" => Self::FreeFacetScales,
            "rainbow-colorscale" => Self::RainbowColorscale,
            "spline-line" | "heatmap-smoothing" => Self::SmoothedData,
            "legend-instead-of-labels" => Self::LegendInsteadOfLabels,
            "axis-title-column-name" => Self::ColumnNameLabel,
            "rotated-tick-labels" => Self::RotatedTicks,
            "all-caps-title" => Self::AllCapsText,
            _ => return None,
        })
    }
}

/// One detected chart or presentation table in a surveyed file.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Figure {
    pub(crate) id: String,
    pub(crate) file: String,
    #[serde(skip)]
    pub(crate) tracked: String,
    pub(crate) line: usize,
    pub(crate) symbol: String,
    pub(crate) call: String,
    pub(crate) kind: &'static str,
    pub(crate) library: &'static str,
    pub(crate) route: &'static str,
}

/// What a `chart-review` survey read.
#[derive(Debug, Serialize)]
struct Read {
    files: usize,
    python: usize,
    typescript: usize,
    book: &'static str,
    libraries: Vec<&'static str>,
    figures: Vec<Figure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    facts: Option<Value>,
}

/// Review charts and presentation tables against Edward Tufte's "The Visual Display of
/// Quantitative Information" as a Duroxide ledger.
///
/// Inspects Python (Plotly, `go.Table`, Dash AG Grid, Matplotlib, Seaborn, pandas) and
/// React/TypeScript (`react-plotly.js`, `plotly.js`, `ag-grid-react`, `ag-grid-community`).
/// Mechanical findings (static rules plus optional `--adapter` runtime measurements) carry
/// deterministic IDs and gate objectively on removal, while remaining per-figure judgment
/// questions form a checklist that must be fixed or answered with a reason before `finish`.
#[derive(Debug, clap::Args)]
#[command(after_long_help = "Examples:
  xpd chart-review src/
  xpd chart-review report.py --adapter figures.py --format json
  xpd chart-review start src/ --check \"pytest\" --adapter figures.py
  xpd chart-review --questions --format json")]
pub(crate) struct ChartReviewArgs {
    #[command(flatten)]
    select: Select<Kind>,
    /// A Python adapter file or shell command that emits rendered figure specs for runtime checks.
    #[arg(long, value_name = "ADAPTER")]
    adapter: Option<String>,
    /// Print the whole inventory of Tufte questions and runtime facts, with no file scan.
    #[arg(long)]
    questions: bool,
}

impl ChartReviewArgs {
    pub(crate) fn run(self, default: Format) -> Result<ExitCode> {
        if self.questions {
            let inv = inventory()?;
            let catalog = json!({
                "book": BOOK,
                "facts": inv.facts,
                "questions": inv.questions.iter().map(inventory::Question::to_catalog_json).collect::<Vec<_>>(),
            });
            match self.select.format_override().unwrap_or(default) {
                Format::Json => println!("{catalog}"),
                Format::Text => print!("{}", catalog_text(&inv)),
            }
            return Ok(ExitCode::SUCCESS);
        }

        let header = |ledger: &Ledger<Kind, Read>| {
            let Read {
                files,
                python,
                typescript,
                libraries,
                figures,
                facts,
                ..
            } = &ledger.read;
            let mut out = String::new();
            let file_word = if *files == 1 { "file" } else { "files" };
            let fig_word = if figures.len() == 1 {
                "figure"
            } else {
                "figures"
            };
            let item_word = if ledger.checklist.len() == 1 {
                "item"
            } else {
                "items"
            };
            let _ = writeln!(
                out,
                "xpd chart-review: {} in {files} {file_word} ({python} Python, {typescript} JS/TS), {} {fig_word}, {} checklist {item_word}",
                ledger.count(),
                figures.len(),
                ledger.checklist.len(),
            );
            if !libraries.is_empty() {
                let _ = writeln!(out, "Libraries: {}", libraries.join(", "));
            }
            if !figures.is_empty() {
                out.push_str("Figures:\n");
                for fig in figures {
                    let _ = writeln!(
                        out,
                        "  {}  {}:{}  {} ({} in {}, route: {})",
                        fig.id, fig.file, fig.line, fig.call, fig.kind, fig.library, fig.route
                    );
                }
            }
            if let Some(facts) = facts {
                if let Some(error) = facts.get("error").and_then(Value::as_str) {
                    let _ = writeln!(out, "Adapter error: {error}");
                } else if let Some(measured) = facts.get("figures").and_then(Value::as_array) {
                    let names: Vec<&str> = measured
                        .iter()
                        .filter_map(|f| f.get("name").and_then(Value::as_str))
                        .collect();
                    let measured_word = if names.len() == 1 {
                        "figure"
                    } else {
                        "figures"
                    };
                    let _ = writeln!(
                        out,
                        "Adapter `{}` measured {} {measured_word}: {}",
                        facts.get("adapter").and_then(Value::as_str).unwrap_or(""),
                        names.len(),
                        names.join(", ")
                    );
                }
            }
            out
        };

        let adapter = self.adapter;
        (self.select).print_with(
            default,
            adapter.as_deref(),
            |paths, files, top, commit, adapter, policy| {
                survey(
                    paths,
                    files,
                    &Listing::of(paths, files, top, commit)?,
                    adapter,
                    policy,
                )
            },
            header,
        )
    }
}

/// Formats `xpd chart-review --questions --format text`.
fn catalog_text(inv: &Inventory) -> String {
    let mut out = String::new();
    out.push_str("Facts that --adapter measures:\n");
    for fact in &inv.facts {
        let _ = writeln!(out, "  {}: {}", fact.id, fact.text);
    }
    let _ = writeln!(
        out,
        "\nQuestions from {BOOK}, {} total:",
        inv.questions.len()
    );
    for q in &inv.questions {
        let _ = writeln!(
            out,
            "\n{} (page {}, {:?}): {}\n  {}\n  Look at: {}\n  In plotly/AG Grid: {}",
            q.id, q.page, q.mode, q.principle, q.text, q.look, q.plotly
        );
    }
    out
}

/// Whether `lang` is a JavaScript or TypeScript language parsed by `tsx`.
fn is_js_ts(lang: SupportLang) -> bool {
    matches!(
        lang,
        SupportLang::TypeScript | SupportLang::Tsx | SupportLang::JavaScript
    )
}

/// Opens a plotly-only rule workspace for Python files.
fn plotly_workspace(paths: &[PathBuf], tree: Tree<'_>) -> Result<Workspace> {
    let root = tree
        .top
        .map(Path::to_path_buf)
        .or_else(|| {
            let first = paths.first()?;
            let absolute = fs::canonicalize(first).ok()?;
            Some(if absolute.is_dir() {
                absolute
            } else {
                absolute.parent().unwrap_or(&absolute).to_path_buf()
            })
        })
        .unwrap_or_else(|| PathBuf::from("."));
    let filters = Filters {
        rules: vec!["plotly".to_owned()],
        severity: None,
    };
    Workspace::open(root, None, &filters).or_else(|_| Workspace::bundled(Style::Default))
}

/// Per-file static extraction result collected across parallel workers.
struct FileResult {
    is_python: bool,
    is_js_ts: bool,
    libraries: Vec<&'static str>,
    figures: Vec<Figure>,
    findings: Vec<Finding>,
}

/// Surveys `paths` for static figures, mechanical findings, optional runtime measurements, and
/// the per-figure Tufte checklist.
fn survey(
    paths: &[PathBuf],
    tree: Tree<'_>,
    listing: &Listing,
    adapter: Option<&str>,
    policy: AdapterPolicy,
) -> Result<SurveyOutput<Kind, Read>> {
    let inv = inventory()?;
    let files = &listing.files;
    let workspace = files
        .iter()
        .any(|(_, lang)| *lang == SupportLang::Python)
        .then(|| plotly_workspace(paths, tree))
        .transpose()?;

    let per_file: Vec<FileResult> = (files.par_iter())
        .filter_map(|(path, language)| {
            let is_py = *language == SupportLang::Python;
            let is_ts = is_js_ts(*language);
            if !is_py && !is_ts {
                return None;
            }
            let text = listing.text(path)?;
            let absolute = listing.absolute(path)?;
            let shown = tree.shown(path);
            let tracked = tree.tracked(&absolute).unwrap_or_else(|| shown.clone());
            let root = syntax::parse_as(&text, *language);
            if is_py {
                let surveyed =
                    python::survey_file(path, &shown, &tracked, &text, &root, workspace.as_ref());
                Some(FileResult {
                    is_python: true,
                    is_js_ts: false,
                    libraries: surveyed.libraries,
                    figures: surveyed.figures,
                    findings: surveyed.findings,
                })
            } else {
                let surveyed = tsx::survey_file(&shown, &tracked, &root);
                Some(FileResult {
                    is_python: false,
                    is_js_ts: true,
                    libraries: surveyed.libraries,
                    figures: surveyed.figures,
                    findings: surveyed.findings,
                })
            }
        })
        .collect();

    let mut libraries: BTreeSet<&'static str> = BTreeSet::new();
    let mut figures = Vec::new();
    let mut findings = Vec::new();
    let mut python_count = 0usize;
    let mut ts_count = 0usize;

    for result in per_file {
        if result.is_python {
            python_count += 1;
        }
        if result.is_js_ts {
            ts_count += 1;
        }
        libraries.extend(result.libraries);
        figures.extend(result.figures);
        findings.extend(result.findings);
    }

    let runtime_result = match adapter {
        Some(spec) => {
            let work_dir = tree
                .top
                .filter(|_| tree.shown_from.is_some())
                .map(Path::to_path_buf)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            Some(runtime::measure(spec, &work_dir, policy)?)
        }
        None => None,
    };

    let coverage = if runtime_result
        .as_ref()
        .is_some_and(|res| res.summary.get("error").is_none())
    {
        inventory::RuntimeCoverage::WithRuntime
    } else {
        inventory::RuntimeCoverage::StaticOnly
    };
    let facts = runtime_result.map(|res| {
        findings.extend(res.findings);
        res.summary
    });
    let checklist = build_checklist(&inv, &figures, coverage);

    let read = Read {
        files: python_count + ts_count,
        python: python_count,
        typescript: ts_count,
        book: BOOK,
        libraries: libraries.into_iter().collect(),
        figures,
        facts,
    };

    Ok(SurveyOutput {
        read,
        findings: ledger::named(findings),
        checklist,
    })
}

/// Builds the per-figure Tufte checklist from the inventory and detected figures.
fn build_checklist(
    inv: &Inventory,
    figures: &[Figure],
    coverage: inventory::RuntimeCoverage,
) -> Vec<ChecklistEntry> {
    let mut checklist = Vec::new();
    for fig in figures {
        for q in &inv.questions {
            if q.applies_to(fig.kind) && q.needs_checklist(coverage) {
                checklist.push(ChecklistEntry {
                    id: format!("{}/{}", fig.id, q.id),
                    file: fig.tracked.clone(),
                    figure: format!(
                        "{}:{} ({} in {})",
                        fig.file, fig.line, fig.kind, fig.library
                    ),
                    kind: fig.kind.to_owned(),
                    question: q.id.clone(),
                    page: q.page,
                    principle: q.principle.clone(),
                    text: q.text.clone(),
                    look: q.look.clone(),
                });
            }
        }
    }
    checklist
}

/// Recommended Plotly or AG Grid target call for a figure kind.
pub(crate) fn route_for(kind: &str) -> &'static str {
    match kind {
        "table" => "go.Table, or AG Grid (dash_ag_grid.AgGrid / AgGridReact)",
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

/// Whether `text` contains at least two multi-letter alphabetic words all written in uppercase.
pub(crate) fn is_all_caps(text: &str) -> bool {
    let words: Vec<&str> = text
        .split(|ch: char| !ch.is_alphabetic())
        .filter(|word| word.chars().count() > 1)
        .collect();
    words.len() >= 2
        && words
            .iter()
            .all(|word| word.chars().all(char::is_uppercase))
}

/// Whether `text` looks like a raw `snake_case` or `camelCase` code identifier rather than prose.
pub(crate) fn looks_like_identifier(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return false;
    }
    if trimmed.contains('_') && trimmed.chars().any(char::is_alphabetic) {
        return true;
    }
    if trimmed.len() >= 5 {
        let chars: Vec<char> = trimmed.chars().collect();
        let has_hump = chars
            .windows(2)
            .any(|pair| pair[0].is_ascii_lowercase() && pair[1].is_ascii_uppercase());
        if has_hump && chars.iter().all(char::is_ascii_alphanumeric) {
            return true;
        }
    }
    false
}

/// Whether a colorscale or colormap name is an unordered rainbow spectrum.
pub(crate) fn is_rainbow_scale(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "rainbow" | "jet" | "hsv" | "spectral" | "gist_rainbow" | "nipy_spectral" | "turbo"
    )
}
