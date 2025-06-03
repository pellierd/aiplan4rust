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
/// This structure maintains an ordered mapping from unique keys to symbols.
/// It provides methods to insert, retrieve, and serialize symbols during parsing.
pub struct SymbolTable {
    symbols: LinkedHashMap<String, Symbol>,
    source: SymbolOrigin,
}

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

    pub fn source(&self) -> &SymbolOrigin {
        &self.source
    }

    pub fn set_source(&mut self, source: SymbolOrigin) {
        self.source = source;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.symbols.iter()
    }

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

    pub fn get_symbols_with_declaration_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        let mut result = Vec::new();

        // Parcourir tous les symboles
        for symbol in self.symbols.values() {
            // Vérifier si le nom correspond, si spécifié
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }

            // Récupérer les déclarations du symbole
            let declarations = symbol.declarations();

            // Vérifier si le filtre sur les déclarations correspond à quelque chose
            let filtered_declarations = SymbolTable::filter_symbol(kind, scope, declarations);

            // Si des déclarations correspondantes sont trouvées, ajouter le symbole à la réponse
            if !filtered_declarations.is_empty() {
                result.push(symbol);
            }
        }

        result
    }

    pub fn get_symbols_with_usage_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Symbol> {
        let mut result = Vec::new();

        // Parcourir tous les symboles
        for symbol in self.symbols.values() {
            // Vérifier si le nom correspond, si spécifié
            if let Some(name) = symbol_name {
                if symbol.name() != name {
                    continue;
                }
            }

            // Récupérer les déclarations du symbole
            let declarations = symbol.usages();

            // Vérifier si le filtre sur les déclarations correspond à quelque chose
            let filtered_usages = SymbolTable::filter_symbol(kind, scope, declarations);

            // Si des déclarations correspondantes sont trouvées, ajouter le symbole à la réponse
            if !filtered_usages.is_empty() {
                result.push(symbol);
            }
        }

        result
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
    /// * `scope` - An optional reference to the scope in which the declaration must be defined (`Scope`).
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
    pub fn filter_declarations(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        if let Some(name) = symbol_name {
            // If the symbol name is provided, try to get the corresponding symbol.
            if let Some(symbol) = self.symbols.get(name) {
                // Filter that symbol's declarations using the provided kind and scope.
                return SymbolTable::filter_symbol(kind, scope, symbol.declarations());
            } else {
                // No such symbol found: return an empty list.
                return Vec::new();
            }
        }

        // If no specific symbol name is provided, iterate over all symbols
        // and collect declarations matching the kind and scope.
        self.symbols
            .values()
            .flat_map(|symbol| SymbolTable::filter_symbol(kind, scope, symbol.declarations()))
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
    pub fn filter_usages(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        // If a symbol name is provided, try to find the symbol in the symbol table.
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                // Found the symbol: filter its usages based on the optional kind and scope criteria.
                return SymbolTable::filter_symbol(kind, scope, symbol.usages());
            } else {
                // No symbol with this name found: return an empty vector.
                return Vec::new();
            }
        }

        // No symbol name provided:
        // Iterate over all symbols and collect usages filtered by kind and scope.
        self.symbols.values()
            .flat_map(|symbol| SymbolTable::filter_symbol(kind, scope, symbol.usages()))
            .collect()
    }


    /// Filters a set of symbols or symbol-related items by optional kind and scope criteria.
    ///
    /// This function iterates over the provided set of items implementing `FilterableSymbol`
    /// and returns those matching the given optional `SymbolKind` and `Scope`.
    ///
    /// # Parameters
    /// - `kind`: Optional symbol kind to match.
    /// - `scope`: Optional scope prefix to match. Only items whose scope starts with this prefix
    ///   will be retained.
    /// - `items`: A set of symbol-like items to filter.
    ///
    /// # Returns
    /// A `Vec` of references to items that match the provided filters.
    ///
    /// # Example
    /// ```rust
    /// let filtered = filter_symbol(Some(&SymbolKind::Predicate), Some(&Scope::new("global")), &my_items);
    /// ```
    fn filter_symbol<'a, T: FilterableSymbol>(
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


    pub fn get_declaration_by_index(&self, index: usize) -> Option<&Declaration> {
        for symbol in self.symbols.values() {
            for declaration in symbol.declarations() {
                if declaration.ast() == index {
                    return Some(declaration);
                }
            }
        }
        None
    }
    pub fn get_usage_by_index(&self, index: usize) -> Option<&Usage> {
        for symbol in self.symbols.values() {
            for usage in symbol.usages() {
                if usage.ast() == index {
                    return Some(usage);
                }
            }
        }
        None
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
