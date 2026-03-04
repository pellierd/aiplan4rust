//! String interning implementation using a pool and an index map.
//!
//! This module defines the [`SymbolInterner`] struct, which efficiently stores
//! unique strings by assigning each a unique numeric index. It is designed to
//! reduce memory usage and speed up string equality checks by avoiding repeated
//! string allocations and using fast numeric lookups instead.
//!
//! # Interning Mechanism
//!
//! Interned strings are stored in two pools (`ident_string_pool` and `literal_string_pool`) as owned boxed strings (`Box<str>`).
//! These boxed strings are leaked to obtain `'static` lifetime references, which serve
//! as keys in hashmaps (`ident_index_map` and `literal_index_map`) mapping from `&'static str` to unique indices.
//!
//! # Features
//!
//! - Avoids duplicate storage of strings by returning indices for repeated strings.
//! - Enables fast retrieval of interned strings by their indices.
//! - Separates storage for identifier strings and literal strings for clarity and potential optimizations.
//! - Supports serialization and deserialization through Serde:
//!   - On serialization, only the string pools are saved.
//!   - On deserialization, the index maps are rebuilt from the pools.
//!
//! # Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::StringInterner;
//! let mut interner = StringInterner::new();
//! let id1 = interner.intern("hello".to_string());
//! let id2 = interner.intern("world".to_string());
//! assert_eq!(interner.get_str(id1), Some("hello"));
//! assert_eq!(interner.get_str(id2), Some("world"));
//! ```
//!
//! # Memory Considerations
//!
//! Interned strings are leaked to provide `'static` references, meaning memory
//! is not reclaimed until the interner itself is dropped. This design is
//! appropriate for long-lived interners or use cases where leaked memory is acceptable.
//!
//! # Display
//!
//! Implements the `Display` trait to output all interned strings (both identifiers and literals),
//! each string on its own line with its index, which is useful for debugging and inspection.
//!

use std::collections::HashMap;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{LiteralId, SymbolId};
use crate::aiplan4rust::syntax::lexer::token::{DURATION_VARIABLE, NUMBER_TYPE, OBJECT_TYPE, TOTAL_TIME};

/// A `StringInterner` is a data structure that stores unique strings efficiently
/// by assigning each string a unique numeric index.
///
/// This reduces memory usage by avoiding duplicate string allocations and
/// enables fast equality checks and lookups via the numeric indices.
///
/// The interner maintains separate pools and maps for identifiers and literals:
///
/// - `ident_string_pool`: A vector of all interned identifier strings stored as owned boxed strings (`Box<str>`).
/// - `ident_index_map`: A hashmap mapping interned identifier string slices (`&'static str`) to their unique indices.
/// - `literal_string_pool`: A vector of all interned literal strings stored as owned boxed strings.
/// - `literal_index_map`: A hashmap mapping interned literal string slices to their unique indices.
///
/// Interned strings are leaked to obtain `'static` lifetimes for the string slices,
/// which are used as keys in the hashmaps.
///
/// # Features
///
/// - Efficiently avoids duplicate string storage by returning indices for repeated strings.
/// - Allows fast retrieval of interned strings by index.
/// - Supports Serde serialization and deserialization:
///   - Serialization stores only the string pools (`Vec<Box<str>>`).
///   - Deserialization reconstructs the index maps from the pools.
///
/// # Example
///
/// ```rust
/// let mut interner = StringInterner::new();
/// let idx1 = interner.intern("hello".to_string());
/// let idx2 = interner.intern("world".to_string());
/// assert_eq!(interner.get_str(idx1), Some("hello"));
/// assert_eq!(interner.get_str(idx2), Some("world"));
/// ```
///
/// # Memory Note
///
/// The interner leaks memory by design to produce `'static` string slices,
/// so memory is reclaimed only when the interner itself is dropped.
///
/// This is suitable for long-lived interners or contexts where leaking is acceptable.
///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolInterner {
    /// Pool holding all interned identifier strings as owned boxed strings.
    symbol_string_pool: Vec<Box<str>>,

    /// Map from interned identifier `'static` string slices to their unique indices.
    symbol_id_map: HashMap<&'static str, usize>,

    /// Pool holding all interned literal strings as owned boxed strings.
    literal_string_pool: Vec<Box<str>>,

    /// Map from interned literal `'static` string slices to their unique indices.
    literal_id_map: HashMap<&'static str, usize>,
}

impl SymbolInterner {

    /// A placeholder string returned when an interned index cannot be resolved.
    /// This string is guaranteed not to conflict with any valid PDDL identifiers.
    pub const UNKNOWN_INTERNED_STRING: &str = "#UNKNOWN";

    /// The interned identifier for the reserved string `"object"`.
    ///
    /// This constant assumes that the string `"object"` is interned at index `0`
    /// during the initialization of the [`SymbolInterner`] via [`intern_reserved`].
    /// It must match the insertion order used in [`SymbolInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::OBJECT_SYMBOL_ID).unwrap(), "object");
    /// ```
    pub const OBJECT_SYMBOL_ID: SymbolId = SymbolId::new(0);

    /// The interned identifier for the reserved string `"number"`.
    ///
    /// This constant assumes that the string `"number"` is interned at index `1`
    /// during the initialization of the [`SymbolInterner`] via [`intern_reserved`].
    /// It must match the insertion order used in [`SymbolInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::NUMBER_SYMBOL_ID).unwrap(), "number");
    /// ```
    pub const NUMBER_SYMBOL_ID: SymbolId = SymbolId::new(1);

    /// The interned identifier for the reserved string `"duration_variable"`.
    ///
    /// This constant assumes that the string `"duration_variable"` is interned at index `2`
    /// during the initialization of the [`SymbolInterner`] using [`intern_reserved`].
    ///
    /// Ensure this index matches the insertion order defined in [`SymbolInterner::new()`].
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(
    ///     interner.expect_str(StringInterner::DURATION_VARIABLE_SYMBOL_ID).unwrap(),
    ///     "duration_variable"
    /// );
    /// ```
    pub const DURATION_VARIABLE_SYMBOL_ID: SymbolId = SymbolId::new(2);

    /// The interned identifier for the reserved string `"total_time"`.
    ///
    /// This constant assumes that the string `"total_time"` is interned at index `3`
    /// during the initialization of the [`SymbolInterner`] using [`intern_reserved`].
    ///
    /// It is important that this constant's value matches the insertion order
    /// of reserved strings in [`SymbolInterner::new()`]. Changing that order without
    /// updating this constant will result in incorrect behavior.
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(
    ///     interner.expect_str(StringInterner::TOTAL_TIME_SYMBOL_ID).unwrap(),
    ///     "total_time"
    /// );
    /// ```
    pub const TOTAL_TIME_SYMBOL_ID: SymbolId = SymbolId::new(3);

    /// Creates a new `StringInterner` with reserved strings pre-interned.
    ///
    /// This constructor initializes an empty string pool and inserts a predefined set
    /// of reserved strings (`"object"`, `"number"`, `"total_time"`) at fixed indices.
    /// These strings are interned using [`intern_reserved`] in a specific order that must
    /// match the declaration of their corresponding [`SymbolId`] constants:
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
    /// A new [`SymbolInterner`] instance with reserved strings already interned.
    ///
    /// # Example
    /// ```rust
    /// let interner = StringInterner::new();
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_OBJECT).unwrap(), "object");
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_NUMBER).unwrap(), "number");
    /// assert_eq!(interner.expect_str(StringInterner::IDENT_TOTAL_TIME).unwrap(), "total_time");
    /// ```
    pub fn new() -> Self {
        let mut interner = SymbolInterner {
            symbol_string_pool: Vec::new(),
            symbol_id_map: HashMap::new(),
            literal_string_pool: Vec::new(),
            literal_id_map: HashMap::new(),
        };

        // Always intern these in the same order as their constant Ident declarations
        interner.intern_reserved_symbol(OBJECT_TYPE);       // index 0
        interner.intern_reserved_symbol(NUMBER_TYPE);       // index 1
        interner.intern_reserved_symbol(DURATION_VARIABLE); // index 2
        interner.intern_reserved_symbol(TOTAL_TIME);        // index 3

        interner
    }

    /// Interns a statically known string without checking for duplicates.
    ///
    /// This method is intended to be used internally to insert predefined
    /// strings (such as reserved keywords or type_checker names) into the interner
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
    fn intern_reserved_symbol(&mut self, s: &'static str) -> SymbolId {
        let idx = self.symbol_string_pool.len();
        self.symbol_string_pool.push(Box::from(s));
        self.symbol_id_map.insert(s, idx);
        SymbolId::new(idx)
    }

    /// Interns the given string and returns a [`SymbolId`] representing it.
    ///
    /// # Case Sensitivity
    ///
    /// To comply with PDDL and HDDL standards (which are case-insensitive), this method
    /// systematically converts the input string to **lowercase** before interning.
    /// For example, interning `"OBJ"`, `"Obj"`, and `"obj"` will all return the same identifier.
    ///
    /// # Behavior
    ///
    /// - If a lowercase version of the string is already interned, returns its existing ID.
    /// - Otherwise, normalizes the string, adds it to the symbol pool, and returns a new ID.
    ///
    /// # Arguments
    ///
    /// * `s` - The string slice or owned string to be interned.
    pub fn intern_symbol<S: AsRef<str>>(&mut self, s: S) -> SymbolId {
        let s = s.as_ref().to_lowercase();
        if let Some(&idx) = self.symbol_id_map.get(s.as_str()) {
            return SymbolId::new(idx);
        }
        let boxed: Box<str> = s.to_string().into_boxed_str();
        let static_str: &'static str = Box::leak(boxed);
        let idx = self.symbol_string_pool.len();
        self.symbol_string_pool.push(static_str.into());
        self.symbol_id_map.insert(static_str, idx);
        SymbolId::new(idx)
    }

    /// Retrieves the interned string by its `SymbolId`.
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
    pub fn resolve_symbol(&self, ident: SymbolId) -> Option<&str> {
        self.symbol_string_pool.get(ident.as_usize()).map(|s| s.as_ref())
    }

    /// Returns the interned string associated with the given `Ident`.
    ///
    /// If the `Ident` is valid and corresponds to a stored string, returns
    /// `Ok(&str)` referencing the interned string slice.
    ///
    /// If the `Ident` is invalid (e.g., out of bounds), returns an
    /// [`InternerError::InvalidIdent`] with a descriptive error message.
    ///
    /// # Arguments
    ///
    /// * `ident` - The `Ident` representing the index of the interned string.
    ///
    /// # Errors
    ///
    /// Returns `Err(InternerError)` if the `Ident` is not valid for this interner.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::aiplan4rust::interner::{StringInterner, Ident, InternerError};
    /// let mut interner = StringInterner::new();
    /// let id = interner.intern("example".to_string());
    /// assert_eq!(interner.try_resolve(id).unwrap(), "example");
    ///
    /// let invalid_id = Ident::new(9999);
    /// assert!(interner.try_resolve(invalid_id).is_err());
    /// ```
    ///
    pub fn try_resolve_symbol(&self, ident: SymbolId) -> Result<&str, InternerError> {
        self.resolve_symbol(ident).ok_or_else(|| {
            InternerError::invalid_ident(ident.as_usize(), self.symbol_string_pool.len())
        })
    }

    /// Lookup an interned string and get its Ident if it exists (no insertion).
    pub fn lookup_symbol(&self, s: &str) -> Option<SymbolId> {
        self.symbol_id_map.get(s).copied().map(SymbolId::new)
    }

    // Look up an interned string and return its ID.
    ///
    /// # Errors
    /// Returns a [`StringInternerError`] if the string has not been interned.
    pub fn try_lookup_symbol(&self, s: &str) -> Result<SymbolId, InternerError> {
        self.symbol_id_map
            .get(s)
            .copied()
            .map(SymbolId::new)
            .ok_or_else(|| InternerError::unknown_ident_string(s))
    }

    /// Returns an iter over the interned `Ident`s (the indices).
    /// Returns an iter over all interned identifiers (`Ident`).
    pub fn symbol_keys(&self) -> impl Iterator<Item =SymbolId> + '_ {
        (0..self.symbol_string_pool.len()).map(SymbolId::new)
    }

    /// Returns an iter over interned strings (`&str`).
    pub fn symbol_values(&self) -> impl Iterator<Item = &str> + '_ {
        self.symbol_string_pool.iter().map(|s| s.as_ref())
    }

    /// Returns an iter over `(Ident, &str)` pairs.
    pub fn iter_symbol_entries(&self) -> impl Iterator<Item = (SymbolId, &str)> + '_ {
        self.symbol_string_pool
            .iter()
            .enumerate()
            .map(|(i, s)| (SymbolId::new(i), s.as_ref()))
    }

    /// Interns a literal string (e.g., numbers, constants) and returns a `Literal`.
    ///
    /// If the string is already interned as a literal, it reuses its index.
    /// Otherwise, it inserts the string into the literal pool and returns a new `Literal`.
    ///
    /// # Example
    /// ```
    /// let mut interner = StringInterner::new();
    /// let lit = interner.intern_literal("42");
    /// assert_eq!(interner.get_literal(lit), Some("42"));
    /// ```
    pub fn intern_literal<S: AsRef<str>>(&mut self, s: S) -> LiteralId {
        let s_ref = s.as_ref();

        if let Some(&idx) = self.literal_id_map.get(s_ref) {
            return LiteralId::new(idx);
        }

        let boxed: Box<str> = s_ref.to_string().into_boxed_str();
        let static_str: &'static str = Box::leak(boxed);

        let idx = self.literal_string_pool.len();
        self.literal_string_pool.push(static_str.into());
        self.literal_id_map.insert(static_str, idx);

        LiteralId::new(idx)
    }

    /// Resolves a literal `Literal` into its string value.
    ///
    /// # Returns
    /// - `Some(&str)` if the index is valid
    /// - `None` otherwise
    pub fn resolve_literal(&self, lit: LiteralId) -> Option<&str> {
        self.literal_string_pool.get(lit.as_usize()).map(|s| s.as_ref())
    }

    /// Try resolving a literal and returns a result.
    ///
    /// # Errors
    /// Returns `Err(InternerError)` if the literal is invalid or out of bounds.
    pub fn try_resolve_literal(&self, literal: LiteralId) -> Result<&str, InternerError> {
        self.resolve_literal(literal).ok_or_else(|| {
            InternerError::invalid_literal(literal.as_usize(), self.literal_string_pool.len())
        })
    }

    /// Looks up a literal string in the interner and returns its identifier if it exists.
    ///
    /// This is a non-allocating operation that returns `None` if the string has
    /// not been previously interned.
    ///
    /// # Arguments
    ///
    /// * `s` - The string slice representing the literal to look up.
    pub fn lookup_literal(&self, s: &str) -> Option<LiteralId> {
        self.literal_id_map.get(s).copied().map(LiteralId::new)
    }

    /// Attempts to look up a literal identifier, returning an error if not found.
    ///
    /// This method is preferred during semantic analysis or linking when a literal
    /// is expected to be already present in the interner.
    ///
    /// # Arguments
    ///
    /// * `s` - The string slice representing the literal to look up.
    ///
    /// # Errors
    ///
    /// Returns [`InternerError::UnknownLiteralString`] if the string is not
    /// registered in the literal pool.
    pub fn try_lookup_literal(&self, s: &str) -> Result<LiteralId, InternerError> {
        self.literal_id_map
            .get(s)
            .copied()
            .map(LiteralId::new)
            .ok_or_else(|| InternerError::unknown_literal_string(s))
    }

    /// Returns an iter over all literal `Literal`s.
    pub fn literal_keys(&self) -> impl Iterator<Item =LiteralId> + '_ {
        (0..self.literal_string_pool.len()).map(LiteralId::new)
    }

    /// Returns an iter over all interned literals (`&str`).
    pub fn literal_values(&self) -> impl Iterator<Item = &str> + '_ {
        self.literal_string_pool.iter().map(|s| s.as_ref())
    }

    /// Returns an iter over `(Literal, &str)` pairs for literals.
    pub fn iter_literal_entries(&self) -> impl Iterator<Item = (LiteralId, &str)> + '_ {
        self.literal_string_pool
            .iter()
            .enumerate()
            .map(|(i, s)| (LiteralId::new(i), s.as_ref()))
    }

}

impl Serialize for SymbolInterner {
    /// Serializes only the interned string pools of the `StringInterner`.
    ///
    /// This implementation serializes the `ident_string_pool` and the
    /// `literal_string_pool`, which hold all interned strings and literals respectively.
    ///
    /// The `ident_index_map` and `literal_index_map` are **not** serialized because
    /// they can be reconstructed from the pools during deserialization.
    ///
    /// # Arguments
    ///
    /// * `serializer` - The serializer instance used to serialize the data.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure of the serialization process.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize both string pools as a tuple
        (&self.symbol_string_pool, &self.literal_string_pool).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SymbolInterner {
    /// Deserializes the two string pools (`ident_string_pool` and `literal_string_pool`)
    /// from a tuple `(Vec<String>, Vec<String>)` and reconstructs their corresponding
    /// index maps (`ident_index_map` and `literal_index_map`).
    ///
    /// # Arguments
    ///
    /// * `deserializer` - The deserializer instance.
    ///
    /// # Returns
    ///
    /// Returns a `Result` wrapping the reconstructed `StringInterner` or an error.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize a tuple of two vectors of strings (for ident and literal pools)
        let (ident_vec, literal_vec): (Vec<String>, Vec<String>) = Deserialize::deserialize(deserializer)?;

        // Helper function to convert Vec<String> into pool and map
        fn build_pool_and_map(vec: Vec<String>) -> (Vec<Box<str>>, HashMap<&'static str, usize>) {
            let mut pool = Vec::with_capacity(vec.len());
            let mut index_map = HashMap::with_capacity(vec.len());

            for (idx, s) in vec.into_iter().enumerate() {
                let boxed: Box<str> = s.into_boxed_str();
                let static_str: &'static str = Box::leak(boxed);
                index_map.insert(static_str, idx);
                pool.push(static_str.into());
            }

            (pool, index_map)
        }

        let (ident_string_pool, ident_index_map) = build_pool_and_map(ident_vec);
        let (literal_string_pool, literal_index_map) = build_pool_and_map(literal_vec);

        Ok(Self {
            symbol_string_pool: ident_string_pool,
            symbol_id_map: ident_index_map,
            literal_string_pool,
            literal_id_map: literal_index_map,
        })
    }
}

impl std::fmt::Display for SymbolInterner {
    /// Formats the `StringInterner` by displaying both the `ident_string_pool` and the `literal_string_pool`.
    ///
    /// # Arguments
    ///
    /// * `f` - A mutable reference to a [`std::fmt::Formatter`] used to write the formatted output.
    ///
    /// # Returns
    ///
    /// A [`std::fmt::Result`] indicating whether the formatting succeeded or failed.
    ///
    /// # Behavior
    ///
    /// The method writes the contents of both string pools, showing indices and strings line-by-line,
    /// for easier readability when printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "StringInterner {{")?;

        writeln!(f, "Symbol String Pool:")?;
        for (idx, s) in self.symbol_string_pool.iter().enumerate() {
            writeln!(f, "    [{}]: {}", idx, s)?;
        }

        writeln!(f, "  Literal String Pool:")?;
        for (idx, s) in self.literal_string_pool.iter().enumerate() {
            writeln!(f, "    [{}]: {}", idx, s)?;
        }

        writeln!(f, "}}")
    }
}
