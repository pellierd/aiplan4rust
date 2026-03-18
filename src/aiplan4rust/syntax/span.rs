//! Module providing the `Span` structure for representing ranges within source code.
//!
//! A `Span` describes a contiguous region in the input text, including both raw offsets
//! and (optional) line and column information. This is useful for error reporting,
//! syntax highlighting, and other tooling that needs to pinpoint precise locations.
//!
//! # Components
//! - [`Span`]: Represents a region of text with start and end offsets, lines, and columns.
//!
//! This module is typically used internally by the parser and lexer.

use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// Represents a span in the source code with information about the start and end positions.
///
/// A `Span` includes:
/// - the raw start and end indices in the input text,
/// - the line and column where the span begins and ends (optional; defaults to `usize::MAX` if unset).
///
/// This structure enables precise tracking of text regions during parsing or lexing.
#[derive(Debug, Clone, Default, PartialEq, Copy, Eq, Hash, Serialize, Deserialize)]
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
    /// Line and column information is initialized to `usize::MAX` as a sentinel
    /// value indicating they are unset.
    ///
    /// # Parameters
    /// - `start`: The start offset in the input text.
    /// - `end`: The end offset in the input text.
    ///
    /// # Returns
    /// A new `Span` instance.
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            begin_line: usize::MAX,
            begin_column: usize::MAX,
            end_line: usize::MAX,
            end_column: usize::MAX,
        }
    }

    /// Returns the start offset of the span.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end offset of the span.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the line where the span begins.
    pub fn begin_line(&self) -> usize {
        self.begin_line
    }

    /// Returns the column where the span begins.
    pub fn begin_column(&self) -> usize {
        self.begin_column
    }

    /// Returns the line where the span ends.
    pub fn end_line(&self) -> usize {
        self.end_line
    }

    /// Returns the column where the span ends.
    pub fn end_column(&self) -> usize {
        self.end_column
    }

    /// Sets the start offset of the span.
    ///
    /// # Parameters
    /// - `start`: The new start offset.
    pub fn set_start(&mut self, start: usize) {
        self.start = start;
    }

    /// Sets the end offset of the span.
    ///
    /// # Parameters
    /// - `end`: The new end offset.
    pub fn set_end(&mut self, end: usize) {
        self.end = end;
    }

    /// Sets the line where the span begins.
    ///
    /// # Parameters
    /// - `begin_line`: The line number where the span starts.
    pub fn set_start_line(&mut self, begin_line: usize) {
        self.begin_line = begin_line;
    }

    /// Sets the column where the span begins.
    ///
    /// # Parameters
    /// - `begin_column`: The column number where the span starts.
    pub fn set_start_column(&mut self, begin_column: usize) {
        self.begin_column = begin_column;
    }

    /// Sets the line where the span ends.
    ///
    /// # Parameters
    /// - `end_line`: The line number where the span ends.
    pub fn set_end_line(&mut self, end_line: usize) {
        self.end_line = end_line;
    }

    /// Sets the column where the span ends.
    ///
    /// # Parameters
    /// - `end_column`: The column number where the span ends.
    pub fn set_end_column(&mut self, end_column: usize) {
        self.end_column = end_column;
    }

    /// Returns the starting line and column as a tuple `(line, column)`.
    pub fn start_position(&self) -> (usize, usize) {
        (self.begin_line, self.begin_column)
    }

    /// Returns the ending line and column as a tuple `(line, column)`.
    pub fn end_position(&self) -> (usize, usize) {
        (self.end_line, self.end_column)
    }

    /// Returns the start and end offsets as a tuple `(start, end)`.
    pub fn position(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{};{};{};{};{};{}]",
            self.start,
            self.end,
            self.begin_line,
            self.begin_column,
            self.end_line,
            self.end_column
        )
    }
}
