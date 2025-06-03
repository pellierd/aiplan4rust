use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::SymbolOrigin;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::FilterableSymbol;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::Symbol;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::symbol::Usage;

use linked_hash_map::LinkedHashMap;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use indexmap::IndexSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A table of symbols used by the aiplan4rust.
///
/// This structure maintains an ordered mapping from unique string keys to `Symbol` instances.
/// It supports efficient insertion, lookup, and iteration of symbols, preserving insertion order
/// via the `LinkedHashMap`.
///
/// The `source` field tracks the origin of the symbols, useful for context or provenance information.
///
/// # Fields
/// - `symbols`: A `LinkedHashMap` storing symbols by their unique names.
/// - `source`: The origin or context from which the symbols were loaded or derived.
///
/// # Usage
/// This table is central to parsing and semantic analysis, holding all symbol declarations,
/// usages, and associated metadata.
///
/// # Example
/// ```rust
/// let mut table = SymbolTable::default();
/// // Insert or query symbols as needed
/// ```
pub struct SymbolTable {
    symbols: LinkedHashMap<String, Symbol>,
    source: SymbolOrigin,
}

/// Provides a default constructor for `SymbolTable`.
///
/// This implementation initializes a new `SymbolTable` with:
/// - An empty `symbols` map (using `LinkedHashMap`).
/// - A default `source` value (`SymbolOrigin::default()`).
///
/// # Returns
/// A new `SymbolTable` instance with default empty contents.
///
/// # Example
/// ```rust
/// let symbol_table = SymbolTable::default();
/// ```
impl Default for SymbolTable {
    fn default() -> Self {
        SymbolTable {
            symbols: LinkedHashMap::new(),
            source: SymbolOrigin::default(),
        }
    }
}

impl SymbolTable {
    /// Creates a new, empty `SymbolTable`.
    ///
    /// # Returns
    ///
    /// A new instance of `SymbolTable` with no symbols.
    pub fn new(source: SymbolOrigin) -> Self {
        SymbolTable {
            symbols: LinkedHashMap::new(),
            source,
        }
    }

    /// Returns a reference to the origin/source information of the symbol table.
    ///
    /// The `source` typically represents metadata about where the symbol table
    /// or its symbols originate from, such as a file, module, or other context.
    ///
    /// # Returns
    /// A reference to the `SymbolOrigin` associated with this symbol table.
    pub fn source(&self) -> &SymbolOrigin {
        &self.source
    }

    /// Sets or updates the origin/source information of the symbol table.
    ///
    /// This can be used to change metadata about where the symbol table or its
    /// contents are considered to come from, which might affect error reporting,
    /// analysis, or other tooling.
    ///
    /// # Parameters
    /// - `source`: The new `SymbolOrigin` value to assign to this symbol table.
    pub fn set_source(&mut self, source: SymbolOrigin) {
        self.source = source;
    }

    /// Returns an iterator over the symbol table's entries as immutable references.
    ///
    /// Each item yielded by the iterator is a tuple containing:
    /// - A reference to the symbol's name (`&String`)
    /// - A reference to the corresponding `Symbol` object (`&Symbol`)
    ///
    /// # Returns
    /// An iterator over all symbol name and symbol pairs in the symbol table.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
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
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Symbol)> {
        self.symbols.iter_mut()
    }

    /// Inserts a symbol into the symbol table using a unique key.
    ///
    /// # Arguments
    /// * `key` - A unique key for the symbol.
    /// * `symbol` - The symbol to insert.
    pub fn insert_symbol(&mut self, key: String, symbol: Symbol) {
        self.symbols.insert(key, symbol);
    }

    /// Retrieves an immutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a reference to the symbol if it exists.
    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Retrieves a mutable reference to a symbol by its key.
    ///
    /// # Arguments
    /// * `name` - The key for the symbol.
    ///
    /// # Returns
    /// An `Option` with a mutable reference if the symbol exists.
    pub fn get_symbol_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
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
    pub fn fetch_symbol_with_declarations(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        // If a specific symbol name is provided, try to get the symbol directly from the hashmap
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                // Retrieve all declarations of the found symbol
                let declarations = symbol.declarations();
                // Filter declarations by optional kind and scope
                let filtered_declarations = SymbolTable::fetch(kind, scope, declarations);
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
                let filtered_declarations = SymbolTable::fetch(kind, scope, declarations);
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
    pub fn fetch_symbol_with_usage(
        &self,
        symbol_name: Option<&str>,
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
                let filtered_usages = SymbolTable::fetch(kind, scope, usages);
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
                !SymbolTable::fetch(kind, scope, usages).is_empty()
            })
            // Collect the filtered symbols into a vector to return
            .collect()
    }

    /// Filters and returns declarations from the symbol table based on optional criteria.
    ///
    /// This method allows you to retrieve a list of declarations matching a given
    /// symbol name, kind, and/or scope. If a symbol name is provided, the search
    /// is limited to that symbol's declarations. Otherwise, all symbols in the
    /// symbol table are searched.
    ///
    /// # Arguments
    ///
    /// * `symbol_name` - An optional string representing the name of the symbol to filter.
    /// * `kind` - An optional reference to the kind of declaration to filter (`SymbolKind`).
    /// * `scope` - An optional reference to the scope in which the declaration must be defined
    ///   (`Scope`).
    ///
    /// # Returns
    ///
    /// A vector of references to `Declaration` objects matching the given filters.
    ///
    /// # Example
    ///
    /// ```rust
    /// let matches = symbol_table.filter_declaration(
    ///     Some("my_symbol"),
    ///     Some(&SymbolKind::Predicate),
    ///     None
    /// );
    /// ```
    pub fn fetch_declarations(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        if let Some(name) = symbol_name {
            // If the symbol name is provided, try to get the corresponding symbol.
            if let Some(symbol) = self.symbols.get(name) {
                // Filter that symbol's declarations using the provided kind and scope.
                return SymbolTable::fetch(kind, scope, symbol.declarations());
            } else {
                // No such symbol found: return an empty list.
                return Vec::new();
            }
        }

        // If no specific symbol name is provided, iterate over all symbols
        // and collect declarations matching the kind and scope.
        self.symbols
            .values()
            .flat_map(|symbol| SymbolTable::fetch(kind, scope, symbol.declarations()))
            .collect()
    }


    /// Filters the usages of symbols based on optional criteria.
    ///
    /// If a `symbol_name` is provided, this function attempts to retrieve the corresponding symbol
    /// and filters its usages according to the specified `kind` and `scope`.
    /// If no `symbol_name` is provided, it filters usages across all symbols in the symbol table.
    ///
    /// # Parameters
    /// - `symbol_name`: Optional name of the symbol whose usages are to be filtered.
    /// - `kind`: Optional kind of symbol usage to filter (e.g., predicate, function).
    /// - `scope`: Optional scope prefix; only usages whose scope starts with this prefix are
    ///   included.
    ///
    /// # Returns
    /// A vector of references to usages (`&Usage`) that match the given filters.
    ///
    /// # Examples
    /// ```
    /// // Filter usages of a specific symbol by kind and scope
    /// let filtered_usages = symbol_table.filter_usages(
    ///     Some("my_symbol"), Some(&SymbolKind::Predicate),
    ///     Some(&Scope::new("global"))
    /// );
    ///
    /// // Filter all usages of kind Function regardless of symbol name
    /// let all_function_usages = symbol_table.filter_usages(None, Some(&SymbolKind::Function), None);
    /// ```
    pub fn fetch_usages(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        // If a symbol name is provided, try to find the symbol in the symbol table.
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                // Found the symbol: filter its usages based on the optional kind and scope criteria.
                return SymbolTable::fetch(kind, scope, symbol.usages());
            } else {
                // No symbol with this name found: return an empty vector.
                return Vec::new();
            }
        }

        // No symbol name provided:
        // Iterate over all symbols and collect usages filtered by kind and scope.
        self.symbols.values()
            .flat_map(|symbol| SymbolTable::fetch(kind, scope, symbol.usages()))
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
    fn fetch<'a, T: FilterableSymbol>(
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
        items: &'a IndexSet<T>,
    ) -> Vec<&'a T> {
        items.iter()
            .filter(|item| {
                if let Some(k) = kind {
                    if item.kind() != k {
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
    /// - `Ok(None)`: No matching usage or declaration found for the given index.
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
    pub fn fetch_declaration_by_usage(
        &self,
        index: usize,
    ) -> Result<Option<&Declaration>, ParserInternalError> {
        // Iterate over every symbol stored in the symbol table
        for symbol in self.symbols.values() {
            // Cache declarations of the current symbol for efficient reuse
            let declarations = symbol.declarations();

            // Iterate through all usages of this symbol
            for usage in symbol.usages() {
                // Check if the usage's AST index matches the requested index
                if usage.ast() == index {
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
                                index
                            )));
                        }
                    }
                }
            }
        }

        // No usage matching the given AST index was found, so return None
        Ok(None)
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
impl fmt::Display for SymbolTable {
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
