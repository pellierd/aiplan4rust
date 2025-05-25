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
            let filtered_declarations = SymbolTable::get_by_filter(kind, scope, declarations);

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
            let filtered_usages = SymbolTable::get_by_filter(kind, scope, declarations);

            // Si des déclarations correspondantes sont trouvées, ajouter le symbole à la réponse
            if !filtered_usages.is_empty() {
                result.push(symbol);
            }
        }

        result
    }

    pub fn get_declarations_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Declaration> {
        if let Some(name) = symbol_name {
            // Si on connaît le nom, on récupère directement le symbole
            if let Some(symbol) = self.symbols.get(name) {
                // Filtrage sur les déclarations du symbole trouvé
                return SymbolTable::get_by_filter(kind, scope, symbol.declarations());
            } else {
                // Pas de symbole avec ce nom, on retourne un vecteur vide
                return Vec::new();
            }
        }

        // Sinon, on parcourt tous les symboles comme avant
        self.symbols.values()
            .flat_map(|symbol| SymbolTable::get_by_filter(kind, scope, symbol.declarations()))
            .collect()
    }

    pub fn get_usages_by_filter(
        &self,
        symbol_name: Option<&str>,
        kind: Option<&SymbolKind>,
        scope: Option<&Scope>,
    ) -> Vec<&Usage> {
        if let Some(name) = symbol_name {
            if let Some(symbol) = self.symbols.get(name) {
                return SymbolTable::get_by_filter(kind, scope, symbol.usages());
            } else {
                return Vec::new();
            }
        }

        self.symbols.values()
            .flat_map(|symbol| SymbolTable::get_by_filter(kind, scope, symbol.usages()))
            .collect()
    }

    pub fn get_by_filter<'a, T: FilterableSymbol>(
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

    /// Retrieves the declarations associated with a specific usage index.
    ///
    /// This function searches through the symbols and their usages to find any
    /// usages that match the given `index`. If a matching usage is found, it then
    /// checks the associated declarations and returns them if their scopes match.
    /// The result is wrapped in an `Option` to differentiate between a missing usage
    /// and the case where matching declarations are found.
    ///
    /// # Parameters
    /// - `index`: The index of the usage to search for.
    ///
    /// # Returns
    /// - `Some(Vec<&Declaration>)`: A vector containing the declarations that match
    ///   the given usage index and scope.
    /// - `None`: If no matching usage is found for the provided index, or if no
    ///   declarations are found that match the usage's scope.
    ///
    /// # Example
    /// ```rust
    /// let result = get_declaration_by_usage(42);
    /// match result {
    ///     Some(declarations) => {
    ///         for declaration in declarations {
    ///             // Handle each declaration
    ///         }
    ///     },
    ///     None => {
    ///         // Handle the case where no matching usage was found
    ///         println!("No usage found for index 42.");
    ///     }
    /// }
    /// ```
    pub fn get_declaration_by_usage(
        &self,
        index: usize,
    ) -> Result<Vec<&Declaration>, ParserInternalError> {
        let mut result = Vec::new();

        // Iterate through each symbol in the symbols map
        for symbol in self.symbols.values() {
            // Iterate through the usages of the current symbol
            for usage in symbol.usages() {
                // If a usage matches the provided index
                if usage.ast() == index {
                    // Search through the declarations of the current symbol
                    for declaration in symbol.declarations() {
                        // Check if the scope of the declaration matches the usage
                        if usage.scope().starts_with(declaration.scope()) {
                            result.push(declaration);
                        }
                    }

                    // If we found matching declarations, return them immediately
                    if !result.is_empty() {
                        return Ok(result);
                    }
                }
            }
        }

        // If the index was never found in usages, return an error
        Err(ParserInternalError::new(format!(
            "AST index {} not found in symbol table usages.",
            index
        )))
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for symbol in self.symbols.values() {
            writeln!(f, "{}", symbol)?;
        }
        Ok(())
    }
}
