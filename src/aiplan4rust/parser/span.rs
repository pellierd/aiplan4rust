use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a span in the source code with information about the start and end positions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    start: usize,
    end: usize,
    begin_line: usize,
    begin_column: usize,
    end_line: usize,
    end_column: usize,
}

impl Span {
    /// Creates a new `Span` with the given start and end positions.
    ///
    /// This function sets the `begin_line`, `begin_column`, `end_line`, and `end_column`
    /// to `usize::MAX` by default. `usize::MAX` is used as a sentinel value to represent
    /// an undefined or uninitialized position. This value indicates that the span's
    /// line and column positions have not been set or are not available.
    ///
    /// # Parameters
    /// - `start`: The start position of the span (index in the text).
    /// - `end`: The end position of the span (index in the text).
    ///
    /// # Returns
    /// Returns a new `Span` object with the provided start and end positions.
    pub fn new(start: usize, end: usize) -> Self {
        Span {
            start,
            end,
            begin_line: usize::MAX,
            begin_column: usize::MAX,
            end_line: usize::MAX,
            end_column: usize::MAX,
        }
    }

    /// Gets the start position of the span (index in the text).
    ///
    /// # Returns
    /// Returns the `start` index of the span.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Gets the end position of the span (index in the text).
    ///
    /// # Returns
    /// Returns the `end` index of the span.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Gets the line where the span begins.
    ///
    /// # Returns
    /// Returns the line number where the span starts.
    pub fn begin_line(&self) -> usize {
        self.begin_line
    }

    /// Gets the column where the span begins.
    ///
    /// # Returns
    /// Returns the column number where the span starts.
    pub fn begin_column(&self) -> usize {
        self.begin_column
    }

    /// Gets the line where the span ends.
    ///
    /// # Returns
    /// Returns the line number where the span ends.
    pub fn end_line(&self) -> usize {
        self.end_line
    }

    /// Gets the column where the span ends.
    ///
    /// # Returns
    /// Returns the column number where the span ends.
    pub fn end_column(&self) -> usize {
        self.end_column
    }

    /// Sets the start position of the span.
    ///
    /// # Parameters
    /// - `start`: The new start position of the span (index in the text).
    pub fn set_start(&mut self, start: usize) {
        self.start = start;
    }

    /// Sets the end position of the span.
    ///
    /// # Parameters
    /// - `end`: The new end position of the span (index in the text).
    pub fn set_end(&mut self, end: usize) {
        self.end = end;
    }

    /// Sets the line where the span begins.
    ///
    /// # Parameters
    /// - `begin_line`: The new line number where the span starts.
    pub fn set_begin_line(&mut self, begin_line: usize) {
        self.begin_line = begin_line;
    }

    /// Sets the column where the span begins.
    ///
    /// # Parameters
    /// - `begin_column`: The new column number where the span starts.
    pub fn set_begin_column(&mut self, begin_column: usize) {
        self.begin_column = begin_column;
    }

    /// Sets the line where the span ends.
    ///
    /// # Parameters
    /// - `end_line`: The new line number where the span ends.
    pub fn set_end_line(&mut self, end_line: usize) {
        self.end_line = end_line;
    }

    /// Sets the column where the span ends.
    ///
    /// # Parameters
    /// - `end_column`: The new column number where the span ends.
    pub fn set_end_column(&mut self, end_column: usize) {
        self.end_column = end_column;
    }

    /// Gets the starting position of the span as a tuple `(line, column)`.
    ///
    /// # Returns
    /// Returns a tuple representing the start position `(begin_line, begin_column)`.
    pub fn start_position(&self) -> (usize, usize) {
        (self.begin_line, self.begin_column)
    }

    /// Gets the ending position of the span as a tuple `(line, column)`.
    ///
    /// # Returns
    /// Returns a tuple representing the end position `(end_line, end_column)`.
    pub fn end_position(&self) -> (usize, usize) {
        (self.end_line, self.end_column)
    }

    /// Gets the start and end positions of the span as a tuple `(start, end)`.
    ///
    /// # Returns
    /// Returns a tuple representing the span's indices `(start, end)`.
    pub fn position(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[start={}, end={}", self.start, self.end)?;

        if self.begin_line != usize::MAX {
            write!(f, ", begin_line={}", self.begin_line)?;
        }
        if self.begin_column != usize::MAX {
            write!(f, ", begin_column={}", self.begin_column)?;
        }
        if self.end_line != usize::MAX {
            write!(f, ", end_line={}", self.end_line)?;
        }
        if self.end_column != usize::MAX {
            write!(f, ", end_column={}", self.end_column)?;
        }

        write!(f, "]")
    }
}
