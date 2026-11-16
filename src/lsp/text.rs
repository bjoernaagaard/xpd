//! Positions in a document. xpd counts bytes of UTF-8, and a client counts lines and code units
//! of the encoding it agreed on with the server.

use std::ops::Range;

use lsp_types::{Position, PositionEncodingKind, TextDocumentContentChangeEvent};

/// The unit a position's `character` counts within its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    /// The first encoding the client offers that the server knows. The client lists them in
    /// the order it prefers, and UTF-16 is what the protocol uses when it offers none.
    pub(crate) fn negotiate(offered: Option<&[PositionEncodingKind]>) -> Self {
        offered
            .into_iter()
            .flatten()
            .find_map(|kind| match kind.as_str() {
                "utf-8" => Some(Self::Utf8),
                "utf-16" => Some(Self::Utf16),
                "utf-32" => Some(Self::Utf32),
                _ => None,
            })
            .unwrap_or(Self::Utf16)
    }

    pub(crate) fn kind(self) -> PositionEncodingKind {
        match self {
            Self::Utf8 => PositionEncodingKind::UTF8,
            Self::Utf16 => PositionEncodingKind::UTF16,
            Self::Utf32 => PositionEncodingKind::UTF32,
        }
    }

    /// The number of code units `c` takes.
    fn width(self, c: char) -> usize {
        match self {
            Self::Utf8 => c.len_utf8(),
            Self::Utf16 => c.len_utf16(),
            Self::Utf32 => 1,
        }
    }
}

/// Where each line of a text starts. The protocol ends a line at `\n`, `\r\n` or `\r`.
#[derive(Debug)]
pub(crate) struct Lines<'t> {
    text: &'t str,
    /// The byte each line starts at. The first is 0.
    starts: Vec<usize>,
    encoding: Encoding,
}

impl<'t> Lines<'t> {
    pub(crate) fn new(text: &'t str, encoding: Encoding) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        for (at, byte) in bytes.iter().enumerate() {
            let ends_line = match byte {
                b'\n' => true,
                b'\r' => bytes.get(at + 1) != Some(&b'\n'),
                _ => false,
            };
            if ends_line {
                starts.push(at + 1);
            }
        }
        Self {
            text,
            starts,
            encoding,
        }
    }

    /// The bytes of line `line` without the characters that end it.
    fn content(&self, line: usize) -> Range<usize> {
        let start = self.starts[line];
        let next = self.starts.get(line + 1).copied();
        let end = next.unwrap_or(self.text.len());
        let ending = self.text[start..end]
            .bytes()
            .rev()
            .take_while(|byte| matches!(byte, b'\n' | b'\r'))
            .count();
        // Only the last line can lack an ending, and then it has nothing to cut.
        start..end - if next.is_some() { ending } else { 0 }
    }

    /// The line that holds the byte `offset`, counted from 0.
    fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset) - 1
    }

    /// The position of the byte `offset`. An offset inside a character counts as its start.
    pub(crate) fn position(&self, offset: usize) -> Position {
        let offset = self.text.floor_char_boundary(offset);
        let line = self.line_of(offset);
        let character: usize = self.text[self.starts[line]..offset]
            .chars()
            .map(|c| self.encoding.width(c))
            .sum();
        Position {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: u32::try_from(character).unwrap_or(u32::MAX),
        }
    }

    /// The byte at `position`. A line past the end is the end of the text, a character past the
    /// end of its line is the end of that line, and a character inside a code point is the
    /// start of that code point, as the protocol says.
    pub(crate) fn offset(&self, position: Position) -> usize {
        let Some(line) = usize::try_from(position.line)
            .ok()
            .filter(|line| *line < self.starts.len())
        else {
            return self.text.len();
        };
        let content = self.content(line);
        let wanted = usize::try_from(position.character).unwrap_or(usize::MAX);
        let mut units = 0;
        for (at, c) in self.text[content.clone()].char_indices() {
            units += self.encoding.width(c);
            if units > wanted {
                return content.start + at;
            }
        }
        content.end
    }

    /// The byte where the line that holds `offset` ends, before the characters that end it.
    pub(crate) fn line_end(&self, offset: usize) -> usize {
        self.content(self.line_of(offset)).end
    }
}

/// Applies one change of `textDocument/didChange` to `text`. A change without a range holds
/// the whole new text.
pub(crate) fn apply(text: &mut String, change: TextDocumentContentChangeEvent, encoding: Encoding) {
    let Some(range) = change.range else {
        *text = change.text;
        return;
    };
    let lines = Lines::new(text, encoding);
    let start = lines.offset(range.start);
    let end = lines.offset(range.end).max(start);
    text.replace_range(start..end, &change.text);
}
