use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{Id, RemapSymbol, SymbolId, TypedSymbol};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::fmt;

/// The optimal capacity for inline (stack) storage of typed lists in PDDL.
///
/// Setting this to 8 aligns with most PDDL predicate and action signatures,
/// ensuring that parameter lists are stored on the stack to avoid heap
/// fragmentation during high-frequency operations like grounding.
pub const OPTIMAL_LIST_CAPACITY: usize = 8;

/// A collection of symbols where each symbol is associated with a specific typing.
///
/// `TypedList` acts as a high-performance container for [`TypedSymbol`], maintaining
/// the relationship between a symbol identifier and its corresponding typing.
///
/// This structure is specifically optimized for PDDL engines:
/// - **Stack Allocation**: Uses `SmallVec` to store up to `OPTIMAL_LIST_CAPACITY`
///   elements (typically 8) on the stack, avoiding heap allocations for most
///   action parameters and predicate signatures.
/// - **Cache Friendliness**: By keeping data inline, it improves CPU cache locality
///   during intensive phases like grounding or state space exploration.
///
/// # Generic Parameters
/// - `SID`: The identifier type for the symbol (e.g., `SymbolId`, `VariableId`).
/// - `TID`: The identifier type for the symbol's typing (e.g., `SymbolId`, `TypeId`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList<SID: Id, TID: Id> {
    /// The internal storage, stack-allocated for small lists.
    typed_symbols: SmallVec<[TypedSymbol<SID, TID>; OPTIMAL_LIST_CAPACITY]>,
}

impl<SID, TID> TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    /// Creates a new, empty `TypedList`.
    ///
    /// This constructor is zero-cost and performs no heap allocations.
    /// The internal storage is initialized on the stack with a capacity
    /// of `OPTIMAL_LIST_CAPACITY` (8), ready to receive elements without
    /// dynamic memory overhead.
    ///
    /// # Examples
    /// ```
    /// use aiplan4rust::TypedList;
    ///
    /// let list: TypedList<SymbolId, TypeId> = TypedList::new();
    /// assert!(list.is_empty());
    /// // No heap allocation occurred here.
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            typed_symbols: SmallVec::new(),
        }
    }

    /// Creates a new, empty `TypedList` with a specified initial capacity.
    ///
    /// # Implementation Details
    /// - If `capacity <= OPTIMAL_LIST_CAPACITY`, the list remains stack-allocated
    ///   (equivalent to `new()`).
    /// - If `capacity > OPTIMAL_LIST_CAPACITY`, it immediately allocates
    ///   the required space on the heap.
    ///
    /// This is useful when you know beforehand that you will store a large
    /// number of symbols (e.g., constants in a massive PDDL domain).
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            typed_symbols: SmallVec::with_capacity(capacity),
        }
    }

    /// Vide la liste de tous ses éléments.
    ///
    /// Cette méthode conserve la capacité allouée, permettant de réutiliser
    /// la mémoire pour les prochaines insertions.
    #[inline]
    pub fn clear(&mut self) {
        self.typed_symbols.clear();
    }

    /// Adds a typed symbol to the end of the list.
    ///
    /// # Parameters
    /// - `typed_symbol`: The symbol-typing pair to add.
    #[inline]
    pub fn push(&mut self, typed_symbol: TypedSymbol<SID, TID>) {
        self.typed_symbols.push(typed_symbol);
    }

    /// Returns an empty instance of `TypedList`.
    ///
    /// # Returns
    /// - An empty `TypedList` with no allocated capacity.
    #[inline]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns the total number of typed symbols in the list.
    ///
    /// # Returns
    /// - The count of elements as a `usize`.
    #[inline]
    pub fn len(&self) -> usize {
        self.typed_symbols.len()
    }

    /// Checks if the list contains no elements.
    ///
    /// # Returns
    /// - `true` if the list length is 0, `false` otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.typed_symbols.is_empty()
    }

    /// Returns an immutable iterator over the elements of the list.
    ///
    /// # Returns
    /// - An iterator yielding references to [`TypedSymbol`].
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, TypedSymbol<SID, TID>> {
        self.typed_symbols.iter()
    }

    /// Returns a mutable iterator over the elements of the list.
    ///
    /// # Returns
    /// - An iterator yielding mutable references to [`TypedSymbol`].
    #[inline]
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, TypedSymbol<SID, TID>> {
        self.typed_symbols.iter_mut()
    }

    /// Retrieves a reference to a symbol at a specific position.
    ///
    /// # Parameters
    /// - `index`: The zero-based position of the element.
    ///
    /// # Returns
    /// - `Some(&TypedSymbol)` if the index exists, or `None` if it is out of bounds.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&TypedSymbol<SID, TID>> {
        self.typed_symbols.get(index)
    }

    /// Provides access to the underlying symbols as a slice.
    ///
    /// # Returns
    /// - A borrowed slice containing all [`TypedSymbol`] elements.
    #[inline]
    pub fn as_slice(&self) -> &[TypedSymbol<SID, TID>] {
        &self.typed_symbols
    }

    /// Retrieves a mutable reference to a symbol at a specific position.
    ///
    /// # Parameters
    /// - `index`: The zero-based position of the element.
    ///
    /// # Returns
    /// - `Some(&mut TypedSymbol)` if the index exists, or `None` if it is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut TypedSymbol<SID, TID>> {
        self.typed_symbols.get_mut(index)
    }

    /// Sorts the list in-place based on symbol identifiers.
    ///
    /// This uses an unstable sort algorithm for optimal performance, meaning
    /// the relative order of identical symbols might not be preserved.
    pub fn sort_by_symbol(&mut self) {
        self.typed_symbols.sort_unstable_by_key(|ts| ts.symbol());
    }

    /// Removes consecutive duplicate symbols from the list.
    ///
    /// # Note
    /// - This only removes adjacent duplicates. To remove all duplicates,
    ///   call [`Self::sort_by_symbol`] first.
    pub fn dedup_by_symbol(&mut self) {
        self.typed_symbols.dedup_by_key(|ts| ts.symbol());
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// Elements for which the predicate returns `false` are removed from the list.
    ///
    /// # Performance
    /// This operation is performed in-place. If the list is stack-allocated,
    /// it remains on the stack throughout the process. It is highly efficient
    /// as it minimizes memory moves.
    ///
    /// # Examples
    /// ```
    /// list.retain(|ts| ts.symbol() != target_id);
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        // SmallVec attend &mut TypedSymbol, on ajuste donc la borne du trait
        F: FnMut(&mut TypedSymbol<SID, TID>) -> bool,
    {
        self.typed_symbols.retain(f);
    }

    /// Creates a draining iterator that removes the specified range from the list
    /// and yields the removed symbols.
    ///
    /// # Implementation Details
    /// Unlike `Vec::drain`, this returns a `smallvec::Drain`. If the list was
    /// stack-allocated, the elements are moved directly from the stack.
    ///
    /// After the iterator is dropped, the list's length is updated, but it
    /// retains its capacity (whether on the stack or the heap).
    ///
    /// # Panics
    /// Panics if the starting point is greater than the end point or if
    /// the end point is greater than the length of the list.
    #[inline]
    pub fn drain<R>(
        &mut self,
        range: R,
    ) -> smallvec::Drain<'_, [TypedSymbol<SID, TID>; OPTIMAL_LIST_CAPACITY]>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.typed_symbols.drain(range)
    }

    /// Appends all elements from a slice to the end of the list.
    ///
    /// # Performance
    /// Since `TypedSymbol` contains a `SmallVec` (via the `Type` field), it is
    /// not `Copy`. We use `.iter().cloned()` which is the idiomatic way to
    /// perform a bulk copy for non-Copy types.
    ///
    /// SmallVec is optimized to pre-allocate the required space before
    /// iterating, maintaining high performance.
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[TypedSymbol<SID, TID>]) {
        self.typed_symbols.extend(other.iter().cloned());
    }

    /// Extracts the content and resets the list while preserving allocated capacity.
    ///
    /// # Implementation Details
    /// This operation moves the current elements into a new `TypedList` and resets
    /// the original list to an empty state.
    /// - If the list was on the stack, it stays on the stack.
    /// - If it was on the heap, the new list takes ownership of the heap allocation,
    ///   and the original list resets to its initial stack-allocated state (8 slots).
    #[inline]
    pub fn take(&mut self) -> Self {
        // std::mem::take resets the field to its Default value (empty SmallVec on stack)
        // while moving the original content (and its heap pointer if any) out.
        Self {
            typed_symbols: std::mem::take(&mut self.typed_symbols),
        }
    }
}

impl RemapSymbol for TypedList<SymbolId, SymbolId> {
    /// Updates all symbols within the list using a provided mapping table.
    ///
    /// # Parameters
    /// - `map`: A reference to a `HashMap` containing the old-to-new `SymbolId` pairs.
    ///
    /// # Returns
    /// - `Ok(())` on success, or an `InternerError` if a symbol cannot be remapped.
    #[inline]
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        for ts in &mut self.typed_symbols {
            ts.remap_symbol(map)?;
        }
        Ok(())
    }
}

impl<SID, TID> From<Vec<TypedSymbol<SID, TID>>> for TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    /// Converts a standard `Vec` of typed symbols into a `TypedList`.
    ///
    /// # Performance
    /// This conversion is optimized for `SmallVec`:
    /// - If the `Vec` contains `OPTIMAL_LIST_CAPACITY` or fewer elements,
    ///   they are moved to the stack and the heap allocation is freed.
    /// - If the `Vec` is larger, `SmallVec` takes ownership of the existing
    ///   heap allocation, avoiding a full memory re-copy.
    #[inline]
    fn from(typed_symbols: Vec<TypedSymbol<SID, TID>>) -> Self {
        Self {
            typed_symbols: SmallVec::from_vec(typed_symbols),
        }
    }
}

impl<SID, TID> FromIterator<TypedSymbol<SID, TID>> for TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    /// Creates a `TypedList` by collecting an iterator of symbols.
    ///
    /// # Parameters
    /// - `iter`: The iterator providing [`TypedSymbol`] elements.
    fn from_iter<I: IntoIterator<Item = TypedSymbol<SID, TID>>>(iter: I) -> Self {
        Self {
            typed_symbols: iter.into_iter().collect(),
        }
    }
}

impl<SID, TID> Extend<TypedSymbol<SID, TID>> for TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    /// Extends the list by appending elements from an iterator.
    ///
    /// # Parameters
    /// - `iter`: An object that can be converted into an iterator of [`TypedSymbol`].
    #[inline]
    fn extend<I: IntoIterator<Item = TypedSymbol<SID, TID>>>(&mut self, iter: I) {
        self.typed_symbols.extend(iter);
    }
}

impl<SID, TID> std::ops::Index<usize> for TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    type Output = TypedSymbol<SID, TID>;

    /// Provides read-only access to symbols using square bracket syntax.
    ///
    /// # Parameters
    /// - `index`: The position of the element to access.
    ///
    /// # Returns
    /// - A reference to the [`TypedSymbol`] at the specified index.
    ///
    /// # Panics
    /// - Panics if the `index` is out of bounds.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.typed_symbols[index]
    }
}

impl<SID: Id, TID: Id> IntoIterator for TypedList<SID, TID> {
    /// The type of the elements being iterated over (owned symbols).
    type Item = TypedSymbol<SID, TID>;
    /// The specific iterator type for SmallVec, preserving stack/heap logic.
    type IntoIter = smallvec::IntoIter<[TypedSymbol<SID, TID>; OPTIMAL_LIST_CAPACITY]>;

    /// Converts the `TypedList` into an owning iterator.
    ///
    /// # Performance
    /// - **Stack Path**: If the list size is $\le 8$, the iterator carries the
    ///   elements on the stack. No heap allocation is performed.
    /// - **Heap Path**: If the list was spilled to the heap, the iterator
    ///   takes ownership of the pointer.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.typed_symbols.into_iter()
    }
}

impl<'a, SID: Id, TID: Id> IntoIterator for &'a TypedList<SID, TID> {
    /// The type of the elements being iterated over (borrowed symbols).
    type Item = &'a TypedSymbol<SID, TID>;
    /// A standard slice iterator, optimized by the compiler.
    type IntoIter = std::slice::Iter<'a, TypedSymbol<SID, TID>>;

    /// Creates a borrowing iterator from a reference to `TypedList`.
    ///
    /// # Performance
    /// Delegates to the underlying slice iterator. This is highly efficient
    /// and allows the compiler to perform SIMD optimizations and loop unrolling.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.typed_symbols.iter()
    }
}

impl<SID: Id, TID: Id> fmt::Display for TypedList<SID, TID> {
    /// Formats the list for standard display purposes.
    ///
    /// # Returns
    /// - `fmt::Result` indicating the success of the write operation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.typed_symbols.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{sym}")?;
        }
        write!(f, ")")
    }
}

impl<SID: Id, TID: Id> InternerDisplay for TypedList<SID, TID>
where
    TypedSymbol<SID, TID>: InternerDisplay,
{
    /// Formats the list using an interner to resolve symbol names.
    ///
    /// # Parameters
    /// - `interner`: The [`SymbolInterner`] used to look up human-readable names.
    ///
    /// # Returns
    /// - `fmt::Result` indicating the success of the write operation.
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        write!(f, "(")?;
        for (i, sym) in self.typed_symbols.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            sym.fmt_with_interner(f, interner)?;
        }
        write!(f, ")")
    }
}

impl<SID: Id, TID: Id> SyntaxInternerDisplay for TypedList<SID, TID>
where
    TypedSymbol<SID, TID>: SyntaxInternerDisplay,
{
    /// Formats the list specifically for syntax-related output with indentation.
    ///
    /// # Parameters
    /// - `interner`: The interner used to resolve names.
    /// - `indent`: The number of spaces to prefix the output.
    ///
    /// # Returns
    /// - `fmt::Result` indicating the success of the write operation.
    fn fmt_syntax_with_interner_and_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
        indent: usize,
    ) -> fmt::Result {
        write_indent(f, indent)?;
        for (i, sym) in self.typed_symbols.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            sym.fmt_syntax_with_interner(f, interner)?;
        }
        Ok(())
    }
}
