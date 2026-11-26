//! # Source files: turning byte offsets into line and column
//!
//! Spans store byte offsets (see `span.rs`). People and editors want
//! "line 10, column 25". This module does that conversion.
//!
//! ## The line table
//!
//! When a file is loaded we scan it once and record the byte offset where each
//! line starts (`line_starts`). Line 1 always starts at offset 0; every `\n`
//! starts a new line at the following byte. To find the line containing an
//! offset we binary-search that sorted table, which is O(log lines) instead of
//! rescanning the file for every diagnostic.
//!
//! ## Columns count characters (DECISIONS D-19)
//!
//! A column is the number of Unicode characters (scalar values) from the start
//! of the line, plus one. So in `let s = "€x"` the `x` is at column 11 even
//! though `€` takes 3 bytes in UTF-8. The LSP (M5) converts to UTF-16 units
//! at its own boundary, because that is what the LSP protocol requires.
//!
//! ## Line endings
//!
//! Only `\n` starts a new line. With Windows `\r\n` endings the `\r` is the
//! last character of its line, so columns before it are unaffected. A lone
//! `\r` (classic Mac OS) is not treated as a line break.
//!
//! ## Robustness
//!
//! The conversion never panics. An offset past the end of the file is clamped
//! to the end, and an offset that lands inside a multibyte character is moved
//! back to the start of that character. Both would indicate a compiler bug,
//! but a bad diagnostic location is better than a crash.

use crate::span::{FileId, Span};

/// A 1-based line and column, ready for display or JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in Unicode characters.
    pub col: u32,
}

/// Returned when a file is too large for `u32` byte offsets (4 GiB).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTooLarge {
    pub name: String,
    pub len: usize,
}

/// One loaded source file: its display name, full text, and line table.
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    text: String,
    /// Byte offset where each line begins. Always non-empty; `line_starts[0]` is 0.
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Loads `text` under the display `name` (usually a path) and builds the
    /// line table.
    pub fn new(
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<SourceFile, FileTooLarge> {
        let name = name.into();
        let text = text.into();
        if u32::try_from(text.len()).is_err() {
            return Err(FileTooLarge {
                name,
                len: text.len(),
            });
        }

        let mut line_starts = vec![0u32];
        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                // Cannot overflow: i + 1 <= text.len(), which fits in u32 (checked above).
                line_starts.push((i + 1) as u32);
            }
        }
        Ok(SourceFile {
            name,
            text,
            line_starts,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Length of the file in bytes.
    pub fn len(&self) -> u32 {
        // Fits: checked in `new`.
        self.text.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Number of lines. An empty file has one (empty) line, and a trailing
    /// `\n` starts a final empty line, matching how editors number them.
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Converts a byte offset to a 1-based line and character column.
    pub fn line_col(&self, offset: u32) -> LineCol {
        let offset = self.clamp_to_char_boundary(offset);

        // `partition_point` returns how many line starts are <= offset. Since
        // line_starts[0] == 0, that count is at least 1, and the line we want
        // is the last start that is <= offset.
        let line_index = self.line_starts.partition_point(|&start| start <= offset) - 1;
        let line_start = self.line_starts[line_index] as usize;

        let col = self.text[line_start..offset as usize].chars().count() + 1;
        LineCol {
            line: line_index as u32 + 1,
            col: col as u32,
        }
    }

    /// The text a span covers, or `None` if the span does not fit this file or
    /// does not fall on character boundaries.
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.text.get(span.start as usize..span.end as usize)
    }

    /// Clamps `offset` to the file length and moves it back to the start of
    /// the character it falls inside, so it is always safe to slice at.
    fn clamp_to_char_boundary(&self, offset: u32) -> u32 {
        let mut offset = (offset as usize).min(self.text.len());
        // Offset 0 is always a boundary, so this loop terminates.
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset as u32
    }
}

/// All source files in one compilation, addressed by `FileId`.
///
/// Files are only ever added, never removed, so a `FileId` handed out once
/// stays valid for the whole run.
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap::default()
    }

    /// Adds a file and returns its id.
    pub fn add(&mut self, file: SourceFile) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(file);
        id
    }

    /// Looks up a file. Returns `None` for an id from a different `SourceMap`.
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    /// All files with their ids, in the order they were added.
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &SourceFile)> {
        self.files
            .iter()
            .enumerate()
            .map(|(i, f)| (FileId(i as u32), f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(text: &str) -> SourceFile {
        SourceFile::new("test.plat", text).expect("small test file")
    }

    fn lc(line: u32, col: u32) -> LineCol {
        LineCol { line, col }
    }

    #[test]
    fn empty_file_has_one_line() {
        let f = file("");
        assert_eq!(f.line_count(), 1);
        assert_eq!(f.line_col(0), lc(1, 1));
    }

    #[test]
    fn single_line() {
        let f = file("let x = 1");
        assert_eq!(f.line_col(0), lc(1, 1));
        assert_eq!(f.line_col(4), lc(1, 5)); // `x`
        assert_eq!(f.line_col(9), lc(1, 10)); // end of file
    }

    #[test]
    fn line_boundaries() {
        // offsets: a=0 \n=1 b=2 \n=3 (empty line 3 starts at 4)
        let f = file("a\nb\n");
        assert_eq!(f.line_count(), 3);
        assert_eq!(f.line_col(0), lc(1, 1)); // a
        assert_eq!(f.line_col(1), lc(1, 2)); // the \n belongs to line 1
        assert_eq!(f.line_col(2), lc(2, 1)); // b
        assert_eq!(f.line_col(3), lc(2, 2));
        assert_eq!(f.line_col(4), lc(3, 1)); // end of file, on the trailing empty line
    }

    #[test]
    fn consecutive_newlines() {
        let f = file("\n\n\nx");
        assert_eq!(f.line_col(0), lc(1, 1));
        assert_eq!(f.line_col(1), lc(2, 1));
        assert_eq!(f.line_col(2), lc(3, 1));
        assert_eq!(f.line_col(3), lc(4, 1)); // x
    }

    #[test]
    fn crlf_line_endings() {
        // offsets: a=0 \r=1 \n=2 b=3
        let f = file("a\r\nb");
        assert_eq!(f.line_count(), 2);
        assert_eq!(f.line_col(1), lc(1, 2)); // \r is the last char of line 1
        assert_eq!(f.line_col(3), lc(2, 1)); // b
    }

    #[test]
    fn lone_cr_is_not_a_line_break() {
        let f = file("a\rb");
        assert_eq!(f.line_count(), 1);
        assert_eq!(f.line_col(2), lc(1, 3));
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        // `€` is 3 bytes, `😀` is 4 bytes, `é` is 2 bytes.
        let text = "\"€😀é\" x";
        let f = file(text);
        let x = text.find('x').expect("x present") as u32;
        // bytes before x: `"`=1, `€`=3, `😀`=4, `é`=2, `"`=1, space=1, total 12
        assert_eq!(x, 12);
        // characters before x: `"` `€` `😀` `é` `"` space = 6, so x is column 7
        assert_eq!(f.line_col(x), lc(1, 7));
    }

    #[test]
    fn columns_on_second_line_with_multibyte_first_line() {
        let f = file("€€€\nab");
        // line 1 is 9 bytes + \n, so line 2 starts at byte 10
        assert_eq!(f.line_col(10), lc(2, 1));
        assert_eq!(f.line_col(11), lc(2, 2));
    }

    #[test]
    fn offset_inside_multibyte_char_moves_back() {
        let f = file("a€b"); // a=0, € = 1..4, b=4
        assert_eq!(f.line_col(1), lc(1, 2));
        assert_eq!(f.line_col(2), lc(1, 2)); // inside €, clamped to its start
        assert_eq!(f.line_col(3), lc(1, 2));
        assert_eq!(f.line_col(4), lc(1, 3));
    }

    #[test]
    fn offset_past_end_is_clamped() {
        let f = file("ab\ncd");
        assert_eq!(f.line_col(999), lc(2, 3));
    }

    #[test]
    fn slice_returns_covered_text() {
        let f = file("let total = 0");
        assert_eq!(f.slice(Span::new(FileId(0), 4, 9)), Some("total"));
        assert_eq!(f.slice(Span::new(FileId(0), 4, 99)), None);
    }

    #[test]
    fn slice_rejects_non_char_boundaries() {
        let f = file("a€b");
        assert_eq!(f.slice(Span::new(FileId(0), 1, 4)), Some("€"));
        assert_eq!(f.slice(Span::new(FileId(0), 2, 4)), None);
    }

    #[test]
    fn source_map_hands_out_sequential_ids() {
        let mut map = SourceMap::new();
        let a = map.add(file("a"));
        let b = map.add(file("b"));
        assert_eq!(a, FileId(0));
        assert_eq!(b, FileId(1));
        assert_eq!(map.get(b).map(|f| f.text()), Some("b"));
        assert!(map.get(FileId(7)).is_none());
    }
}
