//! # Symbol Table for `aiplan4rust`
//!
//! This module defines the [`Table`] struct, which implements the symbol table used by the
//! `aiplan4rust` compiler pipeline. It provides facilities for storing and retrieving symbol entries,
//! performing scoped lookups, checking for declaration and usage conflicts, and preserving symbol
//! insertion order.
//!
//! The symbol table is used throughout parsing, semantic analysis, and linking phases.
//!
//! ## Key Features
//! - Maintains insertion order using `LinkedHashMap` for deterministic diagnostics and output.
//! - Supports filtering by symbol kind and scope.
//! - Tracks both declarations and usages.
//! - Keeps metadata about its construction origin (e.g., domain file, problem file, merged).
//!
//! ## Related Components
//! - [`SymbolEntry`]: Represents a symbol and its declarations/usages.
//! - [`SymbolKind`], [`Scope`], [`Declaration`], [`Usage`]: Metadata for symbols.
//!
//! ## Example
//! ```rust
//! use aiplan4rust::semantic::Table;
//!
//! let mut table = Table::default();
//! assert!(table.iter().count() == 0);
//! ```

use crate::aiplan4rust::interner::{InternerDisplay, InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{RemapSymbol, SymbolId};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Filterable;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol_table::{
    SymbolTableBuilder, SymbolTableError, SymbolTableOrigin,
};
use crate::aiplan4rust::semantic::type_checker::TypeHierarchy;
use crate::aiplan4rust::semantic::{SemanticError, SymbolTable};
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::tree::NodeId;
use linked_hash_map::LinkedHashMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// A symbol table used in `aiplan4rust` to store and manage symbols.
///
/// This structure manages a mapping between identifiers (`Ident`) and their corresponding
/// [`SymbolEntry`] metadata, which includes both declarations and usages of symbols.
/// It plays a key role in name resolution, scope tracking, and error reporting during semantic analysis.
///
/// The underlying data structure is a [`LinkedHashMap`] to preserve insertion order,
/// which is beneficial for deterministic behavior during compilation and debugging.
///
/// # Fields
/// - `symbols`: Ordered mapping from identifiers to `SymbolEntry` instances.
/// - `origin`: Indicates where this table was constructed from (e.g., domain file, problem file, merged).
/// - `root_id`: The root `NodeId` used to generate scope paths.
///
/// # Example
/// ```rust
/// let mut table = Table::default();
/// table.insert_symbol("my_symbol".into(), SymbolEntry::new(...));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    /// Map from symbol identifier to symbol metadata, preserving insertion order.
    pub(super) symbols: LinkedHashMap<SymbolId, SymbolEntry>,

    /// Indicates the origin context of the symbol table (e.g., Domain, Problem, Merged).
    pub(super) origin: SymbolTableOrigin,

    /// Root node ID used to construct the root scope.
    pub(super) root_id: NodeId,

    // --- AJOUT ICI ---
    // Permet de trouver instantanément le nom du symbole à partir d'un ID de nœud
    pub(super) usage_to_symbol: HashMap<NodeId, Usage>,
}

impl Default for Table {
    /// Creates a new, empty symbol table with default metadata.
    ///
    /// - The symbol map is initialized empty.
    /// - The origin is set to [`SymbolTableOrigin::None`].
    /// - The root node ID is set to the default [`NodeId`].
    ///
    /// # Returns
    /// A new default-initialized symbol table.
    ///
    /// # Example
    /// ```rust
    /// let table = Table::default();
    /// assert!(table.iter().next().is_none());
    /// ```
    fn default() -> Self {
        Table {
            symbols: LinkedHashMap::new(),
            origin: SymbolTableOrigin::default(),
            root_id: NodeId::default(),
            usage_to_symbol: HashMap::new(),
        }
    }
}

impl Table {
    /// Creates a new, empty `SymbolTable` with default values.
    ///
    /// # Returns
    /// A new `SymbolTable` instance with no symbols and a default origin.
    pub fn new() -> Self {
        Table::default()
    }

    /// Returns the origin metadata of the symbol table.
    ///
    /// The origin describes the context from which the symbol table was built—
    /// for example, a domain file, a problem file, or the result of merging both.
    ///
    /// # Returns
    /// The `SymbolTableOrigin` associated with this symbol table.
    pub fn origin(&self) -> SymbolTableOrigin {
        self.origin
    }

    /// Updates the origin metadata of the symbol table.
    ///
    /// This is typically used during linking or transformation steps
    /// when the origin context of the table changes (e.g., from Domain to Merged).
    ///
    /// # Parameters
    /// - `origin`: The new `SymbolTableOrigin` to assign.
    pub fn set_origin(&mut self, origin: SymbolTableOrigin) {
        self.origin = origin;
    }

    /// Returns the root scope of the symbol table.
    ///
    /// # Returns
    /// A `Scope` constructed from the symbol table's internal root ID.
    pub fn root_scope(&self) -> Scope {
        Scope::new(self.root_id, None)
    }

    /// Sets the internal root ID used to build the root scope.
    ///
    /// # Parameters
    /// - `root_id`: A `NodeId` representing the new root for scope generation.
    pub fn set_root_id(&mut self, root_id: NodeId) {
        self.root_id = root_id;
    }

    /// Returns an iter over all symbols in the table as immutable references.
    ///
    /// # Returns
    /// An iter yielding (`&Ident`, `&SymbolEntry`) pairs for all entries.
    pub fn iter(&self) -> impl Iterator<Item = (&SymbolId, &SymbolEntry)> {
        self.symbols.iter()
    }

    /// Returns an iter over all symbols in the table as mutable references.
    ///
    /// # Returns
    /// An iter yielding (`&Ident`, `&mut SymbolEntry`) pairs,
    /// allowing in-place modification of symbols.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&SymbolId, &mut SymbolEntry)> {
        self.symbols.iter_mut()
    }

    /// Inserts a symbol into the table under the specified key.
    ///
    /// # Parameters
    /// - `key`: The identifier (`Ident`) for the symbol.
    /// - `symbol`: The `SymbolEntry` to insert.
    pub fn insert_symbol(&mut self, key: SymbolId, symbol: SymbolEntry) {
        self.symbols.insert(key, symbol);
    }

    /// Retrieves an immutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `name`: The identifier (`Ident`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a reference to the symbol, or `None` if not found.
    pub fn get_symbol(&self, name: SymbolId) -> Option<&SymbolEntry> {
        self.symbols.get(&name)
    }

    /// Retrieves an immutable reference to a symbol by its ID, or returns an error if not found.
    ///
    /// This is the fallible version of `get_symbol`. It is preferred when a missing
    /// symbol should stop the current semantic pass and provide a traced error.
    ///
    /// # Parameters
    /// - `id`: The internal identifier ([`SymbolId`持) of the symbol to look up.
    ///
    /// # Returns
    /// - `Ok(&SymbolEntry)`: A reference to the found symbol entry.
    /// - `Err(SymbolTableError::SymbolNotFound)`: If the ID does not exist in the table.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError::SymbolNotFound`] with a captured trace via `#[track_caller]`.
    pub fn try_get_symbol(&self, id: SymbolId) -> Result<&SymbolEntry, SymbolTableError> {
        self.symbols
            .get(&id)
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))
    }

    /// Retrieves a mutable reference to a symbol by its name.
    ///
    /// # Parameters
    /// - `name`: The identifier (`Ident`) of the symbol.
    ///
    /// # Returns
    /// An `Option` containing a mutable reference to the symbol, or `None` if not found.
    pub fn get_symbol_mut(&mut self, name: SymbolId) -> Option<&mut SymbolEntry> {
        self.symbols.get_mut(&name)
    }

    /// Retrieves a mutable reference to a symbol by its ID, or returns an error if not found.
    ///
    /// This method is the fallible counterpart to `get_symbol_mut`. It uses the internal
    /// error factory to capture a trace if the symbol is missing.
    ///
    /// # Parameters
    /// - `id`: The internal identifier ([`SymbolId`持) of the symbol to look up.
    ///
    /// # Returns
    /// - `Ok(&mut SymbolEntry)`: A mutable reference to the found symbol entry.
    /// - `Err(SymbolTableError::SymbolNotFound)`: If no symbol matches the provided ID.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError::SymbolNotFound`] with a captured trace via `#[track_caller]`.
    pub fn try_get_symbol_mut(
        &mut self,
        id: SymbolId,
    ) -> Result<&mut SymbolEntry, SymbolTableError> {
        self.symbols
            .get_mut(&id)
            .ok_or_else(|| SymbolTableError::symbol_not_found(id))
    }

    pub fn get_declaration_from(
        &self,
        symbol_id: SymbolId,
        node_id: NodeId,
    ) -> Option<&Declaration> {
        // Utilisation de get_symbol (et non get_symbol_mut) car la fonction est &self
        self.get_symbol(symbol_id)?.declarations().get(&node_id)
    }

    pub fn get_usage_from(&self, symbol_id: SymbolId, node_id: NodeId) -> Option<&Usage> {
        // Utilisation de get_symbol (et non get_symbol_mut) car la fonction est &self
        self.get_symbol(symbol_id)?.usages().get(&node_id)
    }

    pub fn try_get_declaration_from(
        &self,
        symbol_id: SymbolId,
        node_id: NodeId,
    ) -> Result<&Declaration, SymbolTableError> {
        self.try_get_symbol(symbol_id)?
            .declarations()
            .get(&node_id)
            // On utilise l'erreur existante qui prend juste le NodeId
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    pub fn try_get_usage_from(
        &self,
        symbol_id: SymbolId,
        node_id: NodeId,
    ) -> Result<&Usage, SymbolTableError> {
        self.try_get_symbol(symbol_id)?
            .usages()
            .get(&node_id)
            // On utilise l'erreur existante qui prend juste le NodeId
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    /// Récupère une référence mutable vers une déclaration (Option).
    pub fn get_declaration_from_mut(
        &mut self,
        symbol_id: SymbolId,
        node_id: NodeId,
    ) -> Option<&mut Declaration> {
        self.get_symbol_mut(symbol_id)?
            .declarations_mut()
            .get_mut(&node_id)
    }

    /// Récupère une référence mutable vers une déclaration (Result).
    pub fn try_get_declaration_from_mut(
        &mut self,
        symbol_id: SymbolId,
        node_id: NodeId,
    ) -> Result<&mut Declaration, SymbolTableError> {
        self.try_get_symbol_mut(symbol_id)?
            .declarations_mut()
            .get_mut(&node_id)
            .ok_or_else(|| SymbolTableError::declaration_not_found_for_node(node_id))
    }

    /// Returns an iter over all symbols in the table as immutable references.
    ///
    /// # Returns
    /// An iter yielding references to each `SymbolEntry` in the table.
    pub fn values(&self) -> impl Iterator<Item = &SymbolEntry> {
        self.symbols.values()
    }

    /// Returns a mutable iter over all symbols in the table.
    ///
    /// # Returns
    /// An iter yielding mutable references to each `SymbolEntry`.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut SymbolEntry> {
        self.symbols.iter_mut().map(|(_, symbol)| symbol)
    }

    /// This method scans all symbols of kind [`SymbolKind::PrimitiveType`],
    /// collects their parent declarations, and packages them into a
    /// standalone [`TypeHierarchy`].
    ///
    /// The resulting object is independent of the table's lifetime,
    /// allowing it to be used by the `TypeChecker` without causing
    /// borrow checker conflicts.
    ///
    /// # Returns
    /// A [`TypeHierarchy`] reflecting the state of the types in this table.
    pub fn to_type_hierarchy(&self) -> TypeHierarchy {
        let mut hierarchy_map: HashMap<SymbolId, Vec<SymbolId>> =
            HashMap::with_capacity(self.symbols.len() / 4);

        for (&id, entry) in self.symbols.iter() {
            for decl in entry.declarations().values() {
                if decl.kind() == SymbolKind::PrimitiveType {
                    // Étape 1 : On récupère les parents s'ils existent
                    if let Some(ty) = decl.ty() {
                        let parents = ty.members();

                        // Étape 2 : On insère les parents pour l'enfant actuel
                        // On ne stocke pas le résultat de .entry() dans une variable longue durée
                        hierarchy_map
                            .entry(id)
                            .or_default()
                            .extend(parents.iter().cloned());

                        // Étape 3 : On s'assure que chaque parent a sa propre entrée
                        for &parent_id in parents {
                            hierarchy_map.entry(parent_id).or_default();
                        }
                    } else {
                        // Si pas de parents, on s'assure quand même que le type existe (root)
                        hierarchy_map.entry(id).or_default();
                    }
                }
            }
        }
        TypeHierarchy::new(hierarchy_map)
    }
}

impl RemapSymbol for SymbolTable {
    /// Remaps the identifiers (`Ident`) in the symbol table according to the provided mapping.
    ///
    /// # Parameters
    /// - `map`: A `HashMap` mapping old identifiers (`Ident`) to their corresponding new identifiers (`Ident`).
    ///
    /// # Returns
    /// - `Ok(())` if all identifiers were successfully remapped.
    /// - `Err(InternerError)` if a conflict or missing mapping occurs during remapping.
    fn remap_symbol(&mut self, map: &HashMap<SymbolId, SymbolId>) -> Result<(), InternerError> {
        // Step 1: Internally remap symbols
        for (_key, symbol) in self.symbols.iter_mut() {
            symbol.remap_symbol(map)?;
        }

        // Step 2: Rebuild the map with remapped keys
        let mut new_symbols = LinkedHashMap::with_capacity(self.symbols.len());

        for (key, symbol) in std::mem::take(&mut self.symbols) {
            let new_key = map
                .get(&key)
                .cloned()
                .ok_or_else(|| InternerError::missing_ident(key))?;

            if new_symbols.contains_key(&new_key) {
                return Err(InternerError::conflict(new_key))?;
            }

            new_symbols.insert(new_key, symbol);
        }

        self.symbols = new_symbols;
        Ok(())
    }
}

impl TryFrom<&Ast> for SymbolTable {
    type Error = SemanticError;

    /// Attempts to construct a `SymbolTable` from a reference to an abstract syntax tree (`Ast`).
    ///
    /// This method performs semantic analysis by building the symbol table from the given AST.
    /// It leverages the `SymbolTableBuilder` internally to perform the construction.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the abstract syntax tree from which the symbol table is derived.
    ///
    /// # Returns
    ///
    /// * `Ok(SymbolTable)` if the symbol table is successfully built.
    /// * `Err(SymbolTableError)` if any semantic or construction error occurs during the build process.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::convert::TryFrom;
    ///
    /// let ast = ...; // Assume you have an `Ast` instance
    /// let symbol_table = SymbolTable::try_from(&ast)?;
    /// ```
    fn try_from(ast: &Ast) -> Result<Self, Self::Error> {
        let mut builder = SymbolTableBuilder::new();
        builder.build(ast)
    }
}

impl fmt::Display for Table {
    /// Formats the entire `Table` as a user-friendly string.
    ///
    /// This method implements the `Display` trait for the `Table` typing,
    /// allowing the table to be printed or logged conveniently. It
    /// formats each symbol stored in the table on its own line.
    ///
    /// # Parameters
    /// - `f`: The formatter to write the formatted string into.
    ///
    /// # Returns
    /// Returns a `fmt::Result` indicating success or failure during formatting.
    ///
    /// # Example
    /// ```
    /// let symbol_table = Table::new();
    /// println!("{}", symbol_table); // Prints all symbols line by line.
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Iterate over all symbols stored in the symbol table.
        for symbol in self.symbols.values() {
            // Write each symbol followed by a newline into the formatter.
            // The `?` operator propagates any error that might occur.
            writeln!(f, "{}", symbol)?;
        }
        // Return Ok to indicate successful formatting.
        Ok(())
    }
}

impl InternerDisplay for Table {
    /// Formats the symbol table using the provided string interner.
    ///
    /// This method iterates over all symbols stored in the table and writes their
    /// formatted representation to the given formatter, using the provided string interner
    /// to resolve symbol names or other interned strings.
    ///
    /// Each symbol's formatting is delegated to its own `fmt_with_interner` method.
    /// Symbols are separated by newlines in the output.
    ///
    /// # Parameters
    /// - `f`: The formatter to write the output to.
    /// - `interner`: A `StringInterner` used to resolve interned string IDs into strings.
    ///
    /// # Returns
    /// Returns `fmt::Result`, indicating success or failure during formatting.
    ///
    /// # Example
    /// ```rust
    /// use crate::aiplan4rust::semantic::symbol_table::Table;
    /// use crate::aiplan4rust::interner::{StringInterner, InternerDisplay};
    ///
    /// let table = Table::new();
    /// let interner = StringInterner::new();
    /// // ... populate table and interner ...
    /// println!("{}", table.fmt_with_interner(&mut std::fmt::Formatter::new(), &interner).unwrap());
    /// ```
    fn fmt_with_interner(
        &self,
        f: &mut fmt::Formatter<'_>,
        interner: &SymbolInterner,
    ) -> fmt::Result {
        for symbol in self.symbols.values() {
            symbol.fmt_with_interner(f, interner)?;
            writeln!(f)?;
        }
        Ok(())
    }
}
