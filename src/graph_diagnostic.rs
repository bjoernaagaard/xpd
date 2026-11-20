/// How sure a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    /// Hamilton raises, or Python drops the function.
    Error,
    /// A guess that a person should look at.
    Warning,
    /// Something `xpd graph` could not read.
    Note,
}

impl Level {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// Something about one function that the graph alone does not show.
#[derive(Debug)]
pub(crate) struct Finding {
    pub(crate) line: usize,
    pub(crate) level: Level,
    pub(crate) message: String,
}
