use serde::Deserialize;

use crate::severity::Severity;

/// How xpd reports the rules of the house style, which `metadata.style: house` marks. Such a
/// rule states a preference about code that works.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum Style {
    /// Each rule has the severity that its file gives it.
    #[default]
    Default,
    /// A rule of the house style that is `info` is a `warning`.
    Strict,
}

impl Style {
    pub(crate) const ALL: [Self; 2] = [Self::Default, Self::Strict];

    pub(crate) fn severity(self, declared: Severity, mark: Option<Mark>) -> Severity {
        match (self, mark, declared) {
            (Self::Strict, Some(Mark::House), Severity::Info) => Severity::Warning,
            _ => declared,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Strict => "strict",
        }
    }
}

/// What `metadata.style` says about a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mark {
    /// The rule states a preference of the house style about code that works. With
    /// `style = "strict"` in the settings, such a rule that is `info` is a `warning`.
    House,
}

impl Mark {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::House => "house",
        }
    }
}
