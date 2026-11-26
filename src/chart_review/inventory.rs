//! The embedded Tufte VDQI inventory (`rules/xpd/chart-review.yml`), shared by
//! `xpd chart-review` and `xpd rules --format json`.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The questions and facts written from the ledger of Edward Tufte's book.
const INVENTORY: &str = include_str!("../../rules/xpd/chart-review.yml");

/// The title of the book that the questions cite by page.
pub(crate) const BOOK: &str =
    "Edward Tufte, \"The Visual Display of Quantitative Information\", second edition";

/// The kind tag of a question that applies to every chart.
pub(crate) const ANY_CHART: &str = "any";

/// The kind tag of a presentation table figure.
pub(crate) const TABLE_KIND: &str = "table";

/// Whether runtime measurements ran alongside static checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeCoverage {
    StaticOnly,
    WithRuntime,
}

/// Whether a Tufte principle is checked mechanically, partly mechanically, or purely by judgment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CheckMode {
    Mechanical,
    Hybrid,
    #[default]
    Judgment,
}

/// One measured fact of a rendered figure or table.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Fact {
    pub(crate) id: String,
    pub(crate) text: String,
}

/// One review question, with the page and line of the book it comes from.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Question {
    pub(crate) id: String,
    pub(crate) page: u32,
    pub(crate) line: u32,
    pub(crate) chapter: String,
    pub(crate) principle: String,
    pub(crate) text: String,
    pub(crate) look: String,
    pub(crate) kinds: Vec<String>,
    pub(crate) facts: Vec<String>,
    pub(crate) plotly: String,
    pub(crate) rules: Vec<String>,
    pub(crate) covers: Vec<u32>,
    #[serde(default)]
    pub(crate) mode: CheckMode,
}

impl Question {
    /// Whether this question applies to a figure of `figure_kind`.
    pub(crate) fn applies_to(&self, figure_kind: &str) -> bool {
        let is_table = figure_kind == TABLE_KIND;
        self.kinds
            .iter()
            .any(|kind| kind == figure_kind || (!is_table && kind == ANY_CHART))
    }

    /// Whether this question enters the manual per-figure checklist when mechanical detectors ran.
    pub(crate) fn needs_checklist(&self, coverage: RuntimeCoverage) -> bool {
        match self.mode {
            CheckMode::Judgment | CheckMode::Hybrid => true,
            CheckMode::Mechanical => {
                coverage == RuntimeCoverage::StaticOnly
                    && self.rules.is_empty()
                    && !self.facts.is_empty()
            }
        }
    }

    /// JSON representation for `xpd chart-review --questions`.
    pub(crate) fn to_catalog_json(&self) -> Value {
        let rules: Vec<String> = self
            .rules
            .iter()
            .map(|rule| format!("plotly/{rule}"))
            .collect();
        json!({
            "id": self.id,
            "page": self.page,
            "line": self.line,
            "chapter": self.chapter,
            "principle": self.principle,
            "text": self.text,
            "look": self.look,
            "kinds": self.kinds,
            "plotly": self.plotly,
            "rules": rules,
            "facts": self.facts,
            "covers": self.covers,
            "mode": self.mode,
        })
    }
}

/// Every question and fact of the review.
#[derive(Debug, Deserialize)]
pub(crate) struct Inventory {
    pub(crate) facts: Vec<Fact>,
    pub(crate) questions: Vec<Question>,
}

/// The inventory that ships with xpd.
pub(crate) fn load() -> Result<Inventory> {
    serde_saphyr::from_str(INVENTORY).context("rules/xpd/chart-review.yml does not parse")
}
