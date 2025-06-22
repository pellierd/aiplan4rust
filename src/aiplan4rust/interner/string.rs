use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::syntax::lexer::token::{DURATION_VARIABLE, NUMBER_TYPE, OBJECT_TYPE, TOTAL_TIME};

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

    /// The interned identifier for the reserved string `"object"`.
    ///
    /// This constant assumes that the string `"object"` is interned at index `0`
    /// during the initialization of the [`StringInterner`] via [`intern_reserved`].
    /// It must match the insertion order used in [`StringInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_OBJECT).unwrap(), "object");
    /// ```
    pub const IDENT_OBJECT: Ident = Ident::new(0);

    /// The interned identifier for the reserved string `"number"`.
    ///
    /// This constant assumes that the string `"number"` is interned at index `1`
    /// during the initialization of the [`StringInterner`] via [`intern_reserved`].
    /// It must match the insertion order used in [`StringInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_NUMBER).unwrap(), "number");
    /// ```
    pub const IDENT_NUMBER: Ident = Ident::new(1);

    /// The interned identifier for the reserved string `"duration_variable"`.
    ///
    /// This constant assumes that the string `"duration_variable"` is interned at index `2`
    /// during the initialization of the [`StringInterner`] using [`intern_reserved`].
    ///
    /// Ensure this index matches the insertion order defined in [`StringInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(
    ///     interner.expect_str(StringInterner::IDENT_DURATION_VARIABLE).unwrap(),
    ///     "duration_variable"
    /// );
    /// ```
    pub const IDENT_DURATION_VARIABLE: Ident = Ident::new(2);

    /// The interned identifier for the reserved string `"total_time"`.
    ///
    /// This constant assumes that the string `"total_time"` is interned at index `3`
    /// during the initialization of the [`StringInterner`] using [`intern_reserved`].
    ///
    /// It is important that this constant's value matches the insertion order
    /// of reserved strings in [`StringInterner::new()`]. Changing that order without
    /// updating this constant will result in incorrect behavior.
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(
    ///     interner.expect_str(StringInterner::IDENT_TOTAL_TIME).unwrap(),
    ///     "total_time"
    /// );
    /// ```
    pub const IDENT_TOTAL_TIME: Ident = Ident::new(3);

    /// Creates a new `StringInterner` with reserved strings pre-interned.
    ///
    /// This constructor initializes an empty string pool and inserts a predefined set
    /// of reserved strings (`"object"`, `"number"`, `"total_time"`) at fixed indices.
    /// These strings are interned using [`intern_reserved`] in a specific order that must
    /// match the declaration of their corresponding [`Ident`] constants:
    ///
    /// - `IDENT_OBJECT` → `"object"` → index 0
    /// - `IDENT_NUMBER` → `"number"` → index 1
    /// - 'IDENT_DURATION_VARIABLE` → `"duration_variable"` → index 2
    /// - `IDENT_TOTAL_TIME` → `"total_time"` → index 3
    ///
    /// This setup guarantees stable identifiers for these known strings throughout
    /// the lifetime of the interner.
    ///
    /// # Returns
    /// A new [`StringInterner`] instance with reserved strings already interned.
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_OBJECT).unwrap(), "object");
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_NUMBER).unwrap(), "number");
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_TOTAL_TIME).unwrap(), "total_time");
    /// ```
    pub fn new() -> Self {
        let mut interner = StringInterner {
            string_pool: Vec::new(),
            string_index: HashMap::new(),
        };

        // Always intern these in the same order as their constant Ident declarations
        interner.intern_reserved(OBJECT_TYPE);       // index 0
        interner.intern_reserved(NUMBER_TYPE);       // index 1
        interner.intern_reserved(DURATION_VARIABLE); // index 2
        interner.intern_reserved(TOTAL_TIME);        // index 3


        interner
    }

    /// Interns a statically known string without checking for duplicates.
    ///
    /// This method is intended to be used internally to insert predefined
    /// strings (such as reserved keywords or type names) into the interner
    /// at a fixed position. It **does not** check whether the string already
    /// exists in the pool — calling this function multiple times with the same
    /// string will result in duplicate entries.
    ///
    /// The input string must have `'static` lifetime and is assumed to be
    /// unique in the context of interning. It is inserted into the internal
    /// string pool and assigned the next available index.
    ///
    /// # Arguments
    ///
    /// * `s` - A string slice with `'static` lifetime to be interned directly.
    ///
    /// # Returns
    ///
    /// * `Ident` - A unique identifier associated with the given string.
    ///
    /// # Safety
    ///
    /// This function must only be called during controlled initialization (e.g., in `StringInterner::new`)
    /// and must preserve the order of insertion if corresponding `Ident` constants are declared.
    ///
    /// # Example
    ///
    /// ```rust
    /// const TYPE_OBJECT: &str = "object";
    ///
    /// let mut interner = StringInterner::new();
    /// let ident = interner.intern_reserved(TYPE_OBJECT);
    /// assert_eq!(ident.as_usize(), 0);
    /// ```
    fn intern_reserved(&mut self, s: &'static str) -> Ident {
        let idx = self.string_pool.len();
        self.string_pool.push(Box::from(s));
        self.string_index.insert(s, idx);
        Ident::new(idx)
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
    pub fn resolve(&self, ident: Ident) -> Option<&str> {
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
    pub fn try_resolve(&self, ident: Ident) -> Result<&str, ParserInternalError> {
        self.resolve(ident).ok_or_else(|| {
            ParserInternalError::new(format!(
                "Invalid Ident {}: out of bounds for interner size {}",
                ident.as_usize(),
                self.string_pool.len()
            ))
        })
    }

    // Lookup an interned string and get its Ident if it exists (no insertion).
    pub fn lookup(&self, s: &str) -> Option<Ident> {
        self.string_index.get(s).copied().map(Ident::new)
    }

    /// Returns an iterator over the interned `Ident`s (the indices).
    /// Returns an iterator over all interned identifiers (`Ident`).
    pub fn keys(&self) -> impl Iterator<Item = Ident> + '_ {
        (0..self.string_pool.len()).map(Ident::new)
    }

    /// Returns an iterator over interned strings (`&str`).
    pub fn values(&self) -> impl Iterator<Item = &str> + '_ {
        self.string_pool.iter().map(|s| s.as_ref())
    }

    /// Returns an iterator over `(Ident, &str)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (Ident, &str)> + '_ {
        self.string_pool
            .iter()
            .enumerate()
            .map(|(i, s)| (Ident::new(i), s.as_ref()))
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
