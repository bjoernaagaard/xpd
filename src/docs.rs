/// The folder of the published rule pages, one page for each pack and each rule in it.
const RULES_URL: &str = concat!(env!("CARGO_PKG_HOMEPAGE"), "rules/");

/// The address of the published page about the rule or package check `id` of the pack `module`.
pub(crate) fn rule_url(module: &str, id: &str) -> String {
    format!("{RULES_URL}{module}/{id}/")
}
