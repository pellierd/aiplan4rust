//! FastLineTable module for efficient mapping of byte offsets to line and column numbers.
//!
//! Parsing libraries like LALRPOP produce spans as byte offsets relative to the start of the source,
//! but do not provide line and column information directly.
//! This module addresses that gap by providing a mechanism to efficiently compute
//! the corresponding line and column numbers from byte offsets.
//!
//! [`FastLineTable`] maintains a vector of line start offsets. This allows for $O(\log n)$
//! translation of any byte offset into (line, column) coordinates using **binary search**.
//! This efficiency is essential for generating user-friendly error messages and diagnostics
//! without performance degradation, even on very large source files (e.g., 30MB+).
//!
//! # Key Components
//! - [`FastLineTable`]: Main struct for managing source text indexing and position lookups.
//!
//! # Usage
//! Construct a `FastLineTable` from a source string, then use
//! [`get_position(offset)`](FastLineTable::get_position) or
//! [`get_span(start, end)`](FastLineTable::get_span) to obtain human-readable positions.
//!
//! # Example
//! ```rust
//! use crate::fast_line_table::FastLineTable;
//!
//! let source = "Hello\nWorld\nRust";
//! let flt = FastLineTable::new(source);
//! let (line, col) = flt.get_position(7); // Corresponds to line 2, column 2
//! assert_eq!((line, col), (2, 2));
//! ```

use crate::aiplan4rust::compiler::syntax::Span;

/// A fast line table for efficiently mapping byte offsets to line and column numbers.
///
/// This structure pre-computes the byte offsets for the start of every line in the source text,
/// allowing for high-performance translation from raw byte positions to human-readable
/// (line, column) coordinates.
///
/// # Implementation Details
/// Lookups are performed using **binary search** over the `line_starts` vector,
/// ensuring $O(\log n)$ time complexity, where $n$ is the total number of lines.
/// This approach is highly scalable for massive files (e.g., 30MB+ benchmarks)
/// where linear scanning would be prohibitive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FastLineTable {
    /// Stores the byte offset of the beginning of each line.
    /// Index 0 corresponds to line 1, index 1 to line 2, and so on.
    line_starts: Vec<usize>,
}

impl FastLineTable {
    /// Average line length in bytes, used for initial capacity estimation.
    ///
    /// Public so it can be used elsewhere if needed.
    pub const AVG_LINE_LENGTH: usize = 80;

    /// Creates a new `FastLineTable` by scanning the source for line boundaries.
    ///
    /// This constructor pre-computes the byte offset of the start of every line,
    /// enabling $O(\log n)$ position lookups via binary search.
    ///
    /// # Arguments
    /// - `source`: The full input source code as a string slice.
    ///
    /// # Returns
    /// A new instance of `FastLineTable`.
    pub fn new(source: &str) -> Self {
        // Estimate capacity based on source length and average line length (80 chars)
        // to minimize reallocations of the Vec.
        let estimated_lines = source.len() / Self::AVG_LINE_LENGTH + 1;
        let mut line_starts = Vec::with_capacity(estimated_lines);

        // The first line always starts at offset 0
        line_starts.push(0);

        // Scan for newline characters
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                // The next line starts immediately after the '\n'
                line_starts.push(i + 1);
            }
        }

        Self { line_starts }
    }

    /// Calculates the span (start and end positions) of a substring within the source text,
    /// including line and column information for both start and end positions.
    ///
    /// # Arguments
    ///
    /// * `start` - The byte index in the source string where the span starts.
    /// * `end` - The byte index in the source string where the span ends.
    ///
    /// # Returns
    ///
    /// Returns a `Span` struct containing the start and end byte indices along with
    /// corresponding line and column numbers within the source.
    ///
    /// # Notes
    ///
    /// This function leverages binary search via `get_position` to resolve line and
    /// column numbers in $O(\log n)$ time, making it suitable for frequent calls
    /// during AST initialization even on massive source files.
    pub fn get_span(&self, start: usize, end: usize) -> Span {
        let (sl, sc) = self.get_position(start);
        let (el, ec) = self.get_position(end);

        let mut span = Span::new(start, end);
        span.set_start_line(sl);
        span.set_start_column(sc);
        span.set_end_line(el);
        span.set_end_column(ec);

        span
    }

    /// Retrieves the line and column number corresponding to a given byte offset.
    ///
    /// This method uses a binary search over the precomputed line start offsets,
    /// providing an $O(\log n)$ lookup time where $n$ is the number of lines.
    /// This is highly efficient even for very large files (e.g., several megabytes).
    ///
    /// # Arguments
    /// * `offset` - The byte offset in the source string.
    ///
    /// # Returns
    /// A tuple `(line_number, column_number)`, where:
    /// - `line_number` is the 1-based index of the line.
    /// - `column_number` is the 1-based index of the column within the line.
    ///
    /// # Complexity
    /// $O(\log(\text{number\_of\_lines}))$
    pub fn get_position(&self, offset: usize) -> (usize, usize) {
        if self.line_starts.is_empty() {
            return (1, 1);
        }

        // Ensure the offset does not exceed the last valid position in the source
        let max_offset = *self.line_starts.last().unwrap_or(&0);
        let offset = offset.min(max_offset);

        // Perform a binary search to find the line containing the offset.
        // - Ok(idx): The offset matches exactly the start of a line.
        // - Err(idx): The offset is within the line starting at idx - 1.
        let line_idx = self
            .line_starts
            .binary_search(&offset)
            .unwrap_or_else(|idx| idx - 1);

        let line_start = self.line_starts[line_idx];

        // Convert 0-based index to 1-based line and calculate 1-based column
        (line_idx + 1, offset - line_start + 1)
    }
}
