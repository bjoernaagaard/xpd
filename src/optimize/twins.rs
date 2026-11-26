//! Near copies: stretches of code with the same structure that differ in a few names or
//! values, such as two classes around two functions of a library. A stretch that is equal
//! token for token is a copy, which `xpd untangle` reports.

use std::collections::{BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::syntax::Node;

/// The fewest tokens of a near copy. A shorter stretch has the structure of another by chance.
const TOKENS: usize = 60;
/// The fewest lines of a near copy.
const LINES: usize = 5;
/// The most pairs of texts that two near copies differ in. Each pair is one name or value that
/// a shared function would take as a parameter.
const SWAPS: usize = 4;
/// The largest share of the tokens, in percent, that may differ.
const DIFFERENT_PERCENT: usize = 25;

/// A stretch of one file that is long enough to be a near copy.
#[derive(Debug, Clone)]
pub(super) struct Block {
    /// The same for two stretches with the same kinds of tokens in the same structure.
    pub(super) shape: u64,
    /// The index of the nearest stretch around this one, in the blocks of the file.
    parent: Option<usize>,
    pub(super) tokens: usize,
    /// The first and the last line, counted from 1.
    pub(super) lines: (usize, usize),
    /// The first and the last of its tokens, in the tokens of the file.
    leaves: (usize, usize),
}

/// The stretches of one file, and the text of each of its tokens.
#[derive(Debug, Default)]
pub(super) struct Shapes {
    blocks: Vec<Block>,
    leaves: Vec<String>,
}

/// The stretches of the tree below `root` that are long enough to be a near copy.
pub(super) fn shapes(root: &Node<'_>) -> Shapes {
    let mut shapes = Shapes::default();
    visit(root, &mut shapes);
    shapes
}

/// Whether `node` is a docstring: the first named statement of a module, class or function body
/// holding only a string literal. Skipping docstrings keeps prose out of the swap budget.
fn is_docstring(node: &Node<'_>) -> bool {
    if node.kind() != "expression_statement" {
        return false;
    }
    let mut named = node.children().filter(Node::is_named);
    if named.next().is_none_or(|child| child.kind() != "string") || named.next().is_some() {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    if !matches!(parent.kind().as_ref(), "block" | "module") {
        return false;
    }
    parent
        .children()
        .find(|child| child.is_named() && !child.kind().contains("comment"))
        .is_some_and(|first| first.range() == node.range())
}

/// The token count and the shape of `node`. Adds each stretch below it to `shapes`, and returns
/// the indexes of the nearest ones so that the caller can name itself as their parent.
fn visit(node: &Node<'_>, shapes: &mut Shapes) -> (usize, u64, Vec<usize>) {
    if is_docstring(node) {
        return (0, 0, Vec::new());
    }
    let mut hasher = DefaultHasher::new();
    let children = node.children();
    if children.len() == 0 {
        if node.kind().contains("comment") {
            return (0, 0, Vec::new());
        }
        node.kind_id().hash(&mut hasher);
        shapes.leaves.push(node.text().into_owned());
        return (1, hasher.finish(), Vec::new());
    }
    let first = shapes.leaves.len();
    let mut tokens = 0;
    let mut nearest = Vec::new();
    for child in children {
        let (count, shape, below) = visit(&child, shapes);
        if count > 0 {
            tokens += count;
            shape.hash(&mut hasher);
        }
        nearest.extend(below);
    }
    node.kind_id().hash(&mut hasher);
    let shape = hasher.finish();
    let lines = (node.start_pos().line() + 1, node.end_pos().line() + 1);
    if tokens < TOKENS || lines.1 - lines.0 + 1 < LINES {
        return (tokens, shape, nearest);
    }
    let index = shapes.blocks.len();
    for inner in nearest {
        shapes.blocks[inner].parent = Some(index);
    }
    shapes.blocks.push(Block {
        shape,
        parent: None,
        tokens,
        lines,
        leaves: (first, shapes.leaves.len()),
    });
    (tokens, shape, vec![index])
}

/// A stretch by the index of its file and its index among the stretches of that file.
type At = (usize, usize);

/// A text that two stretches differ in, as the first writes it and as the other does.
pub(super) type Swap<'f> = (&'f str, &'f str);

/// Stretches of code that are the same but for `swaps`.
#[derive(Debug)]
pub(super) struct Twins<'f> {
    /// The file and the stretch of each, in the order of the files.
    pub(super) members: Vec<(&'f str, &'f Block)>,
    /// The texts that differ between the first stretch and another one.
    pub(super) swaps: Vec<Swap<'f>>,
}

/// The texts that the tokens `a` and `b` differ in. `None` when they differ in more than a
/// near copy may, or in nothing.
fn swaps<'f>(a: &'f [String], b: &'f [String]) -> Option<Vec<Swap<'f>>> {
    let mut different = 0;
    let mut pairs = BTreeSet::new();
    for (a, b) in a.iter().zip(b) {
        if a != b {
            different += 1;
            pairs.insert((a.as_str(), b.as_str()));
            if pairs.len() > SWAPS {
                return None;
            }
        }
    }
    (different > 0 && different * 100 <= a.len() * DIFFERENT_PERCENT)
        .then(|| pairs.into_iter().collect())
}

/// The groups of near copies among the files, each file with its path. A stretch inside a
/// larger near copy is left out, so each one is named once, at its full size.
pub(super) fn groups<'f>(files: &'f [(&'f str, Shapes)]) -> Vec<Twins<'f>> {
    let mut by_shape: HashMap<u64, Vec<At>> = HashMap::new();
    for (file, (_, shapes)) in files.iter().enumerate() {
        for (index, block) in shapes.blocks.iter().enumerate() {
            by_shape.entry(block.shape).or_default().push((file, index));
        }
    }
    let leaves = |(file, index): At| {
        let shapes = &files[file].1;
        let (first, end) = shapes.blocks[index].leaves;
        &shapes.leaves[first..end]
    };
    let mut found: Vec<(Vec<At>, Vec<Swap<'f>>)> = Vec::new();
    for alike in by_shape.values().filter(|alike| alike.len() > 1) {
        let mut left: Vec<At> = alike.clone();
        left.sort_unstable();
        while let Some((&first, rest)) = left.split_first() {
            let mut members = vec![first];
            let mut differing = Vec::new();
            let mut apart = Vec::new();
            for &other in rest {
                if leaves(first) == leaves(other) {
                    members.push(other);
                } else if let Some(pairs) = swaps(leaves(first), leaves(other)) {
                    members.push(other);
                    differing.extend(pairs);
                } else {
                    apart.push(other);
                }
            }
            if !differing.is_empty() {
                differing.sort_unstable();
                differing.dedup();
                found.push((members, differing));
            }
            left = apart;
        }
    }
    let grouped: BTreeSet<At> = (found.iter())
        .flat_map(|(members, _)| members.iter().copied())
        .collect();
    let inside = |&(file, index): &At| {
        (files[file].1.blocks[index].parent).is_some_and(|parent| grouped.contains(&(file, parent)))
    };
    let mut groups: Vec<Twins<'f>> = (found.into_iter())
        .filter(|(members, _)| !members.iter().all(inside))
        .map(|(members, swaps)| Twins {
            members: (members.into_iter())
                .map(|(file, index)| (files[file].0, &files[file].1.blocks[index]))
                .collect(),
            swaps,
        })
        .collect();
    groups.sort_by_key(|twins| {
        let (file, block) = twins.members[0];
        (
            std::cmp::Reverse(block.tokens * twins.members.len()),
            file,
            block.lines,
        )
    });
    groups
}
