pub(crate) const OWN_PACK: &str = "xpd";

/// Pack names become Python identifiers in the import gate.
pub(crate) fn is_module_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The public module containing an indexed function or class.
pub(crate) fn public_module<'a>(package: &'a str, symbol: &'a str) -> &'a str {
    symbol
        .rsplit_once('.')
        .map_or(package, |(module, _)| module)
}
