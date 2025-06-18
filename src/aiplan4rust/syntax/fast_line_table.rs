use crate::aiplan4rust::syntax::Span;

/// A fast line table for efficiently mapping byte offsets to line and column numbers.
/// It uses a coarse index to accelerate lookups.
///
/// # Fields
/// - `line_starts`: A vector storing the starting byte offset of each line.
/// - `coarse_index`: A vector storing precomputed offsets and their corresponding line numbers
///   at intervals of `k` lines to speed up lookups.
/// - `k`: The interval for the pre-index (determines how frequently the coarse index stores values).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FastLineTable {
    line_starts: Vec<usize>,
    coarse_index: Vec<(usize, usize)>, // (Offset, Line number) every K lines
}

impl FastLineTable {
    /// Average line length in bytes, used for initial capacity estimation.
    ///
    /// Public so it can be used elsewhere if needed.
    pub const AVG_LINE_LENGTH: usize = 60;

    /// Creates a new `FastLineTable` with an automatically chosen interval `k`.
    ///
    /// The interval is computed based on the number of lines in the source,
    /// aiming to balance lookup speed and memory usage.
    ///
    /// # Arguments
    /// - `source`: The full input source code as a string slice.
    ///
    /// # Returns
    /// A `FastLineTable` with dynamically tuned indexing.
    pub fn new(source: &str) -> Self {
        let total_lines = bytecount::count(source.as_bytes(), b'\n') + 1;
        let k = std::cmp::max(10, total_lines / 100);
        Self::with_capacity(source, k)
    }

    /// Creates a new `FastLineTable` using a manually specified indexing interval `k`.
    ///
    /// A lower `k` gives faster lookups but increases memory usage. A higher `k` reduces
    /// memory usage but may slow down lookup times. Typical values range from 50 to 500.
    ///
    /// # Arguments
    /// - `source`: The full input source code.
    /// - `k`: The interval between entries in the coarse index.
    ///
    /// # Returns
    /// A new instance of `FastLineTable`.
    pub fn with_capacity(source: &str, k: usize) -> Self {
        // Estimate capacity based on source length and average line length
        let estimated_lines = source.len() / Self::AVG_LINE_LENGTH + 1;
        let mut line_starts = Vec::with_capacity(estimated_lines);
        line_starts.push(0);
        let mut coarse_index = Vec::new();

        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                let line_number = line_starts.len() + 1;
                line_starts.push(i + 1);

                if line_number % k == 0 {
                    coarse_index.push((i + 1, line_number));
                }
            }
        }

        Self {
            line_starts,
            coarse_index,
        }
    }

    /// Calculates the span (start and end positions) of a substring within the source text,
    /// including line and column information for both start and end positions.
    ///
    /// # Arguments
    ///
    /// * `start` - The byte index in the source string where the span starts.
    /// * `end` - The byte index in the source string where the span ends.
    /// * `source` - The entire source string from which the span is derived.
    ///
    /// # Returns
    ///
    /// Returns a `Span` struct containing the start and end byte indices along with
    /// corresponding line and column numbers within the source.
    ///
    /// # Notes
    ///
    /// This function iterates over the source string character by character,
    /// updating line and column counts, and stops once the end index is reached.
    /// It also handles the edge case where `end` equals the length of the source.
    pub fn get_span(&self, start: usize, end: usize) -> Span {
        let (sl, sc) = self.get_position(start);
        let (el, ec) = self.get_position(end);

        let mut span = Span::new(start, end);
        span.set_begin_line(sl);
        span.set_begin_column(sc);
        span.set_end_line(el);
        span.set_end_column(ec);

        span
    }

    /// Retrieves the line and column number corresponding to a given byte offset.
    ///
    /// # Arguments
    /// - `offset`: The byte offset in the source string.
    ///
    /// # Returns
    /// A tuple `(line_number, column_number)`, where:
    /// - `line_number` is the 1-based index of the line.
    /// - `column_number` is the 1-based index of the column within the line.
    pub fn get_position(&self, offset: usize) -> (usize, usize) {
        // Clip offset to maximum valid position (end of source)
        let max_offset = self.line_starts.last().copied().unwrap_or(0);
        let offset = offset.min(max_offset);

        // Fast lookup using the coarse index (binary search)
        let mut approx_line = match self
            .coarse_index
            .binary_search_by_key(&offset, |&(pos, _)| pos)
        {
            Ok(idx) => self.coarse_index[idx].1, // Exact match found
            Err(idx) => {
                if idx == 0 {
                    1 // If the offset is before the first indexed entry, start from line 1
                } else {
                    self.coarse_index[idx - 1].1 // Start from the nearest coarse index entry
                }
            }
        };

        // Fine-tune the search with a linear scan from the approximate starting point
        while approx_line < self.line_starts.len() && self.line_starts[approx_line] <= offset {
            approx_line += 1;
        }

        // Compute the column by subtracting the line start offset from the given offset
        let line_start = self.line_starts[approx_line - 1];
        (approx_line, offset - line_start + 1)
    }
}
