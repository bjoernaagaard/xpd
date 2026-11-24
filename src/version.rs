//! Release numbers of Python distributions, such as `3.1.1`.

use std::fmt;
use std::str::FromStr;

use anyhow::{Context, Error, Result, ensure};
use serde::Deserialize;

/// A release number such as `3.1` or `3.1.1`. Trailing zeros are dropped, so `3.1` equals
/// `3.1.0`, and versions order part by part.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Version(Vec<u32>);

impl FromStr for Version {
    type Err = Error;

    /// Reads the leading numbers and ignores what follows them, such as `rc1` in `3.2.0rc1`.
    fn from_str(text: &str) -> Result<Self> {
        let numeric = text
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .next()
            .unwrap_or_default();
        let mut parts = numeric
            .split('.')
            .take_while(|part| !part.is_empty())
            .map(str::parse)
            .collect::<Result<Vec<u32>, _>>()
            .with_context(|| format!("{text:?} has a release number that is too large"))?;
        ensure!(
            !parts.is_empty(),
            "{text:?} does not start with a release number such as 3.1"
        );
        while parts.last() == Some(&0) {
            parts.pop();
        }
        Ok(Self(parts))
    }
}

impl Version {
    /// Whether the parts of `prefix` start this version, as `8.4` starts `8.4.2`.
    pub(crate) fn starts_with(&self, prefix: &Self) -> bool {
        let mut parts = self.0.clone();
        parts.resize(parts.len().max(prefix.0.len()), 0);
        parts.starts_with(&prefix.0)
    }

    /// This version without its last part, as `8` for `8.4`, for the bound of `~=`.
    pub(crate) fn without_last(&self) -> Self {
        let mut parts = self.0.clone();
        parts.pop();
        Self(parts)
    }
}

impl TryFrom<String> for Version {
    type Error = Error;

    fn try_from(text: String) -> Result<Self> {
        text.parse()
    }
}

impl fmt::Display for Version {
    /// Writes at least a major and a minor part, so `2.0` stays `2.0` after its zero is dropped.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self.0.iter();
        write!(f, "{}", parts.next().unwrap_or(&0))?;
        match parts.next() {
            Some(minor) => write!(f, ".{minor}")?,
            None => write!(f, ".0")?,
        }
        parts.try_for_each(|part| write!(f, ".{part}"))
    }
}
