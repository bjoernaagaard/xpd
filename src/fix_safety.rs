use std::fmt;

use clap::{ValueEnum, builder::PossibleValue};
use serde::{Deserialize, Serialize};

/// Whether a fix keeps the code's behavior. `xpd check --fix` applies safe fixes, and
/// `--unsafe-fixes` adds the unsafe ones. The order matters: a fix applies when its
/// applicability is at or below the one asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Applicability {
    Safe,
    Unsafe,
}

impl Applicability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Unsafe => "unsafe",
        }
    }
}

impl ValueEnum for Applicability {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Safe, Self::Unsafe]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.as_str()))
    }
}

impl fmt::Display for Applicability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
