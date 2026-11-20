use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;

/// A parsed file.
pub(crate) type Root = AstGrep<StrDoc<SupportLang>>;

/// One node of a parsed file.
pub(crate) type Node<'t> = ast_grep_core::Node<'t, StrDoc<SupportLang>>;

/// Parses Python source. A syntax error gives a tree with error nodes, not a failure.
pub(crate) fn parse(src: &str) -> Root {
    parse_as(src, SupportLang::Python)
}

/// Parses source of `language`.
pub(crate) fn parse_as(src: &str, language: SupportLang) -> Root {
    use ast_grep_core::tree_sitter::LanguageExt;

    language.ast_grep(src)
}

/// The text of a string literal with no interpolation, without its quotes.
pub(crate) fn string_literal(node: &Node<'_>) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    let mut text = String::new();
    for part in node.children() {
        match part.kind().as_ref() {
            "string_content" => text.push_str(&part.text()),
            "string_start" | "string_end" => {}
            _ => return None,
        }
    }
    Some(text)
}

/// `IPython` magic lines are valid between cell markers. Mask only line-leading syntax errors
/// so that a continuation such as `% b)` is preserved.
pub(crate) fn parses_python_cells(code: &str) -> bool {
    let parses = |source: &str| libcst_native::parse_module(source, None).is_ok();
    if parses(code) {
        return true;
    }
    let root = parse(code);
    let mut hidden = code.to_owned();
    for error in root
        .root()
        .dfs()
        .filter(|node| node.is_error() || node.is_missing())
    {
        let at = error.range().start;
        let line = code[..at].rfind('\n').map_or(0, |end| end + 1);
        let indent = code[line..at].chars().all(|c| matches!(c, ' ' | '\t'));
        if indent && (code[at..].starts_with('%') || code[at..].starts_with('!')) {
            hidden.replace_range(at..=at, "#");
        }
    }
    parses(&hidden)
}
