use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::SeqAccess;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Ident;

/// A `StringInterner` is a data structure that stores unique strings efficiently
/// by assigning each string a unique numeric index.
///
/// This reduces memory usage by avoiding duplicate string allocations and
/// enables fast equality checks and lookups via the numeric indices.
///
/// Interned strings are stored in a pool as owned boxed strings (`Box<str>`) that
/// are leaked to obtain `'static` lifetimes, ensuring their references remain
/// valid for the lifetime of the interner.
///
/// The interner maintains:
/// - A `string_pool` vector holding all interned strings in order.
/// - A `string_index` hashmap mapping each interned string slice (`&'static str`)
///   to its unique index.
///
/// # Features
/// - Efficient string interning without cloning on repeated strings.
/// - Retrieval of strings by their index.
/// - Serialization and deserialization support using Serde:
///   - Only the string pool is serialized (as `Vec<Box<str>>`).
///   - On deserialization, the string index is reconstructed from the pool.
///
/// # Usage
/// ```rust
/// let mut interner = StringInterner::new();
/// let idx1 = interner.intern("hello".to_string());
/// let idx2 = interner.intern("world".to_string());
/// assert_eq!(interner.get_str(idx1), Some("hello"));
/// assert_eq!(interner.get_str(idx2), Some("world"));
/// ```
///
/// # Notes
/// Interned strings are leaked to provide `'static` references, so the memory
/// is not reclaimed until the interner is dropped.
///
/// The design assumes the interner is long-lived or used in contexts where
/// leaked memory is acceptable.
///
/// # Display
/// The interner implements `Display` trait to show all interned strings
/// joined by commas, useful for debugging.
///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringInterner {
    /// Pool holding all interned strings as owned boxed strings.
    string_pool: Vec<Box<str>>,

    /// Map from interned `'static` string slices to their unique index.
    string_index: HashMap<&'static str, usize>,
}

impl StringInterner {
    /// Creates a new, empty `StringInterner`.
    ///
    /// # Returns
    /// A fresh instance of `StringInterner` with no stored strings.
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// ```
    pub fn new() -> Self {
        Self {
            string_pool: Vec::new(),
            string_index: HashMap::new(),
        }
    }

    /// Interns the given string and returns an `Ident` representing it.
    ///
    /// If the string is already interned, returns its existing `Ident`.
    /// Otherwise, adds the string to the pool and returns a new `Ident`.
    pub fn intern(&mut self, s: String) -> Ident {
        if let Some(&idx) = self.string_index.get(s.as_str()) {
            return Ident::new(idx);
        }

        let boxed: Box<str> = s.into_boxed_str();
        let static_str: &'static str = Box::leak(boxed);

        let idx = self.string_pool.len();
        self.string_pool.push(static_str.into());
        self.string_index.insert(static_str, idx);

        Ident::new(idx)
    }

    /// Retrieves the interned string by its `Ident`.
    ///
    /// # Arguments
    /// * `ident` - The `Ident` representing the index of the interned string.
    ///
    /// # Returns
    /// * `Some(&str)` if the `Ident` is valid and corresponds to an interned string.
    /// * `None` if the `Ident` is out of bounds or invalid.
    ///
    /// # Example
    /// ```rust
    /// let mut interner = StringInterner::new();
    /// let ident = interner.intern("hello".to_string());
    /// assert_eq!(interner.get_str(ident), Some("hello"));
    /// assert_eq!(interner.get_str(Ident::new(9999)), None);
    /// ```
    pub fn get_str(&self, ident: Ident) -> Option<&str> {
        self.string_pool.get(ident.as_usize()).map(|s| s.as_ref())
    }
    /// Returns the interned string associated with the given `Ident`.
    ///
    /// If the `Ident` is valid and corresponds to a stored string, returns
    /// `Ok(&str)` referencing the interned string slice.
    ///
    /// If the `Ident` is invalid (e.g., out of bounds), returns a
    /// `ParserInternalError` with a descriptive error message.
    ///
    /// # Arguments
    ///
    /// * `ident` - The `Ident` representing the index of the interned string.
    ///
    /// # Errors
    ///
    /// Returns `Err(ParserInternalError)` if the `Ident` is not valid for this interner.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use your_crate::{StringInterner, Ident, ParserInternalError};
    /// let mut interner = StringInterner::new();
    /// let id = interner.intern("example".to_string());
    /// assert_eq!(interner.expect_str(id).unwrap(), "example");
    ///
    /// let invalid_id = Ident::new(9999);
    /// assert!(interner.expect_str(invalid_id).is_err());
    /// ```
    ///
    pub fn expect_str(&self, ident: Ident) -> Result<&str, ParserInternalError> {
        self.get_str(ident).ok_or_else(|| {
            ParserInternalError::new(format!(
                "Invalid Ident {}: out of bounds for interner size {}",
                ident.as_usize(),
                self.string_pool.len()
            ))
        })
    }
}

impl Serialize for StringInterner {
    /// Serializes only the string pool (`Vec<Box<str>>`) using Serde.
    ///
    /// The `string_index` is not serialized as it can be reconstructed.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.string_pool.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StringInterner {
    /// Deserializes the `string_pool` from a `Vec<String>` and reconstructs
    /// the `string_index` mapping.
    ///
    /// This ensures that after deserialization, the interner has both the pool
    /// and the index ready for lookups.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let vec: Vec<String> = Vec::deserialize(deserializer)?;

        let mut string_pool = Vec::with_capacity(vec.len());
        let mut string_index = HashMap::with_capacity(vec.len());

        for (idx, s) in vec.into_iter().enumerate() {
            let boxed: Box<str> = s.into_boxed_str();
            let static_str: &'static str = Box::leak(boxed);
            string_index.insert(static_str, idx);
            string_pool.push(static_str.into());
        }

        Ok(Self { string_pool, string_index })
    }
}

impl fmt::Display for StringInterner {
    /// Formats the interner as a table with indices and strings,
    /// each string on its own line for better readability.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "StringInterner {{")?;
        for (idx, s) in self.string_pool.iter().enumerate() {
            writeln!(f, "  [{}]: {}", idx, s)?;
        }
        write!(f, "}}")
    }
}
