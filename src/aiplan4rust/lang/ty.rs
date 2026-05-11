use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{Id, RemapSymbol, SymbolId, TypeId};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use core::borrow::Borrow;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

/// The optimal capacity for inline (stack) storage of PDDL types.
///
/// In PDDL, the vast majority of objects have a single type. A capacity of 2
/// allows for common hierarchical pairs (e.g., [Truck, Vehicle]) to be stored
/// without triggering heap allocations, significantly reducing memory pressure
/// during grounding and state exploration.
pub const OPTIMAL_TYPE_CAPACITY: usize = 2;

/// Represents a generic PDDL typing, supporting both atomic types and
/// unions (via the `either` keyword).
///
/// The generic parameter `ID` typically transitions from a `SymbolId` during
/// the syntactic phase to a `TypeId` during the semantic/grounding phase.
///
/// This structure is optimized for high-performance planning engines:
/// - Small hierarchies are stack-allocated via `SmallVec`.
/// - Comparisons and lookups can be performed against raw slices via `Borrow`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Type<ID: Id> {
    /// The constituent atomic identifiers of this typing.
    /// An empty list represents the root type (i.e., 'object').
    members: SmallVec<[ID; OPTIMAL_TYPE_CAPACITY]>,
}

impl<ID: Id> Default for Type<ID> {
    /// Creates a debug, empty `Type`.
    ///
    /// An empty `Type` is considered a "root" or "top" type in PDDL (equivalent to `object`).
    /// This constructor is zero-cost as it does not trigger any heap allocation.
    #[inline]
    fn default() -> Self {
        Self {
            members: SmallVec::new(),
        }
    }
}

impl<ID: Id> Type<ID> {
    /// Creates a new, empty `Type`.
    ///
    /// Represented as a `root` type in PDDL (equivalent to `object`), this
    /// constructor is zero-cost and performs no allocations.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Alias for `new()`. Returns the root of the type hierarchy.
    #[inline]
    pub fn root() -> Self {
        Self::new()
    }

    /// Creates an atomic (primitive) `Type` from a single identifier.
    ///
    /// # Parameters
    /// - `id`: The unique identifier for the atomic type.
    ///
    /// # Returns
    /// A `Type` instance stored inline on the stack.
    #[inline]
    pub fn primitive(id: ID) -> Self {
        Self {
            members: smallvec::smallvec![id],
        }
    }

    /// Creates a compound `Type` representing a union of types (PDDL `either`).
    ///
    /// # Parameters
    /// - `ids`: A slice of identifiers to be included in the union.
    ///
    /// # Returns
    /// A `Type` instance. If `ids.len() <= OPTIMAL_TYPE_CAPACITY`, storage remains
    /// on the stack. Otherwise, it transparently spills to the heap.
    ///
    /// # Panics
    /// Panics if the `ids` slice is empty, as PDDL unions must contain at least one member.
    pub fn either(ids: &[ID]) -> Self {
        assert!(!ids.is_empty(), "A PDDL 'either' type cannot be empty.");

        Self {
            members: SmallVec::from_slice(ids),
        }
    }

    /// Appends a new atomic type to the current typing.
    ///
    /// # Parameters
    /// - `member`: The identifier to add.
    ///
    /// # Performance
    /// May trigger a heap allocation if the total count exceeds `OPTIMAL_TYPE_CAPACITY`.
    #[inline]
    pub fn add_type(&mut self, member: ID) {
        self.members.push(member);
    }

    /// Returns an immutable slice view of the constituent identifiers.
    #[inline]
    pub fn members(&self) -> &[ID] {
        &self.members
    }

    /// Returns a mutable reference to the internal `SmallVec` storage.
    #[inline]
    pub fn members_mut(&mut self) -> &mut SmallVec<[ID; OPTIMAL_TYPE_CAPACITY]> {
        &mut self.members
    }

    /// Returns the number of atomic types defined in this typing.
    #[inline]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns `true` if the typing is empty (represents the root/object type).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Alias for `is_empty()`.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.is_empty()
    }

    /// Returns `true` if the typing consists of exactly one atomic type.
    #[inline]
    pub fn is_primitive(&self) -> bool {
        self.members.len() == 1
    }

    /// Returns `true` if the typing is a union of multiple types (`either`).
    #[inline]
    pub fn is_either(&self) -> bool {
        self.members.len() > 1
    }

    /// Returns an iterator over the constituent type identifiers.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, ID> {
        self.members.as_slice().iter()
    }

    /// Returns a mutable iterator over the constituent type identifiers.
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, ID> {
        self.members.iter_mut()
    }
}

impl<ID: Id> FromIterator<ID> for Type<ID> {
    /// Creates a `Type` by collecting an iterator of identifiers.
    ///
    /// # Implementation Details
    /// This operation is optimized to collect elements directly into the
    /// internal `SmallVec`.
    /// - If the iterator's size is within `OPTIMAL_TYPE_CAPACITY`,
    ///   the resulting `Type` will be stack-allocated.
    /// - If the number of elements exceeds the inline capacity,
    ///   it will automatically spill to a single heap allocation.
    #[inline]
    fn from_iter<I: IntoIterator<Item = ID>>(iter: I) -> Self {
        // SmallVec::from_iter est optimisé pour utiliser size_hint()
        // de l'itérateur et éviter les réallocations inutiles.
        let members = SmallVec::from_iter(iter);

        Self { members }
    }
}

/// Enables direct iteration over a `Type` reference in `for` loops.
///
/// This implementation allows for idiomatic patterns like:
/// ```rust
/// for id in &my_type { /* ... */ }
/// ```
///
/// It leverages the underlying slice iterator, ensuring that the
/// abstraction is entirely erased during compilation.
impl<'a, ID: Id> IntoIterator for &'a Type<ID> {
    /// The type of the elements being iterated over (references to IDs).
    type Item = &'a ID;
    /// The specific iterator type, reusing the standard slice iterator.
    type IntoIter = std::slice::Iter<'a, ID>;

    /// Returns an iterator over the constituent identifiers of the `Type`.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Facilitates high-performance lookups in hashed collections.
///
/// By implementing `Borrow<[ID]>`, this type allows a `HashMap` or `IndexMap`
/// keyed by `Type<ID>` to be queried using a simple slice (`&[ID]`).
///
/// This eliminates the need to allocate or construct a full `Type` instance
/// just to perform a cache lookup, effectively enabling zero-allocation queries.
impl<ID: Id> Borrow<[ID]> for Type<ID> {
    /// Borrows the internal storage as a slice of identifiers.
    #[inline(always)]
    fn borrow(&self) -> &[ID] {
        &self.members
    }
}

/// Efficiently converts a standard `Vec` into a `Type`.
///
/// This implementation is specialized for performance:
/// - If the `Vec` contains `OPTIMAL_TYPE_CAPACITY` or fewer elements,
///   they are moved onto the stack and the heap allocation is freed.
/// - If the `Vec` is larger, `SmallVec` "reclaims" the existing heap allocation,
///   avoiding a new allocation and a full memory copy.
impl<ID: Id> From<Vec<ID>> for Type<ID> {
    /// Performs the conversion by consuming the source `Vec`.
    #[inline]
    fn from(members: Vec<ID>) -> Self {
        Self {
            members: SmallVec::from_vec(members),
        }
    }
}

/// Specialization for `SymbolId` (Syntactic/Symbolic phase).
///
/// These methods provide direct access to PDDL built-in types using
/// reserved identifiers from the `SymbolInterner`.
impl Type<SymbolId> {
    /// Returns a `Type` representing the built-in numeric type.
    #[inline]
    pub fn number() -> Self {
        Self::primitive(SymbolInterner::NUMBER_SYMBOL_ID)
    }

    /// Returns a `Type` representing the built-in `object` type.
    #[inline]
    pub fn object() -> Self {
        Self::primitive(SymbolInterner::OBJECT_SYMBOL_ID)
    }

    /// Checks if this type is exactly the built-in `object`.
    ///
    /// # Implementation Details
    /// Uses `.first()` for bounds-check-free access after confirming
    /// the type is primitive. The compiler optimizes this to a single
    /// integer comparison.
    #[inline]
    pub fn is_object(&self) -> bool {
        self.is_primitive() && self.members.first() == Some(&SymbolInterner::OBJECT_SYMBOL_ID)
    }

    /// Checks if this type is exactly the built-in `number`.
    #[inline]
    pub fn is_number(&self) -> bool {
        self.is_primitive() && self.members.first() == Some(&SymbolInterner::NUMBER_SYMBOL_ID)
    }
}
/// Specialization for `TypeId` (Semantic/Grounding phase).
///
/// These methods are optimized for the grounding and state exploration phases,
/// where type checks are performed frequently.
impl Type<TypeId> {
    /// Returns a `Type` representing the numeric type identifier.
    #[inline]
    pub fn number() -> Self {
        Self::primitive(TypeId::NUMBER_TYPE_ID)
    }

    /// Returns `true` if this typing represents a numeric value.
    ///
    /// # Implementation Details
    /// A typing is considered numeric if it is primitive and its single
    /// member is the reserved `TypeId::NUMBER_TYPE_ID`.
    ///
    /// The use of `map_or` combined with `#[inline]` allows the compiler to
    /// flatten this check into a single integer comparison after bounds-check
    /// elimination.
    #[inline]
    pub fn is_number(&self) -> bool {
        self.is_primitive() && self.members.first().map_or(false, |id| id.is_number())
    }
}

/// Implements symbol remapping for syntactic types.
///
/// This trait is used to update the underlying `SymbolId`s when the
/// interner's mapping changes or during domain canonicalization.
impl RemapSymbol for Type<SymbolId> {
    /// Remaps all constituent member identifiers using the provided map.
    ///
    /// # Parameters
    /// - `map`: A translation table between old and new `SymbolId`s.
    ///
    /// # Errors
    /// Returns `InternerError` if any constituent `SymbolId` cannot be
    /// remapped (e.g., missing entry in the map).
    ///
    /// # Performance
    /// Operates in-place. If the type is stored on the stack (<= 2 members),
    /// this avoids all heap traffic.
    #[inline]
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        for ident in &mut self.members {
            // In-place mutation of the SmallVec elements
            ident.remap_idents(map)?;
        }
        Ok(())
    }
}

// --- 1. General Debug/Log Display ---

/// Implements the standard `Display` trait for generic output.
///
/// Primarily used for logging and internal debugging. It uses a
/// user-friendly `either(A, B)` format for compound types.
impl<ID: Id> fmt::Display for Type<ID> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_primitive() {
            // Safety: is_primitive guarantees len == 1
            write!(f, "{}", self.members[0])
        } else {
            write!(f, "either(")?;
            for (i, id) in self.members.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", id)?;
            }
            write!(f, ")")
        }
    }
}

// --- 2. Logical Display (Interned) ---

/// Specialized display for `SymbolId` using a `SymbolInterner`.
/// Translates raw IDs back into their original string representations.
impl InternerDisplay for Type<SymbolId> {
    fn fmt_with_interner(&self, w: &mut Formatter<'_>, interner: &SymbolInterner) -> fmt::Result {
        if self.members.is_empty() {
            return write!(w, "<empty>");
        }
        for (i, ty) in self.members.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            match interner.resolve_symbol(*ty) {
                Some(name) => write!(w, "{}", name)?,
                None => write!(w, "{}", ty)?, // Fallback to raw ID
            }
        }
        Ok(())
    }
}

/// Specialized display for `TypeId`.
/// Note: Since `TypeId` represents compiled types, it falls back to
/// technical IDs as they are not mapped in the `SymbolInterner`.
impl InternerDisplay for Type<TypeId> {
    fn fmt_with_interner(&self, w: &mut Formatter<'_>, _interner: &SymbolInterner) -> fmt::Result {
        if self.members.is_empty() {
            return write!(w, "<empty>");
        }
        for (i, ty) in self.members.iter().enumerate() {
            if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", ty)?;
        }
        Ok(())
    }
}

// --- 3. Syntactic PDDL Export ---

/// Implements the PDDL syntax-compliant display for `SymbolId`.
///
/// Handles the correct S-expression format: `object`, `name`, or `(either name1 name2)`.
impl SyntaxInternerDisplay for Type<SymbolId> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        match self.members.len() {
            0 => write!(f, "object"),
            1 => {
                let ty = self.members[0];
                match interner.resolve_symbol(ty) {
                    Some(name) => write!(f, "{}", name),
                    None => write!(f, "{}", ty),
                }
            }
            _ => {
                write!(f, "(either")?;
                for ty in &self.members {
                    write!(f, " ")?;
                    match interner.resolve_symbol(*ty) {
                        Some(name) => write!(f, "{}", name)?,
                        None => write!(f, "{}", ty)?,
                    }
                }
                write!(f, ")")
            }
        }
    }
}

/// Fallback syntax display for `TypeId`.
/// Reuses the standard `Display` as `TypeId` is usually only seen in
/// post-compilation debug views.
impl SyntaxInternerDisplay for Type<TypeId> {
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut Formatter<'_>,
        _interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        write!(f, "{}", self)
    }
}
