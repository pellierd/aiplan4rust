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
use crate::aiplan4rust::lang::{SymbolId, RemapSymbol};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Filterable;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolEntry;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol_table::{SymbolTableBuilder, SymbolTableError, SymbolTableOrigin};
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::tree::NodeId;
use crate::aiplan4rust::semantic::SymbolTable;
use linked_hash_map::LinkedHashMap;
use serde::Deserialize;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
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
    symbols: LinkedHashMap<SymbolId, SymbolEntry>,

    /// Indicates the origin context of the symbol table (e.g., Domain, Problem, Merged).
    origin: SymbolTableOrigin,

    /// Root node ID used to construct the root scope.
    root_id: NodeId,

    // --- AJOUT ICI ---
    // Permet de trouver instantanément le nom du symbole à partir d'un ID de nœud
    pub usage_to_symbol: HashMap<NodeId, Usage>,
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
    pub fn iter(&self) -> impl Iterator<Item=(&SymbolId, &SymbolEntry)> {
        self.symbols.iter()
    }

    /// Returns an iter over all symbols in the table as mutable references.
    ///
    /// # Returns
    /// An iter yielding (`&Ident`, `&mut SymbolEntry`) pairs,
    /// allowing in-place modification of symbols.
    pub fn iter_mut(&mut self) -> impl Iterator<Item=(&SymbolId, &mut SymbolEntry)> {
        self.symbols.iter_mut()
    }

    /// Consumes the symbol table and returns an iter over its entries.
    ///
    /// # Returns
    /// An iter yielding `(Ident, SymbolEntry)` pairs, consuming the table.
    pub fn into_iter(self) -> impl Iterator<Item=(SymbolId, SymbolEntry)> {
        self.symbols.into_iter()
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

    /// Returns an iter over all symbols in the table as immutable references.
    ///
    /// # Returns
    /// An iter yielding references to each `SymbolEntry` in the table.
    pub fn values(&self) -> impl Iterator<Item=&SymbolEntry> {
        self.symbols.values()
    }

    /// Returns a mutable iter over all symbols in the table.
    ///
    /// # Returns
    /// An iter yielding mutable references to each `SymbolEntry`.
    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut SymbolEntry> {
        self.symbols.iter_mut().map(|(_, symbol)| symbol)
    }

    /// Collects symbols from the symbol table that have at least one declaration matching the provided filters.
    ///
    /// This method searches the symbol table for entries whose declarations match the given optional criteria:
    /// - the symbol's name (`symbol_name`),
    /// - the kind of declaration (`kind`),
    /// - and the declaration scope (`scope`).
    ///
    /// If a specific `symbol_name` is provided, the method attempts a direct lookup of that symbol and filters
    /// its declarations accordingly. If no name is provided, it scans all symbols in the table.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional reference to an `Ident` to restrict the search to a specific symbol.
    /// - `kind`: An optional reference to a `SymbolKind` to filter declarations by kind.
    /// - `scope`: An optional reference to a `Scope` to filter declarations by scope prefix.
    ///
    /// # Returns
    /// A vector of references to `SymbolEntry` instances that have at least one declaration matching
    /// the specified filters. If no matches are found, an empty vector is returned.
    ///
    /// # Examples
    /// ```rust
    /// // Fetch the symbol "mySymbol" if it has any Predicate declarations in the "global" scope
    /// let symbols = symbol_table.collect_symbol_with_declaration(
    ///     Some(&Ident::new("mySymbol")),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global"))
    /// );
    ///
    /// // Fetch all symbols with any Function declaration regardless of name or scope
    /// let symbols = symbol_table.collect_symbol_with_declaration(
    ///     None,
    ///     Some(&SymbolKind::Function),
    ///     None
    /// );
    /// ```
    pub fn collect_symbol_with_declaration(
        &self,
        symbol_name: Option<&SymbolId>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&SymbolEntry> {
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                let declarations = symbol.declarations();
                let filtered = Table::collect(kind, scope, declarations);
                if !filtered.is_empty() {
                    return vec![symbol];
                }
                return Vec::new();
            }
            return Vec::new();
        }

        self.symbols
            .values()
            .filter(|symbol| {
                let declarations = symbol.declarations();
                !Table::collect(kind, scope, declarations).is_empty()
            })
            .collect()
    }


    /// Collects symbols that have at least one usage matching the provided optional filters.
    ///
    /// This method searches the symbol table for entries with usages filtered by:
    /// - the symbol's name (`symbol_name`),
    /// - the kind of usage (`kind`),
    /// - and the usage scope (`scope`).
    ///
    /// If a specific `symbol_name` is provided, the lookup is optimized by directly accessing that
    /// symbol and checking whether it has any matching usages. If not provided, all symbols in the
    /// table are scanned.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional reference to an `Ident` to restrict the search to a specific symbol.
    /// - `kind`: An optional reference to a `SymbolKind` to filter usages by kind.
    /// - `scope`: An optional reference to a `Scope` to filter usages by scope prefix.
    ///
    /// # Returns
    /// A vector of references to `SymbolEntry` instances that have at least one usage matching
    /// all specified filters.
    ///
    /// # Behavior
    /// - If `symbol_name` is provided:
    ///   - Attempts to retrieve the symbol directly.
    ///   - Filters its usages by `kind` and `scope`.
    ///   - Returns a vector containing the symbol if any matching usage exists.
    ///   - Returns an empty vector if no match is found or the symbol doesn't exist.
    /// - If `symbol_name` is `None`:
    ///   - Iterates through all symbols in the table.
    ///   - Collects and returns those with at least one matching usage.
    ///
    /// # Examples
    /// ```rust
    /// // Fetch the symbol "foo" if it has at least one Predicate usage in the global scope
    /// let symbols = symbol_table.collect_symbol_with_usages(
    ///     Some(&Ident::new("foo")),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global"))
    /// );
    ///
    /// // Fetch all symbols with any Action usage regardless of name or scope
    /// let symbols = symbol_table.collect_symbol_with_usages(None, Some(&SymbolKind::Action), None);
    /// ```
    pub fn collect_symbol_with_usages(
        &self,
        symbol_name: Option<&SymbolId>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&SymbolEntry> {
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                let usages = symbol.usages();
                let filtered_usages = Table::collect(kind, scope, usages);
                if !filtered_usages.is_empty() {
                    return vec![symbol];
                }
            }
            return Vec::new();
        }

        self.symbols
            .values()
            .filter(|symbol| {
                let usages = symbol.usages();
                !Table::collect(kind, scope, usages).is_empty()
            })
            .collect()
    }

    /// Filters and collects declarations from the symbol table based on optional criteria.
    ///
    /// This method retrieves declarations that match the provided filters:
    /// - `symbol_name`: if specified, restricts the search to declarations of that symbol only.
    /// - `kind`: if specified, filters declarations by their kind (e.g., `Predicate`, `Task`).
    /// - `scope`: if specified, filters declarations by the scope in which they are defined.
    ///
    /// If no `symbol_name` is given, the search is performed over all symbols in the table.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional reference to an `Ident` to restrict the search to declarations of that symbol.
    /// - `kind`: An optional reference to a `SymbolKind` to filter declarations by kind.
    /// - `scope`: An optional reference to a `Scope` to filter declarations by scope.
    ///
    /// # Returns
    /// A vector of references to `Declaration` instances matching all specified criteria.
    ///
    /// # Examples
    /// ```rust
    /// // Collect all Predicate declarations for the symbol "my_symbol"
    /// let predicates = symbol_table.collect_declarations(
    ///     Some(&Ident::new("my_symbol")),
    ///     Some(&SymbolKind::Predicate),
    ///     None,
    /// );
    ///
    /// // Collect all Task declarations across all symbols within a specific scope
    /// let scoped_tasks = symbol_table.collect_declarations(
    ///     None,
    ///     Some(&SymbolKind::Task),
    ///     Some(&Scope::new("global")),
    /// );
    /// ```
    pub fn collect_declarations(
        &self,
        symbol_name: Option<&SymbolId>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                return Table::collect(kind, scope, symbol.declarations());
            } else {
                return Vec::new();
            }
        }

        self.symbols
            .values()
            .flat_map(|symbol| Table::collect(kind, scope, symbol.declarations()))
            .collect()
    }

    /// Filters and collects usages of symbols based on optional criteria.
    ///
    /// This method retrieves usages of symbols stored in the symbol table, optionally filtered by:
    /// - the symbol's name,
    /// - the kind of usage (e.g., `Predicate`, `Function`, `Task`),
    /// - and the scope within which the usage occurs.
    ///
    /// If a specific `symbol_name` is provided, only usages of that symbol are considered.
    /// Otherwise, usages of all symbols in the table are filtered.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional reference to an `Ident` to restrict the search to usages of that symbol.
    /// - `kind`: An optional reference to a `SymbolKind` to filter usages by their kind.
    /// - `scope`: An optional reference to a `Scope` that serves as a prefix filter on usage scopes.
    ///
    /// # Returns
    /// A vector of references to all `Usage` instances that match the provided filters.
    ///
    /// # Examples
    /// ```
    /// // Filter usages of a specific symbol "my_symbol" that are of kind Predicate and within "global" scope.
    /// let filtered_usages = symbol_table.collect_usages(
    ///     Some(&Ident::new("my_symbol")),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global")),
    /// );
    ///
    /// // Filter all usages of kind Function across all symbols, ignoring symbol name and scope.
    /// let all_function_usages = symbol_table.collect_usages(None, Some(&SymbolKind::Function), None);
    /// ```
    pub fn collect_usages(
        &self,
        symbol_name: Option<&SymbolId>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                return Table::collect(kind, scope, symbol.usages());
            } else {
                return Vec::new();
            }
        }

        self.symbols.values()
            .flat_map(|symbol| Table::collect(kind, scope, symbol.usages()))
            .collect()
    }

    /// Filters a collection of symbols or symbol-like items based on optional kind and scope criteria.
    ///
    /// This function iterates over the provided collection of items implementing the `Filterable` trait,
    /// returning only those that match the specified optional `kind` and `scope` filters. If a filter
    /// is `None`, it is ignored.
    ///
    /// # Parameters
    /// - `kind`: An optional reference to a `SymbolKind` to filter by. If provided, only items matching
    ///   this kind are included.
    /// - `scope`: An optional reference to a `Scope` to filter by. If provided, only items whose scope
    ///   starts with this scope prefix are included.
    /// - `items`: A reference to a `HashSet` of items implementing `Filterable` to filter.
    ///
    /// # Returns
    /// A vector of references to the filtered items.
    ///
    /// # Example
    /// ```rust
    /// let filtered = collect(
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global")),
    ///     &my_items,
    /// );
    /// ```
    fn collect<'a, T: Filterable>(
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
        items: &'a HashSet<T>,
    ) -> Vec<&'a T> {
        items.iter()
            .filter(|item| {
                if let Some(k) = kind {
                    if item.kind() != *k {
                        return false;
                    }
                }
                if let Some(s) = scope {
                    if !s.starts_with(item.scope()) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    /// Retrieves the declaration associated with a usage identified by a specific AST syntax node ID.
    ///
    /// This function searches through all symbols and their usages in the symbol table to find a usage
    /// that matches the given AST node ID. Once found, it filters the declarations of that symbol to find
    /// those whose scope contains the usage's scope, ensuring the declaration is valid in context.
    ///
    /// # Parameters
    /// - `node_id`: The AST `NodeId` representing the usage to resolve.
    ///
    /// # Returns
    /// Returns a `Result` wrapping an `Option`:
    /// - `Ok(Some(&Declaration))`: If exactly one matching declaration was found.
    /// - `Ok(None)`: If no matching usage or declaration was found.
    /// - `Err(SymbolTableError)`: If multiple declarations match the usage, indicating ambiguous resolution.
    ///
    /// # Errors
    /// Returns [`SymbolTableError::AmbiguousUsage`] if more than one declaration
    /// matches the given usage node. This signals an ambiguous resolution.
    ///
    /// # Examples
    /// ```rust
    /// let declaration = symbol_table.resolve_declaration_by_usage(node_id)?;
    /// match declaration {
    ///     Some(decl) => println!("Declaration: {:?}", decl),
    ///     None => println!("No declaration found."),
    /// }
    /// ```
    pub fn resolve_declaration_by_usage(
        &self,
        node_id: NodeId,
        expected_kind: SymbolKind,
    ) -> Result<Option<&Declaration>, SymbolTableError> {
        // 1. Accès O(1) : On récupère le StringID et l'Usage via l'index de performance
        let usage = match self.usage_to_symbol.get(&node_id) {
            Some(data) => data,
            None => return Ok(None),
        };

        // 2. Accès O(1) : On récupère l'entrée du symbole (SymbolEntry)
        // On utilise ok_or pour éviter un unwrap() risqué
        let symbol_entry = self.symbols.get(&usage.symbol_id()).unwrap();

        let declarations = symbol_entry.declarations();
        let u_scope = usage.scope();
        let u_len = u_scope.len();

        // 3. OPTIMISATION : Fast Path pour le cas majoritaire (une seule déclaration)
        // On évite de créer un itérateur et une closure.
        if declarations.len() == 1 {
            // On récupère l'unique élément sans créer de Filter
            if let Some(decl) = declarations.iter().next() {
                let d_scope = decl.scope();
                if d_scope.len() <= u_len
                    && decl.kind() == expected_kind
                    && u_scope.starts_with(d_scope)
                {
                    return Ok(Some(decl));
                }
            }
            return Ok(None);
        }

        // 4. Cas des surcharges (D > 1) : Filtrage avec gestion de l'ambiguïté
        let mut matching_iter = declarations.iter().filter(|decl| {
            let d_scope = decl.scope();
            // Le test de longueur ici aussi évite des comparaisons de vecteurs inutiles
            d_scope.len() <= u_len
                && decl.kind() == expected_kind
                && u_scope.starts_with(d_scope)
        });

        // On récupère le premier match
        let first_match = matching_iter.next();

        // Gestion de l'ambiguïté : si un deuxième élément correspond dans le même contexte
        if let Some(_second_match) = matching_iter.next() {
            return Err(SymbolTableError::ambiguous_usage(node_id, vec![]));
        }

        Ok(first_match)
    }

    /// Reconstruit l'index de performance à partir des données de la table.
    /// À appeler après un merge ou un remapping.
    /// À appeler après un merge ou un remapping.
    pub fn rebuild_usage_index(&mut self) {
        // 1. On vide l'index actuel
        self.usage_to_symbol.clear();

        // 2. On parcourt toutes les entrées de symboles
        // On n'a plus besoin du `ident` ici puisque l'Usage le contient déjà
        for entry in self.symbols.values() {
            // 3. Pour chaque symbole, on parcourt ses usages
            for usage in entry.usages() {
                // L'index ne contient plus que le NodeId vers l'Usage
                self.usage_to_symbol.insert(
                    usage.node_id(),
                    usage.clone()
                );
            }
        }
    }

    /// Resolves the unique declaration associated with a usage node ID, or returns an error if ambiguous or missing.
    ///
    /// This function delegates to `resolve_declaration_by_usage` and upgrades the result:
    /// - Returns the declaration if found.
    /// - Returns [`SymbolTableError::DeclarationNotFoundForUsage`] if no matching declaration is found.
    /// - Propagates any other error.
    ///
    /// # Parameters
    /// - `node_id`: The `NodeId` of the usage in the AST.
    ///
    /// # Returns
    /// - `Ok(&Declaration)`: If exactly one matching declaration is found.
    /// - `Err(SymbolTableError)`: If no declaration is found or multiple declarations match.
    ///
    /// # Example
    /// ```rust
    /// let decl = symbol_table.try_resolve_declaration_by_usage(node_id)?;
    /// println!("Declaration: {:?}", decl);
    /// ```
    pub fn try_resolve_declaration_by_usage(
        &self,
        node_id: NodeId,
        expected_kind: SymbolKind,
    ) -> Result<&Declaration, SymbolTableError> {
        match self.resolve_declaration_by_usage(node_id, expected_kind)? {
            Some(decl) => Ok(decl),
            None => Err(SymbolTableError::declaration_not_found_for_usage(node_id)),
        }
    }

    /// Resolves the most appropriate declaration for a given symbol usage.
    ///
    /// This function attempts to find a declaration matching the provided symbol name, usage kind,
    /// and scope. It performs filtering and validation to ensure that the returned declaration
    /// is unambiguous and valid in the current context.
    ///
    /// Specifically, for symbols of kind `Task`, this method includes a fallback mechanism:
    /// if no declaration of kind `Task` is found, it attempts to resolve a declaration of kind
    /// `Action`.
    ///
    /// # Parameters
    /// - `symbol_name`: The name of the symbol to resolve. This should correspond to a declared
    ///   symbol.
    /// - `usage_kind`: The kind of usage for the symbol (e.g., `Predicate`, `Task`, `Action`). This
    ///   guides the filtering and validation ops to pick an appropriate declaration.
    /// - `scope`: The lexical or semantic scope in which the symbol usage occurs. Only declarations
    ///   within or compatible with this scope are considered.
    ///
    /// # Returns
    /// Returns a `Result` wrapping an `Option`:
    /// - `Ok(Some(&Declaration))`: If exactly one valid declaration matching the criteria is found.
    /// - `Ok(None)`: If no matching declaration exists for the given filters.
    /// - `Err(SymbolTableError)`: If multiple conflicting or ambiguous declarations are found,
    ///   making it impossible to resolve a unique declaration.
    ///
    /// # Errors
    /// This function returns a [`SymbolTableError::DuplicateDeclarationForUnique`] if multiple declarations
    /// match the criteria and no clear resolution can be made. This error helps identify semantic
    /// issues such as duplicated or conflicting declarations.
    ///
    /// # Behavior Details
    /// - The resolution process uses [`collect_declarations`] internally to collect candidate
    ///   declarations.
    /// - Validation and disambiguation of these candidates are done by [`select_valid_declaration`].
    /// - The special fallback from `Task` to `Action` allows flexible handling of task-like symbols.
    ///
    /// # Examples
    /// ```rust
    /// let symbol_name = Ident::new("move");
    /// let usage_kind = SymbolKind::Task;
    /// let scope = Scope::new("global");
    ///
    /// match symbol_table.resolve_declaration(&symbol_name, &usage_kind, &scope) {
    ///     Ok(Some(decl)) => println!("Resolved declaration: {:?}", decl),
    ///     Ok(None) => println!("No matching declaration found."),
    ///     Err(err) => eprintln!("Error resolving declaration: {}", err),
    /// }
    /// ```
    ///
    /// [`collect_declarations`]: Table::collect_declarations
    /// [`select_valid_declaration`]: Table::select_valid_declaration
    pub fn resolve_declaration(
        &self,
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<Option<&Declaration>, SymbolTableError> {
        // Define a closure to fetch and validate declarations for a specific SymbolKind
        let resolve_candidates = |kind: SymbolKind| {
            // Fetch declarations matching symbol_name, kind, and scope
            let decls = self.collect_declarations(Some(symbol_name), Some(&kind), Some(scope));
            // Select the most valid declaration from the fetched list
            Self::select_valid_declaration(symbol_name, usage_kind, &decls)
        };

        // Special case handling when usage_kind is Task
        match usage_kind {
            SymbolKind::Task => match resolve_candidates(SymbolKind::Task)? {
                // If a Task declaration is found, return it
                Some(decl) => Ok(Some(decl)),
                // Otherwise, fallback to resolving Action declarations
                None => resolve_candidates(SymbolKind::Action),
            },
            // For other usage kinds, resolve declarations directly
            _ => resolve_candidates(*usage_kind),
        }
    }

    /// Attempts to resolve a unique declaration for a given symbol usage, returning an error if none is found.
    ///
    /// This function calls [`resolve_declaration`] internally to attempt to find a declaration
    /// matching the provided symbol name, usage kind, and scope. Unlike `resolve_declaration`
    /// which returns an `Option`, this method converts the `None` case into a `SymbolTableError`
    /// of typing `SymbolDeclarationNotFound`, enforcing that a declaration *must* be found.
    ///
    /// # Parameters
    /// - `symbol_name`: The identifier of the symbol to resolve.
    /// - `usage_kind`: The kind of usage (e.g., `Predicate`, `Task`, `Action`) for filtering declarations.
    /// - `scope`: The lexical or semantic scope where the symbol usage occurs.
    ///
    /// # Returns
    /// - `Ok(&Declaration)`: The uniquely resolved declaration matching the criteria.
    /// - `Err(SymbolTableError)`: If no matching declaration is found or if there are multiple ambiguous matches.
    ///
    /// # Errors
    /// Returns [`SymbolTableError::DeclarationNotFound`] if no matching declaration exists
    /// for the given symbol, kind, and scope.
    ///
    /// # Examples
    /// ```rust
    /// let decl = symbol_table.try_resolve_declaration(&symbol_name, &usage_kind, &scope)?;
    /// println!("Resolved declaration: {:?}", decl);
    /// ```
    ///
    /// [`resolve_declaration`]: Self::resolve_declaration
    pub fn try_resolve_declaration(
        &self,
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<&Declaration, SymbolTableError> {
        match self.resolve_declaration(symbol_name, usage_kind, scope)? {
            Some(decl) => Ok(decl),
            None => Err(SymbolTableError::declaration_not_found(
                symbol_name.clone(),
                usage_kind.clone(),
                scope.clone(),
            )),
        }
    }

    /// Selects a valid declaration among candidates for a given symbol usage.
    ///
    /// Applies kind-specific rules to determine which declaration is valid and unambiguous.
    /// Used internally by `resolve_declaration`.
    ///
    /// # Parameters
    /// - `symbol_name`: The name of the symbol.
    /// - `usage_kind`: The kind of usage that triggered the lookup.
    /// - `declarations`: All candidate declarations matching the symbol, kind, and scope.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: If one valid declaration is found.
    /// - `Ok(None)`: If no declaration is acceptable.
    /// - `Err`: If multiple valid declarations cause ambiguity.
    fn select_valid_declaration<'a>(
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, SymbolTableError> {
        // Match on the usage kind to determine the appropriate validation strategy
        match usage_kind {
            // For PrimitiveType or Predicate kinds, use specific validation ops
            SymbolKind::PrimitiveType | SymbolKind::Predicate => {
                Self::validate_type_or_predicate_declarations(symbol_name, usage_kind, declarations)
            }
            // For Task kind, use task-specific validation ops
            SymbolKind::Task => {
                Self::validate_task_declarations(symbol_name, declarations)
            }
            // For all other kinds, handle based on the number of declarations found
            _ => match declarations.len() {
                0 => Ok(None), // No declarations found
                1 => Ok(Some(declarations[0])), // Exactly one declaration found, return it
                // Multiple declarations found, return an error indicating ambiguity
                _ => Err(SymbolTableError::duplicate_declaration(*symbol_name, *usage_kind, declarations.len())),
            },
        }
    }

    /// Determines whether a declaration kind is compatible with a usage kind.
    ///
    /// Used to allow limited polymorphism (e.g., using a `Predicate` in a type_checker context).
    ///
    /// # Returns
    /// `true` if the declaration kind is allowed for the given usage kind.
    fn is_declaration_kind_compatible(usage_kind: &SymbolKind, decl_kind: &SymbolKind) -> bool {
        // Determine compatibility between the usage kind and declaration kind
        match usage_kind {
            // For PrimitiveType usage, compatible with PrimitiveType or Predicate declarations
            SymbolKind::PrimitiveType => {
                *decl_kind == SymbolKind::PrimitiveType || *decl_kind == SymbolKind::Predicate
            },
            // For Predicate usage, compatible with Predicate or PrimitiveType declarations
            SymbolKind::Predicate => {
                *decl_kind == SymbolKind::Predicate || *decl_kind == SymbolKind::PrimitiveType
            },
            // For Task usage, compatible with Task or Action declarations
            SymbolKind::Task => {
                *decl_kind == SymbolKind::Task || *decl_kind == SymbolKind::Action
            },
            // For other usage kinds, no compatibility by default
            _ => false,
        }
    }

    /// Validates declarations for types and predicates, allowing limited overlap.
    ///
    /// Allows exactly one matching declaration and optionally one compatible declaration (e.g.,
    /// Predicate + PrimitiveType).
    ///
    /// # Rules
    /// - Only one declaration with the correct kind is allowed.
    /// - One other compatible kind may exist, but not more.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: If validation logic.
    /// - `Ok(None)`: If no matching declaration exists.
    /// - `Err`: If validation fails due to ambiguity or incompatible kinds.
    fn validate_type_or_predicate_declarations<'a>(
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, SymbolTableError> {
        // Collect all declarations that exactly match the usage kind
        let matching: Vec<_> = declarations.iter().filter(|d| d.symbol_kind() == *usage_kind).collect();

        match matching.len() {
            // No matching declarations found
            0 => Ok(None),
            // Exactly one matching declaration found
            1 => match declarations.len() {
                // If there is only one declaration in total, return the matching one
                1 => Ok(Some(matching[0])),
                // If there are exactly two declarations in total, check the other one
                2 => match declarations.iter().find(|d| d.symbol_kind() != *usage_kind) {
                    // If the other declaration kind is compatible, still return the matching one
                    Some(other) if Self::is_declaration_kind_compatible(usage_kind, &other.symbol_kind()) => Ok(Some(matching[0])),
                    // Otherwise, multiple conflicting declarations are found — return an error
                    _ => Err(SymbolTableError::duplicate_declaration(*symbol_name, *usage_kind, declarations.len())),
                },
                // More than two declarations in total means ambiguity — return an error
                _ => Err(SymbolTableError::duplicate_declaration(*symbol_name, *usage_kind, declarations.len())),
            },
            // More than one matching declaration is ambiguous — return an error
            _ => Err(SymbolTableError::duplicate_declaration(*symbol_name, *usage_kind, declarations.len())),
        }
    }

    /// Validates task declarations, allowing exactly one declaration of kind `Task` or `Action`.
    ///
    /// If the declaration kind is incompatible, it will be ignored.
    ///
    /// # Parameters
    /// - `symbol_name`: The identifier of the symbol being validated.
    /// - `declarations`: A slice of references to declarations associated with the symbol.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))` if exactly one valid declaration (`Task` or `Action`) is found.
    /// - `Ok(None)` if no valid declarations are found or if declarations are incompatible.
    /// - `Err(SymbolTableError)` if multiple declarations cause ambiguity.
    ///
    /// # Errors
    /// Returns an error if more than one declaration exists, indicating ambiguous task declarations.
    fn validate_task_declarations<'a>(
        symbol_name: &SymbolId,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, SymbolTableError> {
        // If there is more than one declaration, return an error indicating ambiguity
        if declarations.len() > 1 {
            return Err(SymbolTableError::duplicate_declaration(
                *symbol_name,
                SymbolKind::Task,
                declarations.len(),
            ));
        }

        // Check the first (and only) declaration if it exists
        match declarations.first() {
            Some(decl) => match decl.symbol_kind() {
                // If the declaration kind is Task or Action, return it as valid
                SymbolKind::Task | SymbolKind::Action => Ok(Some(decl)),
                // Otherwise, no valid declaration found; return None
                _ => Ok(None),
            },
            // No declarations found, so return None
            None => Ok(None),
        }
    }

    /// Resolves a unique declaration of a specific symbol kind assumed to appear only once in the AST.
    ///
    /// This function is intended for resolving symbols that are expected to have exactly one declaration
    /// in a well-formed AST, such as a `DomainName` or `ProblemName`. It collects all declarations of
    /// the specified kind and returns:
    ///
    /// - The declaration if exactly one is found.
    /// - `None` if no declaration is found.
    /// - An error if multiple declarations are found, indicating an invalid or ambiguous AST.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of symbol to resolve, typically one that is required to be unique.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(&Declaration))` if exactly one declaration of the specified kind is found.
    /// * `Ok(None)` if no such declaration exists.
    /// * `Err(SymbolTableError::DuplicateDeclarationForUnique)` if multiple declarations are found,
    ///    indicating a semantic or structural error in the AST.
    ///
    /// # Errors
    ///
    /// Returns a [`SymbolTableError::DuplicateDeclarationForUnique`] if more than one declaration of the given
    /// `SymbolKind` is found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let domain_decl = symbol_table.resolve_unique_declaration(SymbolKind::DomainName)?;
    /// match domain_decl {
    ///     Some(decl) => println!("Domain declaration found at span: {:?}", decl.span()),
    ///     None => println!("No domain declared."),
    /// }
    /// ```
    pub fn resolve_unique_declaration(
        &self,
        kind: SymbolKind,
    ) -> Result<Option<&Declaration>, SymbolTableError> {
        let declarations = self.collect_declarations(
            None,
            Some(&kind),
            Some(&self.root_scope()),
        );

        match declarations.len() {
            0 => Ok(None),
            1 => Ok(Some(declarations[0])),
            _ => Err(SymbolTableError::duplicated_declaration_for_unique(
                kind,
                declarations.iter().map(|d| (*d).clone()).collect(),
            )),
        }
    }

    /// Attempts to resolve a unique declaration of a specific symbol kind assumed to appear only once in the AST.
    ///
    /// This function wraps [`resolve_unique_declaration`] and enforces uniqueness by returning an error
    /// if no declaration or more than one declaration is found for the given kind. It is useful in contexts
    /// where a declaration must exist and must be unique.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of symbol to resolve, typically one that is expected to be unique.
    ///
    /// # Returns
    ///
    /// * `Ok(&Declaration)` if exactly one declaration of the specified kind is found.
    /// * `Err(SymbolTableError)` if no such declaration exists or if multiple conflicting declarations are found.
    ///
    /// # Errors
    ///
    /// * Returns [`SymbolTableError::DeclarationNotFoundForKind`] if no declaration of the given kind exists.
    /// * Returns [`SymbolTableError::DuplicateDeclarationForUnique`] if multiple declarations of the same kind are found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let unique_decl = symbol_table.try_resolve_unique_declaration(SymbolKind::DomainName)?;
    /// println!("Domain declared at: {:?}", unique_decl.span());
    /// ```
    pub fn try_resolve_unique_declaration(
        &self,
        kind: SymbolKind,
    ) -> Result<&Declaration, SymbolTableError> {
        match self.resolve_unique_declaration(kind)? {
            Some(declaration) => Ok(declaration),
            None => Err(SymbolTableError::declaration_not_found_for_kind(kind)),
        }
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
            let new_key = map.get(&key)
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
    type Error = SymbolTableError;

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
