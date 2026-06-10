use std::fmt;
use serde::{Deserialize, Serialize};

/// Represents a contiguous range of indices within a shared value storage.
///
/// `TypeRange` defines a half-open interval `[start, end[` used to identify
/// which slice of a global `ObjectId` vector belongs to a specific type.
///
/// This structure is central to the `ValueRegistry`'s ability to provide
/// $O(1)$ access to type domains while maintaining memory efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TypeRange {
    /// The starting index of the range (inclusive).
    start: usize,
    /// The ending index of the range (exclusive).
    end: usize,
}

impl TypeRange {
    /// Creates a new `TypeRange` with the specified bounds.
    ///
    /// # Parameters
    /// - `start`: The beginning of the range (inclusive).
    /// - `end`: The end of the range (exclusive).
    ///
    /// # Example
    /// ```
    /// let range = TypeRange::new(0, 10);
    /// assert_eq!(range.len(), 10);
    /// ```
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the starting index (inclusive).
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the ending index (exclusive).
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the number of elements covered by this range.
    ///
    /// This method uses saturating subtraction to prevent underflow if
    /// the range is malformed.
    #[inline]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns `true` if the range contains no elements.
    ///
    /// A range is considered empty if the start index is greater than
    /// or equal to the end index.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl fmt::Display for TypeRange {
    /// Formats the range using the standard mathematical interval notation.
    ///
    /// Output format: `[start, end]`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
    }
}
