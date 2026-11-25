//! # Spans: where a piece of syntax came from
//!
//! A `Span` is a half-open byte range `[start, end)` inside one source file.
//! Every token, AST node, and HIR node carries one, so that any later phase
//! (type errors, effect errors, Z3 counterexamples, fix plans) can point back
//! at the exact characters the user wrote.
//!
//! Three design points worth knowing:
//!
//! 1. **Bytes, not characters.** Rust strings are UTF-8, and slicing a `&str`
//!    takes byte indices. Storing byte offsets makes `&text[start..end]` free
//!    and exact. Human-friendly line and column numbers are *derived* from the
//!    offset only when a diagnostic is rendered (see `source.rs`).
//! 2. **`u32`, not `usize`.** A span is created for every node in the program,
//!    so its size matters. Two `u32`s plus a file id is 12 bytes, versus 24
//!    with `usize` on 64-bit machines. The cost is a 4 GiB limit per source
//!    file, which `SourceFile::new` checks instead of silently overflowing.
//!    (rustc makes the same trade.)
//! 3. **Half-open ranges.** `end` is one past the last byte. This makes the
//!    length simply `end - start`, lets an empty span (`start == end`) mark a
//!    position such as "end of file", and means adjacent spans share a
//!    boundary without overlapping.
//!
//! Spans also carry a `FileId`. In M0 there is only ever one file, but once
//! the module system lands (M2) a single diagnostic may point into several
//! files, and adding the id later would mean touching every node.

/// Identifies one file inside a `SourceMap`. Cheap to copy and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// A half-open byte range `[start, end)` within a single file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// Creates a span. If `start > end` (a compiler bug, never user input),
    /// debug builds panic so the bug is caught in tests; release builds swap
    /// the two so rendering still points somewhere sensible.
    pub fn new(file: FileId, start: u32, end: u32) -> Span {
        debug_assert!(start <= end, "span start {start} is after end {end}");
        if start <= end {
            Span { file, start, end }
        } else {
            Span { file, start: end, end: start }
        }
    }

    /// An empty span marking a single position, such as end of file.
    pub fn point(file: FileId, offset: u32) -> Span {
        Span { file, start: offset, end: offset }
    }

    /// Number of bytes covered.
    pub fn len(self) -> u32 {
        self.end - self.start
    }

    /// True for a zero-width span (a position rather than a range).
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// The smallest span covering both `self` and `other`. Used by the parser
    /// to build a node's span from its first and last child, for example
    /// `a + b` spans from the start of `a` to the end of `b`.
    ///
    /// Both spans must be in the same file; if they are not (a compiler bug),
    /// debug builds panic and release builds return `self` unchanged.
    pub fn to(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file, "cannot join spans from different files");
        if self.file != other.file {
            return self;
        }
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// True if `offset` lies inside this span. An empty span contains nothing.
    pub fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: FileId = FileId(0);

    #[test]
    fn len_and_empty() {
        let s = Span::new(F, 3, 7);
        assert_eq!(s.len(), 4);
        assert!(!s.is_empty());
        assert!(Span::point(F, 5).is_empty());
    }

    #[test]
    fn to_covers_both_in_any_order() {
        let a = Span::new(F, 2, 4);
        let b = Span::new(F, 10, 12);
        assert_eq!(a.to(b), Span::new(F, 2, 12));
        assert_eq!(b.to(a), Span::new(F, 2, 12));
    }

    #[test]
    fn to_with_overlap_and_containment() {
        let outer = Span::new(F, 0, 20);
        let inner = Span::new(F, 5, 6);
        assert_eq!(outer.to(inner), outer);
        assert_eq!(Span::new(F, 0, 5).to(Span::new(F, 3, 9)), Span::new(F, 0, 9));
    }

    #[test]
    fn contains_is_half_open() {
        let s = Span::new(F, 3, 5);
        assert!(!s.contains(2));
        assert!(s.contains(3));
        assert!(s.contains(4));
        assert!(!s.contains(5));
        assert!(!Span::point(F, 3).contains(3));
    }
}
