use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

// Imports de ton projet
use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};


/// Trait pour tous les wrappers d'identifiants basés sur un index usize.
pub trait Id: Copy + Eq + Ord + Default + std::hash::Hash + Serialize + fmt::Display + From<usize> + Into<usize> {
    fn new(idx: usize) -> Self;
    fn as_usize(self) -> usize;
    fn is_valid(self) -> bool {
        self.as_usize() != usize::MAX
    }
}

// --- MACRO DE GÉNÉRATION UNIFORME ---

macro_rules! impl_id_type {
    ($id:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $id {
            pub value: usize,
        }

        impl Id for $id {
            fn new(value: usize) -> Self { Self { value } }
            fn as_usize(self) -> usize { self.value }
        }

        impl Default for $id {
            fn default() -> Self { Self { value: usize::MAX } }
        }

        impl $id {
            pub const fn new(value: usize) -> Self { Self { value } }
            pub fn as_usize(self) -> usize { self.value }
            pub fn is_valid(&self) -> bool { self.value != usize::MAX }
            pub fn invalid_value() -> usize { usize::MAX }
        }

        impl From<usize> for $id {
            fn from(value: usize) -> Self { Self::new(value) }
        }

        impl From<$id> for usize {
            fn from(id: $id) -> Self { id.value }
        }

        impl<T> Index<$id> for Vec<T> {
            type Output = T;
            fn index(&self, id: $id) -> &Self::Output { &self[id.value] }
        }

        impl<T> IndexMut<$id> for Vec<T> {
            fn index_mut(&mut self, id: $id) -> &mut Self::Output { &mut self[id.value] }
        }
    };
}

// --- DÉFINITIONS DES TYPES ---

impl_id_type!(SymbolId);
impl_id_type!(LiteralId);

impl_id_type!(TypeId);
impl_id_type!(VariableId);
impl_id_type!(ObjectId);
impl_id_type!(FluentId);
impl_id_type!(NumericFluentId);

impl_id_type!(PredicateSymbolId);
impl_id_type!(FunctionSymbolId);
impl_id_type!(PreferenceSymbolId);
impl_id_type!(TaskSymbolId);
impl_id_type!(ActionSymbolId);
impl_id_type!(ActionDefId);
impl_id_type!(MethodSymbolId);
impl_id_type!(TaskLabelSymbolId);

//impl_id_type!(AtomSkeletonId);
impl_id_type!(FunctionSkeletonId);
impl_id_type!(TaskSkeletonId);

// --- LOGIQUE SPÉCIFIQUE (REMAP) ---

impl SymbolId {
    pub fn remap_idents(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
        Ok(())
    }
}

impl LiteralId {
    pub fn remap_literal(&mut self, map: &HashMap<LiteralId, LiteralId>) {
        if let Some(new) = map.get(self) {
            *self = *new;
        }
    }
}

// --- AFFICHAGE (DISPLAY AVEC PRÉFIXES) ---

macro_rules! impl_display_prefix {
    ($id:ident, $prefix:expr) => {
        impl fmt::Display for $id {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}#{}", $prefix, self.value)
            }
        }
    };
}

// --- Symbols & Interning ---
// s1, s42 (Generic internal symbol)
impl_display_prefix!(SymbolId, "s");
// L10 (Literal value, usually a constant or a boolean)
impl_display_prefix!(LiteralId, "L");

// --- Domain & Typing ---
// T1, T_robot (Type identifier)
impl_display_prefix!(TypeId, "T");
// v0, v1 (Variable identifier)
impl_display_prefix!(VariableId, "v");
// o12 (Instance of an object/constant)
impl_display_prefix!(ObjectId, "o");

// --- Domain Model Symbols (Names) ---
// p5 (Predicate name/symbol)
impl_display_prefix!(PredicateSymbolId, "p");
// f2 (Function name/symbol - lower case to distinguish from skeleton)
impl_display_prefix!(FunctionSymbolId, "f");
// tk4 (Task symbol - specific to HTN/Planning)
impl_display_prefix!(TaskSymbolId, "tk");
// a3 (Action symbol/operator name)
impl_display_prefix!(ActionSymbolId, "a");
impl_display_prefix!(ActionDefId, "adef");
// m7 (Method symbol in HTN planning)
impl_display_prefix!(MethodSymbolId, "m");
// pr1 (Preference symbol for soft constraints)
impl_display_prefix!(PreferenceSymbolId, "pr");
// TL9 (Task Label instance - Upper case to distinguish from Task Symbol)
impl_display_prefix!(TaskLabelSymbolId, "TL");

// --- Fluents (State Variables) ---
// fl8 (Generic fluent - used for state tracking)
impl_display_prefix!(FluentId, "fl");
// nf4 (Numeric fluent - e.g., battery levels, distances)
impl_display_prefix!(NumericFluentId, "nf");
// of2 (Object fluent - a state variable returning an ObjectId)

// --- Skeletons (Structural Instances) ---
// FS2 (Function Skeleton: Function + Arguments)
impl_display_prefix!(FunctionSkeletonId, "FS");
// TS7 (Task Skeleton: Task + Arguments)
impl_display_prefix!(TaskSkeletonId, "TS");
// --- TRAITS INTERNER (RESOLUTION) ---

impl TypeId {
    /// Reserved identifier for the "number" type, used for numeric fluents and functions.
    /// By convention, this corresponds to the second entry in a standard interner.
    pub const NUMBER_TYPE_ID: Self = Self::new(1);

    /// Reserved sentinel identifier for the PDDL root type (the implicit 'object' type).
    ///
    /// This specific ID is used because the 'object' type is inconsistently defined
    /// in PDDL files: it can be explicitly declared, used implicitly as a parent,
    /// or omitted entirely. In our Datalog engine, a "null" or empty type
    /// specification always resolves to this root sentinel to ensure consistency.
    pub const ROOT_TYPE_ID: Self = Self::new(usize::MAX);

    /// Returns the sentinel value representing the PDDL root type ('object').
    #[inline]
    pub const fn root() -> Self {
        Self::ROOT_TYPE_ID
    }

    /// Checks if this type represents the PDDL root type.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.as_usize() == Self::ROOT_TYPE_ID.as_usize()
    }

    /// Returns the reserved identifier for the "number" type.
    #[inline]
    pub const fn number() -> Self {
        Self::NUMBER_TYPE_ID
    }

    /// Checks if this type represents a numeric value.
    #[inline]
    pub fn is_number(&self) -> bool {
        self.as_usize() == Self::NUMBER_TYPE_ID.as_usize()
    }
}

impl InternerDisplay for SymbolId {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if let Some(name) = interner.resolve_symbol(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "{}", SymbolInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl InternerDisplay for LiteralId {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "{}", SymbolInterner::UNKNOWN_INTERNED_STRING)
        }
    }
}

impl SyntaxInternerDisplay for SymbolId {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &SymbolInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(name) = interner.resolve_symbol(*self) {
            write!(f, "{}", name)
        } else {
            write!(f, "<uninterned_sym:{}>", self.value)
        }
    }
}

impl SyntaxInternerDisplay for LiteralId {
    fn fmt_syntax_with_interner_and_indent(&self, f: &mut Formatter<'_>, interner: &SymbolInterner, indent: usize) -> fmt::Result {
        write_indent(f, indent)?;
        if let Some(val) = interner.resolve_literal(*self) {
            write!(f, "{}", val)
        } else {
            write!(f, "<uninterned_lit:{}>", self.value)
        }
    }
}


const NEGATION_FLAG: usize = 1 << (std::mem::size_of::<usize>() * 8 - 1);
const ID_MASK: usize = !NEGATION_FLAG;

/// A specialized identifier for Atom Skeletons (predicates with arguments).
///
/// # The Bit-Tagging Mechanism
/// Unlike other identifiers in the system, `AtomSkeletonId` uses a **Most Significant Bit (MSB)**
/// tagging mechanism to represent logical negation directly within the ID.
///
/// * **Positive Atom:** The MSB is `0`. The value represents the raw index in the predicate registry.
/// * **Negative Atom:** The MSB is `1` (`NEGATION_FLAG`). The remaining bits represent the raw index.
///
///
///
/// # Why this ID is different
/// While most IDs are simple indices, `AtomSkeletonId` must frequently handle the
/// Closed World Assumption (CWA) and Precondition/Effect negation. By encoding
/// negation in the ID:
/// 1. **Memory Efficiency:** We avoid wrapping atoms in a `Negative(AtomId)` enum,
///    keeping the data structure flat and small.
/// 2. **Transparent Indexing:** The `Index` trait is overloaded to automatically
///    mask the negation bit. This allows using a negated ID to access positive
///    metadata (like arity or types) without manual bit-stripping.
/// 3. **Datalog Compatibility:** It allows the Datalog engine to treat `not P`
///    as a distinct positive fact, simplifying reachability analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AtomSkeletonId {
    /// The internal representation containing both the index and the optional negation flag.
    pub value: usize,
}

impl Id for AtomSkeletonId {
    /// Creates a new ID from a raw `usize`.
    ///
    /// * `value`: The raw integer value (can include the `NEGATION_FLAG`).
    /// * **Returns**: A new `AtomSkeletonId` instance.
    fn new(value: usize) -> Self { Self { value } }

    /// Returns the raw index by masking the negation bit.
    ///
    /// * **Returns**: A `usize` index between `0` and `2^63 - 1` (on 64-bit systems),
    ///   guaranteed to be safe for vector indexing.
    #[inline(always)]
    fn as_usize(self) -> usize { self.value & ID_MASK }
}

impl Default for AtomSkeletonId {
    /// Returns a sentinel invalid ID (`usize::MAX`).
    fn default() -> Self { Self { value: usize::MAX } }
}

impl AtomSkeletonId {
    /// Constant constructor for the ID.
    pub const fn new(value: usize) -> Self { Self { value } }

    /// Returns the pure index, stripped of any negation flags.
    ///
    /// * **Returns**: The index part of the ID as a `usize`.
    #[inline(always)]
    pub fn as_usize(self) -> usize { self.value & ID_MASK }

    /// Checks if the ID is valid.
    ///
    /// * **Returns**: `true` if the ID is not equal to `invalid_value()`.
    pub fn is_valid(&self) -> bool { self.value != usize::MAX }

    /// Returns the sentinel value for invalid IDs.
    pub fn invalid_value() -> usize { usize::MAX }

    // --- Specific Negation Logic ---

    /// Returns `true` if this ID represents a negated atom.
    ///
    /// * **Returns**: `true` if the MSB is set, `false` otherwise.
    #[inline(always)]
    pub fn is_negated(self) -> bool {
        (self.value & NEGATION_FLAG) != 0
    }

    /// Creates a negated version of the current ID.
    ///
    /// * **Returns**: A new `AtomSkeletonId` with the MSB set.
    ///   If the ID was already negated, it remains negated.
    #[inline(always)]
    pub fn to_negated(self) -> Self {
        Self { value: self.value | NEGATION_FLAG }
    }

    /// Removes the negation bit to return the positive base ID.
    ///
    /// * **Returns**: A new `AtomSkeletonId` with the MSB set to `0`.
    #[inline(always)]
    pub fn strip_negation(self) -> Self {
        Self { value: self.value & ID_MASK }
    }
}

impl From<usize> for AtomSkeletonId {
    fn from(value: usize) -> Self { Self::new(value) }
}

impl From<AtomSkeletonId> for usize {
    /// Returns the raw bit-packed value.
    ///
    /// * **Warning**: This value includes the `NEGATION_FLAG`.
    ///   Do not use this for direct array indexing; use `.as_usize()` instead.
    fn from(id: AtomSkeletonId) -> Self { id.value }
}

impl<T> Index<AtomSkeletonId> for Vec<T> {
    type Output = T;
    /// Automatically masks the negation bit to provide safe indexing into metadata vectors.
    ///
    /// * `id`: The `AtomSkeletonId` (positive or negative).
    /// * **Returns**: A reference to the element at the stripped index.
    #[inline(always)]
    fn index(&self, id: AtomSkeletonId) -> &Self::Output {
        &self[id.as_usize()]
    }
}

impl<T> IndexMut<AtomSkeletonId> for Vec<T> {
    /// Automatically masks the negation bit for safe mutable indexing.
    ///
    /// * `id`: The `AtomSkeletonId` (positive or negative).
    /// * **Returns**: A mutable reference to the element at the stripped index.
    #[inline(always)]
    fn index_mut(&mut self, id: AtomSkeletonId) -> &mut Self::Output {
        &mut self[id.as_usize()]
    }
}

impl fmt::Display for AtomSkeletonId {
    /// Formats the ID for debugging.
    ///
    /// * **Output format**: `AS#10` for positive, `not_AS#10` for negative.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_negated() {
            write!(f, "not_AS#{}", self.as_usize())
        } else {
            write!(f, "AS#{}", self.as_usize())
        }
    }
}
