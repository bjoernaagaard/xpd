use std::collections::HashSet;
use std::sync::LazyLock;

use clap::ValueEnum;
use regex::Regex;

/// How `graph`, `rules`, `find`, `api`, `new` and `test` print what they found.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Format {
    Text,
    Json,
}

/// A URL: its scheme and the rest up to a space, a quote, a backslash or an angle bracket.
static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b([a-z][a-z0-9+.-]*://)([^\s"'\\<>`]*)"#).expect("the URL pattern parses")
});

/// `text` with the parts of every URL that carry credentials replaced by `****`: the user and
/// password before the host, the query and the fragment. A token in a path segment stays,
/// because a path cannot be told from a token, so doctor also hides the lines of tables it does
/// not own.
pub(crate) fn redact(text: &str) -> String {
    redact_except(text, &HashSet::new())
}

/// Every URL in `text`, as [`redact`] finds them.
pub(crate) fn urls(text: &str) -> HashSet<String> {
    URL.find_iter(text)
        .map(|found| found.as_str().to_owned())
        .collect()
}

/// `text` as [`redact`] leaves it, except that a URL equal to one of `keep` stays whole. `keep`
/// holds xpd's own constants, such as the source links of doctor's catalog, whose anchors make
/// the links useful. A URL from a project matches only when it is one of those very strings.
pub(crate) fn redact_except(text: &str, keep: &HashSet<String>) -> String {
    URL.replace_all(text, |captures: &regex::Captures<'_>| {
        if keep.contains(&captures[0]) {
            return captures[0].to_owned();
        }
        let scheme = &captures[1];
        let rest = &captures[2];
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        let host = authority
            .rsplit_once('@')
            .map_or_else(|| authority.to_owned(), |(_, host)| format!("****@{host}"));
        let (tail, fragment) = match tail.split_once('#') {
            Some((tail, _)) => (tail, "#****"),
            None => (tail, ""),
        };
        let tail = match tail.split_once('?') {
            Some((path, _)) => format!("{path}?****"),
            None => tail.to_owned(),
        };
        format!("{scheme}{host}{tail}{fragment}")
    })
    .into_owned()
}

/// `text`, a message of another tool, with every URL cut to its scheme and host, since a tool
/// can quote an index URL whose path holds a token.
pub(crate) fn hosts_only(text: &str) -> String {
    URL.replace_all(text, |captures: &regex::Captures<'_>| {
        let scheme = &captures[1];
        let rest = &captures[2];
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        let host = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let tail = if tail.is_empty() { "" } else { "/****" };
        format!("{scheme}{host}{tail}")
    })
    .into_owned()
}
