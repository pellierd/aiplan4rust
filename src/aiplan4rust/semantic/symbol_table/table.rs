
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Filterable;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::arena::{ArenaAst, NodeId};
use crate::aiplan4rust::semantic::symbol_table::{SymbolTableBuilder, SymbolTableOrigin};

use linked_hash_map::LinkedHashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::collections::{HashMap, HashSet};

/// A symbol table used in `aiplan4rust` to store and manage symbols.
///
/// This structure maintains an ordered mapping from identifiers to their corresponding `Symbol`
/// instances. It is central to parsing, semantic analysis, and later stages such as linking.
///
/// The insertion order of symbols is preserved via `LinkedHashMap`, which is useful for
/// deterministic behavior during iteration (e.g., for diagnostics or code generation).
///
/// # Fields
/// - `symbols`: Maps `Ident` to `Symbol`, preserving insertion order.
/// - `origin`: Indicates the source context of this table (e.g., Domain, Problem, Merged).
///
/// # Example
/// ```rust
/// let mut table = SymbolTable::default();
/// // Insert or lookup symbols
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    symbols: LinkedHashMap<Ident, Symbol>,
    origin: SymbolTableOrigin,
}

impl Default for Table {
    /// Creates a new `SymbolTable` with empty contents and default origin.
    ///
    /// # Returns
    /// A `SymbolTable` with an empty symbol map and origin set to `SymbolTableOrigin::None`.
    ///
    /// # Example
    /// ```rust
    /// let table = SymbolTable::default();
    /// assert!(table.is_empty());
    /// ```
    fn default() -> Self {
        Table {
            symbols: LinkedHashMap::new(),
            origin: SymbolTableOrigin::default(),
        }
    }
}


impl Table {
    /// Creates a new, empty `SymbolTable` with the given origin.
    ///
    /// # Parameters
    /// - `origin`: The origin context of this symbol table (e.g., Domain, Problem, Merged).
    ///
    /// # Returns
    /// A new `SymbolTable` instance with no symbols and the specified origin.
    pub fn new(origin: SymbolTableOrigin) -> Self {
        Table {
            symbols: LinkedHashMap::new(),
            origin,
        }
    }

    /// Returns the origin metadata of the symbol table.
    ///
    /// The origin describes where the symbol table was constructed from—
    /// for example, a domain file, a problem file, or the result of merging both.
    ///
    /// # Returns
    /// A reference to the `SymbolTableOrigin` associated with this symbol table.
    pub fn origin(&self) -> SymbolTableOrigin {
        self.origin
    }

    /// Sets or updates the origin metadata of the symbol table.
    ///
    /// This is useful during linking or transformations when the origin context
    /// of a table changes (e.g., from Domain to Merged).
    ///
    /// # Parameters
    /// - `origin`: The new `SymbolTableOrigin` to assign.
    pub fn set_origin(&mut self, origin: SymbolTableOrigin) {
        self.origin = origin;
    }


    /// Returns an iterator over the symbol table's entries as immutable references.
    ///
    /// Each item yielded by the iterator is a tuple containing:
    /// - A reference to the symbol's name (`&String`)
    /// - A reference to the corresponding `Symbol` object (`&Symbol`)
    ///
    /// # Returns
    /// An iterator over all symbol name and symbol pairs in the symbol table.
    pub fn iter(&self) -> impl Iterator<Item = (&Ident, &Symbol)> {
        self.symbols.iter()
    }

    /// Returns an iterator over the symbol table's entries as mutable references.
    ///
    /// Each item yielded by the iterator is a tuple containing:
    /// - A reference to the symbol's name (`&String`)
    /// - A mutable reference to the corresponding `Symbol` object (`&mut Symbol`)
    ///
    /// # Returns
    /// An iterator over all symbol name and mutable symbol pairs, allowing modification
    /// of the symbols during iteration.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Ident, &mut Symbol)> {
        self.symbols.iter_mut()
    }

    /// Returns an iterator over the symbol table's entries, consuming the table.
    ///
    /// Each item yielded by the iterator is a tuple containing:
    /// - The symbol's name (`Ident`)
    /// - The corresponding `Symbol` object (`Symbol`)
    ///
    /// # Returns
    /// An iterator over all symbol name and symbol pairs in the symbol table, consuming the table.
    pub fn into_iter(self) -> impl Iterator<Item = (Ident, Symbol)> {
        self.symbols.into_iter()
    }

    /// Inserts a symbol into the symbol table using a unique key.
    ///
    /// # Arguments
    /// * `key` - A unique key for the symbol.
    /// * `symbol` - The symbol to insert.
    pub fn insert_symbol(&mut self, key: Ident, symbol: Symbol) {
        self.symbols.insert(key, symbol);
    }

    /// Retrieves an immutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a reference to the symbol if it exists.
    pub fn get_symbol(&self, name: Ident) -> Option<&Symbol> {
        self.symbols.get(&name)
    }

    /// Retrieves a mutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a mutable reference if the symbol exists.
    pub fn get_symbol_mut(&mut self, name: Ident) -> Option<&mut Symbol> {
        self.symbols.get_mut(&name)
    }

    /// Returns an iterator over all symbols in the table.
    ///
    /// # Returns
    /// An iterator yielding references to all symbols.
    pub fn values(&self) -> impl Iterator<Item=&Symbol> {
        self.symbols.values()
    }

    /// Returns a mutable iterator over all symbols in the table.
    ///
    /// # Returns
    /// An iterator yielding mutable references to all symbols.
    pub fn values_mut(&mut self) -> impl Iterator<Item=&mut Symbol> {
        self.symbols.iter_mut().map(|(_, symbol)| symbol)
    }

    /// Retrieves symbols from the symbol table that have declarations matching optional filters.
    ///
    /// This function searches the symbol table for symbols optionally filtered by name, declaration
    /// kind, and declaration scope. If a specific symbol name is provided, the search is optimized
    /// by looking up that symbol directly in the internal map. Otherwise, it iterates over all
    /// symbols.
    ///
    /// For each symbol found, its declarations are filtered according to the provided `kind` and
    /// `scope` criteria. Only symbols with at least one declaration matching these filters are
    /// included in the result.
    ///
    /// # Parameters
    /// - `symbol_name`: Optional name of the symbol to look up. If `Some(name)`, the function
    ///   attempts to retrieve that exact symbol directly. If `None`, the search considers all
    ///   symbols.
    /// - `kind`: Optional kind of declarations to filter by (e.g., Predicate, Function). If `None`,
    ///   declarations of all kinds are considered.
    /// - `scope`: Optional scope prefix to filter declarations. Only declarations whose scope
    ///   starts with this prefix are considered. If `None`, all scopes are considered.
    ///
    /// # Returns
    /// A vector of references to symbols that have at least one declaration matching the provided
    /// filters. If no matching symbols are found, returns an empty vector.
    ///
    /// # Examples
    /// ```rust
    /// // Fetch all symbols named "mySymbol" with Predicate declarations in the "global" scope.
    /// let symbols = symbol_table.fetch_symbol_with_declarations(
    ///     Some("mySymbol"),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global"))
    /// );
    ///
    /// // Fetch all symbols with Function declarations, regardless of name or scope.
    /// let symbols = symbol_table.fetch_symbol_with_declarations(
    ///     None,
    ///     Some(&SymbolKind::Function),
    ///     None
    /// );
    /// ```
    pub fn collect_symbol_with_declaration(
        &self,
        symbol_name: Option<&Ident>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        // If a specific symbol name is provided, try to get the symbol directly from the hashmap
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                // Retrieve all declarations of the found symbol
                let declarations = symbol.declarations();
                // Filter declarations by optional kind and scope
                let filtered_declarations = Table::collect(kind, scope, declarations);
                // If any filtered declarations exist, return the symbol wrapped in a vector
                if !filtered_declarations.is_empty() {
                    return vec![symbol];
                } else {
                    // If no matching declarations, return an empty vector
                    return Vec::new();
                }
            } else {
                // If the symbol name is not found, return an empty vector
                return Vec::new();
            }
        }

        // If no symbol name is provided, iterate over all symbols in the symbol table
        self.symbols.values()
            // Keep only symbols that have declarations matching the filters
            .filter(|symbol| {
                // Retrieve all declarations of the current symbol
                let declarations = symbol.declarations();
                // Filter declarations by kind and scope
                let filtered_declarations = Table::collect(kind, scope, declarations);
                // Keep symbol only if filtered declarations are not empty
                !filtered_declarations.is_empty()
            })
            // Collect and return all matching symbols as a vector
            .collect()
    }


    /// Retrieves symbols from the symbol table that have usages matching optional filters.
    ///
    /// This function returns all symbols that have at least one usage filtered by the given
    /// `kind` and `scope`. If a specific `symbol_name` is provided, the function attempts to
    /// find that symbol directly and checks its usages, returning early if a match is found.
    /// Otherwise, it scans all symbols in the table to find those with matching usages.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional name of the symbol to filter by. If specified, the lookup
    ///   is optimized to directly find this symbol and check its usages.
    /// - `kind`: An optional `SymbolKind` to filter usages by. Only usages matching this kind
    ///   are considered.
    /// - `scope`: An optional `Scope` prefix to filter usages. Only usages whose scope starts
    ///   with this prefix are considered.
    ///
    /// # Returns
    /// A vector of references to symbols that have at least one usage matching the specified filters.
    ///
    /// # Behavior
    /// - If `symbol_name` is provided:
    ///     - Attempts to retrieve the symbol by name.
    ///     - Filters its usages by `kind` and `scope`.
    ///     - Returns a vector with the symbol if any matching usage exists.
    ///     - Returns an empty vector if no matching usages are found or symbol doesn't exist.
    /// - If `symbol_name` is `None`:
    ///     - Iterates all symbols in the table.
    ///     - Returns those symbols with at least one usage matching the filters.
    ///
    /// # Examples
    /// ```rust
    /// // Fetch all symbols named "foo" that have usages of kind Predicate in the global scope
    /// let symbols = symbol_table.fetch_symbol_with_usage(
    ///     Some("foo"),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global"))
    /// );
    ///
    /// // Fetch all symbols with usages of kind Action regardless of scope or name
    /// let symbols = symbol_table.fetch_symbol_with_usage(None, Some(&SymbolKind::Action), None);
    /// ```
    pub fn collect_symbol_with_usages(
        &self,
        symbol_name: Option<&Ident>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        // If a specific symbol name is provided, try to get that symbol directly from the symbol table
        if let Some(name) = symbol_name {
            // Attempt to find the symbol by name in the symbol map
            if let Some(symbol) = self.symbols.get(name) {
                // Retrieve all usages associated with the symbol
                let usages = symbol.usages();
                // Filter the usages based on the given kind and scope
                let filtered_usages = Table::collect(kind, scope, usages);
                // If there is at least one matching usage, return the symbol in a vector
                if !filtered_usages.is_empty() {
                    return vec![symbol];
                }
            }
            // If symbol is not found or no usages match, return an empty vector immediately
            return Vec::new();
        }

        // If no symbol name is specified, iterate over all symbols in the table
        self.symbols.values()
            // Keep only those symbols that have at least one usage matching the filters
            .filter(|symbol| {
                let usages = symbol.usages();
                !Table::collect(kind, scope, usages).is_empty()
            })
            // Collect the filtered symbols into a vector to return
            .collect()
    }

    /// Filters and collects declarations from the symbol table based on optional criteria.
    ///
    /// This method retrieves a list of declarations that match the provided filters:
    /// - `symbol_name`: if specified, restricts the search to declarations of that symbol only.
    /// - `kind`: if specified, filters declarations by their kind (e.g., Predicate, Task).
    /// - `scope`: if specified, filters declarations by the scope in which they are defined.
    ///
    /// If no `symbol_name` is given, the search is performed over all symbols in the table.
    ///
    /// # Parameters
    /// - `symbol_name`: Optional symbol name to restrict the search to declarations of that symbol.
    /// - `kind`: Optional kind of declarations to filter by (`SymbolKind`).
    /// - `scope`: Optional scope to filter declarations by (`Scope`).
    ///
    /// # Returns
    /// A vector of references to `Declaration` instances matching all the specified criteria.
    ///
    /// # Examples
    /// ```rust
    /// // Collect all Predicate declarations for the symbol "my_symbol"
    /// let predicates = symbol_table.collect_declarations(
    ///     Some("my_symbol"),
    ///     Some(&SymbolKind::Predicate),
    ///     None,
    /// );
    ///
    /// // Collect all declarations of kind Task across all symbols within a specific scope
    /// let scoped_tasks = symbol_table.collect_declarations(
    ///     None,
    ///     Some(&SymbolKind::Task),
    ///     Some(&Scope::new("global")),
    /// );
    /// ```
    pub fn collect_declarations(
        &self,
        symbol_name: Option<&Ident>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        if let Some(name) = symbol_name {
            // If the symbol name is provided, try to get the corresponding symbol.
            if let Some(symbol) = self.symbols.get(name) {
                // Filter that symbol's declarations using the provided kind and scope.
                return Table::collect(kind, scope, symbol.declarations());
            } else {
                // No such symbol found: return an empty list.
                return Vec::new();
            }
        }

        // If no specific symbol name is provided, iterate over all symbols
        // and collect declarations matching the kind and scope.
        self.symbols
            .values()
            .flat_map(|symbol| Table::collect(kind, scope, symbol.declarations()))
            .collect()
    }

    /// Filters and collects usages of symbols based on optional criteria.
    ///
    /// This function retrieves usages of symbols stored in the symbol table, optionally filtered by:
    /// - the symbol's name,
    /// - the kind of usage (e.g., Predicate, Function, Task),
    /// - and the scope within which the usage occurs.
    ///
    /// If a specific `symbol_name` is provided, only usages of that symbol are considered.
    /// Otherwise, usages of all symbols in the table are filtered.
    ///
    /// # Parameters
    /// - `symbol_name`: An optional symbol name to restrict the search to usages of that symbol.
    /// - `kind`: An optional `SymbolKind` to filter usages by their kind.
    /// - `scope`: An optional `Scope` that serves as a prefix filter on the usage scopes.
    ///
    /// # Returns
    /// A vector containing references to all `Usage` instances that match the provided filters.
    ///
    /// # Examples
    /// ```
    /// // Filter usages of a specific symbol "my_symbol" that are of kind Predicate and within "global" scope.
    /// let filtered_usages = symbol_table.collect_usages(
    ///     Some("my_symbol"),
    ///     Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global")),
    /// );
    ///
    /// // Filter all usages of kind Function across all symbols, ignoring symbol name and scope.
    /// let all_function_usages = symbol_table.collect_usages(None, Some(&SymbolKind::Function), None);
    /// ```
    pub fn collect_usages(
        &self,
        symbol_name: Option<&Ident>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        // If a symbol name is provided, try to find the symbol in the symbol table.
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                // Found the symbol: filter its usages based on the optional kind and scope criteria.
                return Table::collect(kind, scope, symbol.usages());
            } else {
                // No symbol with this name found: return an empty vector.
                return Vec::new();
            }
        }

        // No symbol name provided:
        // Iterate over all symbols and collect usages filtered by kind and scope.
        self.symbols.values()
            .flat_map(|symbol| Table::collect(kind, scope, symbol.usages()))
            .collect()
    }

    /// Filters a collection of symbols or symbol-like items by optional kind and scope criteria.
    ///
    /// This function iterates over the provided set of items implementing the `FilterableSymbol`
    /// trait, returning only those that match the specified optional `kind` and `scope` filters. If
    /// a filter is `None`, it is ignored.
    ///
    /// # Parameters
    /// - `kind`: An optional reference to a `SymbolKind` to filter by. If provided, only items with
    ///   this kind are retained.
    /// - `scope`: An optional reference to a `Scope` to filter by. If provided, only items whose
    ///   scope starts with this scope prefix are retained.
    /// - `items`: A reference to a collection of items implementing `FilterableSymbol` to filter.
    ///
    /// # Returns
    /// A vector of references to the filtered items.
    ///
    /// # Example
    /// ```rust
    /// let filtered = fetch(
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

    /// Retrieves the declaration associated with a usage identified by a specific AST node index.
    ///
    /// This function searches all symbols and their usages in the symbol table to find a usage
    /// that matches the given AST node index. It then filters the declarations of the matching
    /// symbol to find those whose scope contains the usage's scope.
    ///
    /// # Parameters
    /// - `index`: The AST node index representing the usage to look up.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: Exactly one matching declaration found.
    /// - `Ok(None)`: No matching usage or declaration found for the given xx.
    /// - `Err(ParserInternalError)`: Multiple declarations match the usage, indicating ambiguity.
    ///
    /// # Errors
    /// Returns an error if multiple declarations correspond to the same usage index, signaling
    /// an invalid state.
    ///
    /// # Examples
    /// ```rust
    /// let declaration = symbol_table.fetch_declaration_by_usage(42)?;
    /// match declaration {
    ///     Some(decl) => println!("Declaration found: {:?}", decl),
    ///     None => println!("No declaration found for usage 42"),
    /// }
    /// ```
    pub fn resolve_declaration_by_usage(
        &self,
        node_id: NodeId,
    ) -> Result<Option<&Declaration>, ParserInternalError> {
        // Iterate over every symbol stored in the symbol table
        for symbol in self.symbols.values() {
            // Cache declarations of the current symbol for efficient reuse
            let declarations = symbol.declarations();

            // Iterate through all usages of this symbol
            for usage in symbol.usages() {
                // Check if the usage's AST index matches the requested index
                if usage.node_id() == node_id {
                    // Filter declarations to those whose scope is compatible with the usage's scope
                    let filtered: Vec<&Declaration> = declarations
                        .iter()
                        .filter(|decl| usage.scope().starts_with(decl.scope()))
                        .collect();

                    // Handle the filtered results based on how many matches were found
                    match filtered.len() {
                        0 => return Ok(None),            // No declaration matches this usage
                        1 => return Ok(Some(filtered[0])), // Exactly one declaration found, return it
                        _ => {
                            // Multiple matching declarations found, which is an error case
                            return Err(ParserInternalError::new(format!(
                                "Multiple declarations found for usage at AST index {}.",
                                node_id
                            )));
                        }
                    }
                }
            }
        }

        // No usage matching the given AST index was found, so return None
        Ok(None)
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
    ///   guides
    ///   the filtering and validation logic to pick an appropriate declaration.
    /// - `scope`: The lexical or semantic scope in which the symbol usage occurs. Only declarations
    ///   within or compatible with this scope are considered.
    ///
    /// # Returns
    /// Returns a `Result` wrapping an `Option`:
    /// - `Ok(Some(&Declaration))`: If exactly one valid declaration matching the criteria is found.
    /// - `Ok(None)`: If no matching declaration exists for the given filters.
    /// - `Err(ParserInternalError)`: If multiple conflicting or ambiguous declarations are found,
    ///   making it impossible to resolve a unique declaration.
    ///
    /// # Errors
    /// This function returns a [`ParserInternalError`] if multiple declarations match the criteria
    /// and no clear resolution can be made. This error helps identify semantic issues such as
    /// duplicated or conflicting declarations.
    ///
    /// # Behavior Details
    /// - The resolution process uses [`fetch_declarations`] internally to collect candidate
    ///   declarations.
    /// - Validation and disambiguation of these candidates are done by [`select_valid_declaration`].
    /// - The special fallback from `Task` to `Action` allows flexible handling of task-like symbols.
    ///
    /// # Examples
    /// ```rust
    /// let symbol_name = "move";
    /// let usage_kind = SymbolKind::Task;
    /// let scope = Scope::new("global");
    ///
    /// match symbol_table.resolve_declaration(symbol_name, &usage_kind, &scope) {
    ///     Ok(Some(decl)) => println!("Resolved declaration: {:?}", decl),
    ///     Ok(None) => println!("No matching declaration found."),
    ///     Err(err) => eprintln!("Error resolving declaration: {}", err),
    /// }
    /// ```
    ///
    /// [`fetch_declarations`]: Table::collect_declarations
    /// [`select_valid_declaration`]: Table::select_valid_declaration
    /// [`ParserInternalError`]: crate::errors::ParserInternalError
    pub fn resolve_declaration(
        &self,
        symbol_name: &Ident,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<Option<&Declaration>, ParserInternalError> {
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
        symbol_name: &Ident,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        // Match on the usage kind to determine the appropriate validation strategy
        match usage_kind {
            // For PrimitiveType or Predicate kinds, use specific validation logic
            SymbolKind::PrimitiveType | SymbolKind::Predicate => {
                Self::validate_type_or_predicate_declarations(symbol_name, usage_kind, declarations)
            }
            // For Task kind, use task-specific validation logic
            SymbolKind::Task => {
                Self::validate_task_declarations(symbol_name, declarations)
            }
            // For all other kinds, handle based on the number of declarations found
            _ => match declarations.len() {
                0 => Ok(None), // No declarations found
                1 => Ok(Some(declarations[0])), // Exactly one declaration found, return it
                // Multiple declarations found, return an error indicating ambiguity
                _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, declarations.len())),
            },
        }
    }

    /// Determines whether a declaration kind is compatible with a usage kind.
    ///
    /// Used to allow limited polymorphism (e.g., using a `Predicate` in a type context).
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
    /// - `Ok(Some(&Declaration))`: If validation passes.
    /// - `Ok(None)`: If no matching declaration exists.
    /// - `Err`: If validation fails due to ambiguity or incompatible kinds.
    fn validate_type_or_predicate_declarations<'a>(
        symbol_name: &Ident,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
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
                    _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, declarations.len())),
                },
                // More than two declarations in total means ambiguity — return an error
                _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, declarations.len())),
            },
            // More than one matching declaration is ambiguous — return an error
            _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, matching.len())),
        }
    }

    /// Validates task declarations by allowing either a `Task` or an `Action`, but only one.
    ///
    /// If the declaration kind is incompatible, it is ignored.
    ///
    /// # Returns
    /// - `Ok(Some(&Declaration))`: If one valid declaration is found.
    /// - `Ok(None)`: If none found or incompatible.
    /// - `Err`: If multiple declarations cause ambiguity.
    fn validate_task_declarations<'a>(
        symbol_name: &Ident,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        // If there is more than one declaration, return an error indicating ambiguity
        if declarations.len() > 1 {
            return Err(Self::multiple_declarations_error(
                symbol_name,
                &SymbolKind::Task,
                declarations.len())
            );
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

    /// Resolves the unique `DomainName` symbol in the symbol table.
    ///
    /// This function searches for symbols of kind `DomainName` in the current symbol table.
    /// If exactly one such symbol is found, it returns a reference to it.
    /// If no `DomainName` symbol is found, it returns `Ok(None)`.
    /// If more than one symbol of this kind exists, it returns an error indicating a malformed AST.
    ///
    /// # Returns
    /// - `Ok(Some(&Symbol))` if exactly one `DomainName` symbol is found.
    /// - `Ok(None)` if no such symbol exists.
    /// - `Err(ParserInternalError)` if multiple `DomainName` symbols are found.
    ///
    /// # Errors
    /// Returns an error when multiple `DomainName` symbols are found, indicating
    /// that the annotated syntax tree (AST) is structurally invalid.
    pub fn resolve_domain_name_declaration(&self) -> Result<Option<&Symbol>, ParserInternalError> {
        self.resolve_unique_declaration(SymbolKind::DomainName)
    }

    /// Resolves the unique `ProblemName` symbol in the symbol table.
    ///
    /// This function searches for symbols of kind `ProblemName` in the current symbol table.
    /// If exactly one such symbol is found, it returns a reference to it.
    /// If no `ProblemName` symbol is found, it returns `Ok(None)`.
    /// If more than one symbol of this kind exists, it returns an error indicating a malformed AST.
    ///
    /// # Returns
    /// - `Ok(Some(&Symbol))` if exactly one `ProblemName` symbol is found.
    /// - `Ok(None)` if no such symbol exists.
    /// - `Err(ParserInternalError)` if multiple `ProblemName` symbols are found.
    ///
    /// # Errors
    /// Returns an error when multiple `ProblemName` symbols are found, indicating
    /// that the annotated syntax tree (AST) is structurally invalid.
    pub fn resolve_problem_name_declaration(&self) -> Result<Option<&Symbol>, ParserInternalError> {
        self.resolve_unique_declaration(SymbolKind::ProblemName)
    }

    /// Resolves a unique symbol of a specific kind assumed to be singular in the AST.
    ///
    /// This internal utility function is designed to resolve symbols that are expected
    /// to appear only once per annotated syntax tree, such as `DomainName` or `ProblemName`.
    /// It collects all symbols of the specified kind and returns:
    /// - The symbol if exactly one is found.
    /// - `None` if no symbol is found.
    /// - An error if more than one symbol is found, which indicates an invalid AST.
    ///
    /// # Arguments
    /// - `kind`: The kind of symbol to resolve, typically one that is expected to be unique.
    ///
    /// # Returns
    /// - `Ok(Some(&Symbol))` if a single symbol of the specified kind is found.
    /// - `Ok(None)` if no symbol of that kind exists.
    /// - `Err(ParserInternalError)` if multiple symbols of the same kind are found.
    ///
    /// # Errors
    /// Returns an error if multiple declarations of the same `SymbolKind` are found,
    /// indicating a semantic or structural error in the annotated syntax tree.
    fn resolve_unique_declaration(
        &self,
        kind: SymbolKind,
    ) -> Result<Option<&Symbol>, ParserInternalError> {
        let symbols = self.collect_symbol_with_declaration(None, Some(&kind), None);

        match symbols.len() {
            0 => Ok(None),
            1 => Ok(Some(symbols[0])),
            _ => Err(ParserInternalError::new(format!(
                "Malformed Annotated Syntax Tree: multiple declarations found for symbol kind {:?}: {:?}",
                kind, symbols,
            ))),
        }
    }


    /// Constructs a `ParserInternalError` indicating that multiple declarations exist
    /// for a symbol where only one was expected.
    ///
    /// This is typically used in resolution contexts where ambiguity from multiple
    /// declarations of the same symbol name and kind is not permitted.
    ///
    /// # Parameters
    /// - `symbol_name`: The name of the symbol that caused the ambiguity.
    /// - `usage_kind`: The kind the symbol was expected to match (e.g., `Predicate`, `Task`).
    /// - `count`: The number of declarations found, which exceeded the allowed amount.
    ///
    /// # Returns
    /// A `ParserInternalError` describing the ambiguity in symbol declarations.
    fn multiple_declarations_error(
        symbol_name: &Ident,
        usage_kind: &SymbolKind,
        count: usize,
    ) -> ParserInternalError {
        ParserInternalError::new(format!(
            "Symbol '{}' with kind '{:?}' has {} declarations, which is invalid.",
            symbol_name, usage_kind, count
        ))
    }

    pub fn remap_identsv3(&mut self, map: &HashMap<Ident, Ident>) {
        let mut new_symbols = LinkedHashMap::new();

        // On consomme self.symbols avec `drain()` pour éviter les clones
        for (key, mut symbol) in self.symbols.drain() {
            // Remapper les Symbol eux-mêmes
            symbol.remap_idents(map);

            // Trouver la nouvelle clé
            let new_key = map.get(&key).cloned().unwrap_or_else(|| key.clone());

            // Vérification de conflits
            if new_symbols.contains_key(&new_key) {
                panic!(
                    "Conflit de remapping : deux symboles sont remappés vers {:?}",
                    new_key
                );
            }

            new_symbols.insert(new_key, symbol);
        }

        // Remplacer la map d'origine
        self.symbols = new_symbols;
    }
    pub fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        // Étape 1 : remap interne des Symbol
        for (_key, symbol) in self.symbols.iter_mut() {
            symbol.remap_idents(map);
        }

        // Étape 2 : reconstruire la map avec les clés remappées
        let mut new_symbols = LinkedHashMap::with_capacity(self.symbols.len());

        // On prend la map complète pour pouvoir la vider et déplacer les valeurs
        for (key, symbol) in std::mem::take(&mut self.symbols) {
            // Cherche la nouvelle clé (clone seulement si mappé)
            let new_key = map.get(&key).cloned().unwrap_or(key);

            if new_symbols.contains_key(&new_key) {
                panic!("Conflit : plusieurs symboles remappés vers {:?}", new_key);
            }

            new_symbols.insert(new_key, symbol);
        }

        self.symbols = new_symbols;
    }




    /// Merges two symbol tables (`domain` and `problem`) into a single `SymbolTable`.
    ///
    /// The resulting symbol table has the origin set to `SymbolTableOrigin::Merged`.
    ///
    /// Symbols from the `domain` table are inserted first. Then symbols from the `problem`
    /// table are merged:
    /// - If a symbol from the `problem` table has the same identifier as one in the merged table,
    ///   an attempt is made to merge their contents via `Symbol::merge_with`.
    /// - If merging fails (e.g., symbol names differ), the function returns a `ParserInternalError`.
    /// - Otherwise, the symbol is inserted directly if it does not exist yet.
    ///
    /// # Parameters
    ///
    /// - `domain`: The symbol table representing domain-level symbols.
    /// - `problem`: The symbol table representing problem-level symbols.
    ///
    /// # Returns
    ///
    /// - `Ok(SymbolTable)`: The merged symbol table with combined entries.
    /// - `Err(ParserInternalError)`: If a symbol merge fails due to incompatible symbols.
    ///
    /// # Errors
    ///
    /// This function returns an error if two symbols with the same identifier cannot be merged,
    /// typically because their internal names differ.
    ///
    /// # Example
    ///
    /// ```rust
    /// let merged_table = merge(domain_table, problem_table)?;
    /// ```
    ///
    pub fn merge(domain: Table, problem: Table) -> Result<Table, ParserInternalError> {
        let mut merged = Table::new(SymbolTableOrigin::Merged);

        // Insert symbols from the domain table
        for (ident, symbol) in domain.into_iter() {
            merged.insert_symbol(ident, symbol);
        }

        // Merge or insert symbols from the problem table
        for (ident, symbol) in problem.into_iter() {
            if let Some(existing_symbol) = merged.get_symbol_mut(ident) {
                if !existing_symbol.merge_with(symbol) {
                    return Err(ParserInternalError::new(format!(
                        "Failed to merge symbol with different name: {}", ident
                    )));
                }
            } else {
                merged.insert_symbol(ident, symbol);
            }
        }

        Ok(merged)
    }

    /// Creates a new `SymbolTable` by building it from the given AST.
    ///
    /// This function serves as a convenient entry point to construct
    /// a symbol table based on the provided abstract syntax tree (`ArenaAst`).
    /// Internally, it uses `SymbolTableBuilder` to perform the construction.
    ///
    /// # Parameters
    /// - `ast`: A reference to the AST (`ArenaAst`) from which to build the symbol table.
    ///
    /// # Returns
    /// - `Ok(SymbolTable)` if the symbol table was successfully built.
    /// - `Err(ParserInternalError)` if any error occurred during the building process.
    ///
    /// # Example
    /// ```
    /// let ast = ...; // Assume you have an ArenaAst instance
    /// let symbol_table = SymbolTable::from_ast(&ast)?;
    /// ```
    ///
    /// # Errors
    /// Returns `ParserInternalError` if semantic errors or other parsing issues are detected during building.
    pub fn from_ast(ast: &ArenaAst) -> Result<Table, ParserInternalError> {
        let mut builder = SymbolTableBuilder::new();
        let symbol_table = builder.build(ast)?;
        Ok(symbol_table)
    }


    pub fn to_string_with_interner(&self, interner: &StringInterner) -> String {
        let mut out = String::new();
        let _ = self.fmt_with_interner(&mut out, interner);
        out
    }

    pub fn fmt_with_interner(
        &self,
        w: &mut dyn fmt::Write,
        interner: &StringInterner,
    ) -> fmt::Result {
        // Parcourt tous les symboles dans la table
        for symbol in self.symbols.values() {
            // Utilise la méthode fmt_with_interner de chaque symbole, en passant l'interner
            symbol.fmt_with_interner(w, interner)?;
            writeln!(w)?; // Ajoute un saut de ligne après chaque symbole
        }
        Ok(())
    }

}

/// Implements the `Display` trait for `SymbolTable`.
///
/// This allows a `SymbolTable` instance to be formatted as a user-friendly string,
/// typically for debugging or printing purposes. Each symbol in the table is
/// displayed on its own line.
///
/// # Example
/// ```
/// let symbol_table = SymbolTable::new();
/// println!("{}", symbol_table); // Prints all symbols line by line.
/// ```
impl fmt::Display for Table {
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
