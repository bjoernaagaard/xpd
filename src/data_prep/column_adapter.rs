use std::path::Path;

use anyhow::{Context as _, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ledger::adapter_runner::invoke_adapter;

const PROFILE: &str = include_str!("../../py/profile.py");
const RUNNER: &str = include_str!("../../py/data_facts.py");
const RECORDED_TARGET: &str = "xpd-data-prep:";

#[derive(Serialize, Deserialize)]
struct AdapterSpec {
    command: Option<String>,
    target: Option<String>,
}

pub(crate) fn record_target(
    command: Option<String>,
    target: Option<String>,
) -> Result<Option<String>> {
    if target.is_none() {
        return Ok(command);
    }
    let spec = serde_json::to_string(&AdapterSpec { command, target })?;
    Ok(Some(format!("{RECORDED_TARGET}{spec}")))
}

fn decode(spec: &str) -> Result<AdapterSpec> {
    if let Some(recorded) = spec.strip_prefix(RECORDED_TARGET) {
        return serde_json::from_str(recorded).context("invalid recorded data-prep adapter");
    }
    Ok(AdapterSpec {
        command: Some(spec.to_owned()),
        target: None,
    })
}

#[cfg(feature = "ledger-distributed")]
pub(crate) fn adapter_command(spec: &str) -> Result<Option<String>> {
    Ok(decode(spec)?.command)
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(super) struct Facts {
    #[serde(default)]
    pub(super) columns: Vec<Column>,
    #[serde(default)]
    pub(super) target: Option<String>,
    #[serde(default)]
    pub(super) model_family: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<String>,
    #[serde(flatten)]
    pub(super) measurements: serde_json::Map<String, Value>,
}

impl Facts {
    pub(super) fn duplicate_rows(&self) -> bool {
        self.measurements
            .get("duplicate_row_share")
            .and_then(Value::as_f64)
            .is_some_and(|share| share > 0.0)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Column {
    #[serde(rename = "column", alias = "name")]
    pub(super) name: String,
    #[serde(default)]
    pub(super) numeric: Option<bool>,
    #[serde(default)]
    pub(super) categorical: bool,
    #[serde(default)]
    pub(super) key_like: bool,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    skew: Option<f64>,
    #[serde(default)]
    outlier_share: Option<f64>,
    #[serde(default)]
    pub(super) zero_share: Option<f64>,
    #[serde(default)]
    pub(super) recommendation: String,
    #[serde(flatten)]
    measurements: serde_json::Map<String, Value>,
}

impl Column {
    fn recommend(&self, model_family: &str, target: Option<&str>, rows: u64) -> &'static str {
        if model_family.eq_ignore_ascii_case("tree") {
            return "none (tree model)";
        }
        if target == Some(self.name.as_str()) {
            return match (self.min, self.skew) {
                (Some(min), Some(skew)) if min > 0.0 && skew > 4.0 => {
                    "TransformedTargetRegressor(transformer=PowerTransformer(method=\"box-cox\"))"
                }
                (Some(min), Some(skew)) if min >= 0.0 && skew > 2.0 => {
                    "TransformedTargetRegressor(func=log1p, inverse_func=expm1)"
                }
                _ => "none (no target transform)",
            };
        }
        if self.zero_share.is_some_and(|share| share > 0.5) {
            return "none (review the zero-inflated distribution)";
        }
        let (Some(skew), Some(outliers)) = (self.skew, self.outlier_share) else {
            return "none (no measured numeric distribution)";
        };
        if outliers > 0.05 || skew.abs() > 2.0 {
            return if rows >= 1_000 {
                "RobustScaler; alternatives: QuantileTransformer(output_distribution=\"normal\") or PowerTransformer(method=\"yeo-johnson\")"
            } else if rows >= 30 {
                "RobustScaler; alternative: PowerTransformer(method=\"yeo-johnson\")"
            } else {
                "RobustScaler"
            };
        }
        if skew.abs() <= 1.0 && outliers <= 0.01 {
            "StandardScaler"
        } else {
            "RobustScaler"
        }
    }
}

pub(super) fn measure(spec: &str, directory: &Path) -> Result<Option<Facts>> {
    let AdapterSpec { command, target } = decode(spec)?;
    let Some(command) = command else {
        return Ok(None);
    };
    let source = serde_json::to_string(PROFILE)?;
    let selected = target
        .as_ref()
        .map_or_else(|| Ok("None".to_owned()), serde_json::to_string)?;
    let runner = format!(
        "profile = {{\"__name__\": \"xpd_profile\"}}\nexec({source}, profile)\n{RUNNER}\nmain(profile, {selected})"
    );
    let payload = invoke_adapter(&command, directory, &runner)?;
    ensure!(
        payload["columns"].is_array(),
        "adapter JSON must hold a columns array"
    );
    let mut facts: Facts =
        serde_json::from_value(payload).context("invalid data-prep column facts")?;
    facts.target = target.or(facts.target).or_else(|| {
        facts.columns.iter().find_map(|column| {
            matches!(column.name.as_str(), "y" | "target" | "label").then(|| column.name.clone())
        })
    });
    if let Some(target) = &facts.target {
        ensure!(
            facts.columns.iter().any(|column| &column.name == target),
            "target column {target:?} is absent from adapter JSON"
        );
    }
    let rows = facts
        .measurements
        .get("rows")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    for column in &mut facts.columns {
        column.recommendation = column
            .recommend(&facts.model_family, facts.target.as_deref(), rows)
            .to_owned();
    }
    facts
        .measurements
        .insert("adapter".to_owned(), Value::String(command));
    Ok(Some(facts))
}
