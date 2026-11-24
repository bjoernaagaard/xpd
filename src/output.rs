use std::sync::LazyLock;

use clap::ValueEnum;
use regex::Regex;

/// How `graph`, `rules`, `find`, `api`, `new` and `test` print what they found.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    Text,
    Json,
}

/// The credentials in a URL, the user and password between `://` and the last `@` of its
/// authority, as in an index URL of `[[tool.uv.index]]` in `pyproject.toml`.
static CREDENTIALS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b([a-z][a-z0-9+.-]*://)[^/?#\s"'\\<>`]+@"#)
        .expect("the credentials pattern parses")
});

/// `text` with the user and password of every URL replaced by `****`, so that no output shows a
/// credential from a project file or from a tool's message about it.
pub(crate) fn redact(text: &str) -> String {
    CREDENTIALS.replace_all(text, "${1}****@").into_owned()
}
