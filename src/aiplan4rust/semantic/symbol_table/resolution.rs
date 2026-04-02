use crate::aiplan4rust::semantic::symbol::{Declaration, SymbolKind};
use crate::aiplan4rust::semantic::symbol_table::table::Table;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;

impl Table {
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
        let declarations = self.collect_declarations(None, Some(&kind), Some(&self.root_scope()));

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
