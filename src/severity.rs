use ast_grep_config::Severity;

pub(crate) fn name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
        Severity::Off => "off",
    }
}

/// The severity that `name` names, as a rule file writes it. `off` is no severity to ask for.
pub(crate) fn parse(text: &str) -> Result<Severity, String> {
    [
        Severity::Error,
        Severity::Warning,
        Severity::Info,
        Severity::Hint,
    ]
    .into_iter()
    .find(|severity| name(severity) == text)
    .ok_or_else(|| format!("{text} is not error, warning, info or hint"))
}

/// How severe a severity is, so that merging a rule's documents keeps the highest.
pub(crate) fn rank(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 4,
        Severity::Warning => 3,
        Severity::Info => 2,
        Severity::Hint => 1,
        Severity::Off => 0,
    }
}
