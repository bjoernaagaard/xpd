use std::fmt;

use ast_grep_config::Severity as AstSeverity;
use clap::{ValueEnum, builder::PossibleValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Severity {
    Off,
    Hint,
    Info,
    Warning,
    Error,
}

impl Severity {
    const SELECTABLE: [Self; 4] = [Self::Error, Self::Warning, Self::Info, Self::Hint];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
            Self::Off => "off",
        }
    }

    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        Self::SELECTABLE
            .into_iter()
            .find(|severity| severity.as_str() == text)
            .ok_or_else(|| format!("{text} is not error, warning, info or hint"))
    }
}

impl ValueEnum for Severity {
    fn value_variants<'a>() -> &'a [Self] {
        &Self::SELECTABLE
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        (*self != Self::Off).then(|| PossibleValue::new(self.as_str()))
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&AstSeverity> for Severity {
    fn from(severity: &AstSeverity) -> Self {
        match severity {
            AstSeverity::Off => Self::Off,
            AstSeverity::Hint => Self::Hint,
            AstSeverity::Info => Self::Info,
            AstSeverity::Warning => Self::Warning,
            AstSeverity::Error => Self::Error,
        }
    }
}
