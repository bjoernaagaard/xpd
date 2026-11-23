use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::{fmt, fs};

use anyhow::{Context, Result, ensure};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Contract {
    pub(super) input_schema: Vec<Column>,
    pub(super) output_schema: Vec<Column>,
    pub(super) row_identity: RowIdentity,
    pub(super) materialization: Materialization,
    #[serde(default, deserialize_with = "unique_tolerances")]
    pub(super) float_atol: BTreeMap<String, f64>,
    #[serde(default)]
    pub(super) missing_float_columns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Column {
    pub(super) name: String,
    pub(super) dtype: Dtype,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(super) enum Dtype {
    String,
    Int64,
    Float64,
    Boolean,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RowIdentity {
    Position,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Materialization {
    Lazy,
    Eager,
}

impl Contract {
    pub(super) fn read(path: &Path) -> Result<Self> {
        let read = || -> Result<Self> {
            let contract: Self = serde_json::from_slice(&fs::read(path)?)?;
            check_schema(&contract.input_schema)?;
            check_schema(&contract.output_schema)?;
            let floats: BTreeSet<_> = contract
                .output_schema
                .iter()
                .filter(|column| column.dtype == Dtype::Float64)
                .map(|column| column.name.as_str())
                .collect();
            for (name, tolerance) in &contract.float_atol {
                ensure!(
                    floats.contains(name.as_str()),
                    "float_atol names non-Float64 output {name:?}"
                );
                ensure!(
                    tolerance.is_finite() && *tolerance >= 0.0,
                    "invalid float_atol for {name:?}"
                );
            }
            let mut missing = BTreeSet::new();
            for name in &contract.missing_float_columns {
                ensure!(
                    floats.contains(name.as_str()),
                    "missing_float_columns names non-Float64 output {name:?}"
                );
                ensure!(
                    missing.insert(name),
                    "duplicate missing_float_columns entry {name:?}"
                );
            }
            Ok(contract)
        };
        read().with_context(|| format!("cannot read migration contract {}", path.display()))
    }
}

fn check_schema(schema: &[Column]) -> Result<()> {
    let mut names = BTreeSet::new();
    for column in schema {
        ensure!(!column.name.is_empty(), "empty column name");
        ensure!(
            names.insert(&column.name),
            "duplicate column {:?}",
            column.name
        );
    }
    Ok(())
}

fn unique_tolerances<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Unique;

    impl<'de> Visitor<'de> for Unique {
        type Value = BTreeMap<String, f64>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object of float tolerances with distinct column names")
        }

        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut values = BTreeMap::new();
            while let Some((name, value)) = map.next_entry()? {
                if values.insert(name, value).is_some() {
                    return Err(de::Error::custom("duplicate float_atol column"));
                }
            }
            Ok(values)
        }
    }

    deserializer.deserialize_map(Unique)
}
