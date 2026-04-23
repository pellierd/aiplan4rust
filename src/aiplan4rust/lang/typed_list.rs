use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{Id, RemapSymbol, SymbolId, TypedSymbol};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A collection of symbols where each symbol is associated with a specific typing.
///
/// `TypedList` acts as a container for [`TypedSymbol`], maintaining the relationship
/// between a symbol identifier and its corresponding typing identifier.
///
/// # Generic Parameters
/// - `SID`: The identifier typing for the symbol (e.g., `StringID`, `VariableID`). Must implement [`Id`].
/// - `TID`: The identifier typing for the symbol's typing (e.g., `TypeID`). Must implement [`Id`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypedList<SID: Id, TID: Id> {
    typed_symbols: Vec<TypedSymbol<SID, TID>>,
}

impl<SID, TID> TypedList<SID, TID>
where
    SID: Id,
    TID: Id,
{
    /// Creates a new, empty `TypedList`.
    ///
    /// The list is initialized without any symbols and will not allocate
    /// until elements are pushed into it.
    ///
    /// # Examples
    /// ```
    /// use my_crate::TypedList;
    ///
    /// let list: TypedList<MySymbolId, MyTypeId> = TypedList::new();
    /// assert!(list.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            typed_symbols: Vec::new(),
        }
    }

    /// Crée une nouvelle `TypedList` vide avec une capacité initiale spécifiée.
    ///
    /// La liste pourra contenir au moins `capacity` éléments sans réallouer.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            typed_symbols: Vec::with_capacity(capacity),
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
    pub fn push(&mut self, typed_symbol: TypedSymbol<SID, TID>) {
        self.typed_symbols.push(typed_symbol);
    }

    /// Returns an empty instance of `TypedList`.
    ///
    /// # Returns
    /// - An empty `TypedList` with no allocated capacity.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Returns the total number of typed symbols in the list.
    ///
    /// # Returns
    /// - The count of elements as a `usize`.
    pub fn len(&self) -> usize {
        self.typed_symbols.len()
    }

    /// Checks if the list contains no elements.
    ///
    /// # Returns
    /// - `true` if the list length is 0, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.typed_symbols.is_empty()
    }

    /// Returns an immutable iterator over the elements of the list.
    ///
    /// # Returns
    /// - An iterator yielding references to [`TypedSymbol`].
    pub fn iter(&self) -> std::slice::Iter<'_, TypedSymbol<SID, TID>> {
        self.typed_symbols.iter()
    }

    /// Returns a mutable iterator over the elements of the list.
    ///
    /// # Returns
    /// - An iterator yielding mutable references to [`TypedSymbol`].
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
    pub fn get(&self, index: usize) -> Option<&TypedSymbol<SID, TID>> {
        self.typed_symbols.get(index)
    }

    /// Provides access to the underlying symbols as a slice.
    ///
    /// # Returns
    /// - A borrowed slice containing all [`TypedSymbol`] elements.
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

    /// Conserve uniquement les éléments qui satisfont le prédicat donné.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&TypedSymbol<SID, TID>) -> bool,
    {
        self.typed_symbols.retain(f);
    }

    /// Crée un itérateur qui extrait (déplace) tous les éléments de la liste.
    ///
    /// La liste est vidée mais conserve sa capacité allouée.
    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, TypedSymbol<SID, TID>>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.typed_symbols.drain(range)
    }

    /// Ajoute les éléments d'un slice à la fin de la liste.
    ///
    /// Cette opération est très efficace car elle utilise la performance
    /// native de `Vec::extend_from_slice`.
    pub fn extend_from_slice(&mut self, other: &[TypedSymbol<SID, TID>]) {
        self.typed_symbols.extend_from_slice(other);
    }

    /// Extrait le contenu et réinitialise la liste en conservant la capacité.
    pub fn take(&mut self) -> Self {
        let old_capacity = self.typed_symbols.capacity();
        // On prend le Vec (zéro allocation)
        let symbols = std::mem::take(&mut self.typed_symbols);
        // On redonne un Vec vide au buffer avec la capacité initiale pour les prochains appels
        self.typed_symbols = Vec::with_capacity(old_capacity);

        Self {
            typed_symbols: symbols,
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
    /// Converts a vector of typed symbols into a `TypedList`.
    ///
    /// # Parameters
    /// - `typed_symbols`: The vector to be converted.
    ///
    /// # Returns
    /// - A `TypedList` instance owning the provided symbols.
    fn from(typed_symbols: Vec<TypedSymbol<SID, TID>>) -> Self {
        Self { typed_symbols }
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
    fn extend<I: IntoIterator<Item = TypedSymbol<SID, TID>>>(&mut self, iter: I) {
        for item in iter {
            self.typed_symbols.push(item);
        }
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
    fn index(&self, index: usize) -> &Self::Output {
        &self.typed_symbols[index]
    }
}

impl<SID: Id, TID: Id> IntoIterator for TypedList<SID, TID> {
    type Item = TypedSymbol<SID, TID>;
    type IntoIter = std::vec::IntoIter<TypedSymbol<SID, TID>>;

    /// Converts the `TypedList` into an owning iterator.
    ///
    /// # Returns
    /// - An iterator consuming the list and yielding [`TypedSymbol`] elements.
    fn into_iter(self) -> Self::IntoIter {
        self.typed_symbols.into_iter()
    }
}

impl<'a, SID: Id, TID: Id> IntoIterator for &'a TypedList<SID, TID> {
    type Item = &'a TypedSymbol<SID, TID>;
    type IntoIter = std::slice::Iter<'a, TypedSymbol<SID, TID>>;

    /// Creates a borrowing iterator from a reference to `TypedList`.
    ///
    /// # Returns
    /// - An iterator yielding references to [`TypedSymbol`].
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
