//! Copies of code: whole definitions or blocks that are written out in more than one place,
//! token for token. Comments and layout are not compared.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::syntax::Node;

/// The fewest tokens of a copy. A shorter stretch, such as a call with a few arguments, repeats
/// by chance.
const TOKENS: usize = 60;
/// The fewest lines of a copy.
const LINES: usize = 5;

/// A stretch of one file that is long enough to be a copy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TokenBlock {
    /// The same for two stretches with the same tokens.
    pub(super) hash: u64,
    /// The hash of the nearest stretch around this one.
    parent: Option<u64>,
    pub(super) tokens: usize,
    /// The first and the last line, counted from 1.
    pub(super) lines: (usize, usize),
}

/// The stretches of the tree below `root` that are long enough to be a copy.
pub(crate) fn blocks(root: &Node<'_>) -> Vec<TokenBlock> {
    let mut blocks = Vec::new();
    visit(root, &mut blocks);
    blocks
}

/// The token count and the hash of `node`. Adds each stretch below it to `blocks`, and returns
/// the indexes of the nearest ones so that the caller can name itself as their parent.
fn visit(node: &Node<'_>, blocks: &mut Vec<TokenBlock>) -> (usize, u64, Vec<usize>) {
    let mut hasher = DefaultHasher::new();
    let children = node.children();
    if children.len() == 0 {
        if node.kind().contains("comment") {
            return (0, 0, Vec::new());
        }
        node.kind_id().hash(&mut hasher);
        node.text().hash(&mut hasher);
        return (1, hasher.finish(), Vec::new());
    }
    let mut tokens = 0;
    let mut nearest = Vec::new();
    for child in children {
        let (count, hash, below) = visit(&child, blocks);
        if count > 0 {
            tokens += count;
            hash.hash(&mut hasher);
        }
        nearest.extend(below);
    }
    node.kind_id().hash(&mut hasher);
    let hash = hasher.finish();
    let lines = (node.start_pos().line() + 1, node.end_pos().line() + 1);
    if tokens < TOKENS || lines.1 - lines.0 + 1 < LINES {
        return (tokens, hash, nearest);
    }
    for inner in nearest {
        blocks[inner].parent = Some(hash);
    }
    blocks.push(TokenBlock {
        hash,
        parent: None,
        tokens,
        lines,
    });
    (tokens, hash, vec![blocks.len() - 1])
}

/// The groups of equal stretches among `blocks`, each stretch with the file it is in. A stretch
/// inside a larger copy is left out, so each copy is named once, at its full size.
pub(super) fn exact_copy_groups<'f>(
    blocks: &'f [(&'f str, TokenBlock)],
) -> Vec<Vec<(&'f str, &'f TokenBlock)>> {
    let mut by_hash: HashMap<u64, Vec<(&str, &TokenBlock)>> = HashMap::new();
    for (file, block) in blocks {
        by_hash.entry(block.hash).or_default().push((file, block));
    }
    let copied = |hash: u64| by_hash.get(&hash).is_some_and(|group| group.len() > 1);
    let mut groups: Vec<Vec<(&str, &TokenBlock)>> = (by_hash.values())
        .filter(|group| group.len() > 1)
        .filter(|group| (group.iter()).any(|(_, block)| !block.parent.is_some_and(copied)))
        .cloned()
        .collect();
    for group in &mut groups {
        group.sort_by_key(|(file, block)| (*file, block.lines));
    }
    groups.sort_by_key(|group| {
        let (file, block) = group[0];
        (
            std::cmp::Reverse(block.tokens * group.len()),
            file,
            block.lines,
        )
    });
    groups
}

use std::sync::Arc;

use crate::ledger::salsa_db::SourceText;

/// How many per-file facts stay in memory at once.
#[expect(dead_code, reason = "the surveys that call it come in the next slice")]
pub(crate) const UNTANGLE_FACTS_LRU: usize = 2048;

/// The per-file facts of `xpd untangle` for one file: its lines and its copied stretches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UntangleFileSummary {
    /// The lines of the file.
    pub(crate) lines: usize,
    /// The stretches of the file that are long enough to be a copy.
    pub(crate) blocks: Vec<TokenBlock>,
}

/// The facts of one file for `xpd untangle`, as a tracked query with a memory cap. Only the
/// compact stretches are kept, never a syntax tree.
#[salsa::tracked(lru = 2048, returns(ref))]
pub(crate) fn untangle_file_facts(
    db: &dyn salsa::Database,
    source: SourceText,
) -> Arc<UntangleFileSummary> {
    let text: Arc<str> = Arc::from(source.text(db));
    let root = crate::syntax::parse_as(&text, source.lang(db));
    Arc::new(UntangleFileSummary {
        lines: text.lines().count(),
        blocks: blocks(&root.root()),
    })
}
