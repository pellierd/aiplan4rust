use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Filterable, Scope, SymbolEntry, SymbolKind, Usage,
};
use crate::aiplan4rust::semantic::symbol_table::table::Table;
use crate::aiplan4rust::tree::NodeId;
use indexmap::IndexMap;

impl Table {
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
            if let Some(symbol) = self.symbols.get(name.as_usize()) {
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
            .iter()
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
            if let Some(symbol) = self.symbols.get(name.as_usize()) {
                let usages = symbol.usages();
                let filtered_usages = Table::collect(kind, scope, usages);
                if !filtered_usages.is_empty() {
                    return vec![symbol];
                }
            }
            return Vec::new();
        }

        self.symbols
            .iter()
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
            if let Some(symbol) = self.symbols.get(name.as_usize()) {
                return Table::collect(kind, scope, symbol.declarations());
            } else {
                return Vec::new();
            }
        }

        self.symbols
            .iter()
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
            if let Some(symbol) = self.symbols.get(name.as_usize()) {
                return Table::collect(kind, scope, symbol.usages());
            } else {
                return Vec::new();
            }
        }

        self.symbols
            .iter()
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
        // Remplacement du HashSet par IndexMap (NodeId est la clé standard ici)
        items: &'a IndexMap<NodeId, T>,
    ) -> Vec<&'a T> {
        items
            .values() // On itère uniquement sur les valeurs (&T)
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
}
