use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::symbol::{Declaration, Filterable, Scope, SymbolKind};
use crate::aiplan4rust::semantic::symbol_table::table::Table;
use crate::aiplan4rust::semantic::symbol_table::SymbolTableError;
use crate::aiplan4rust::tree::NodeId;

impl Table {
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
    /// [`select_valid_declaration`]: Table::apply_semantic_matching
    pub fn resolve_declaration(
        &self,
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<Option<&Declaration>, SymbolTableError> {
        // 1. Accès direct O(1) à l'entrée du symbole
        let entry = match self.symbols.get(symbol_name) {
            Some(e) => e,
            None => return Ok(None),
        };

        let decls = entry.declarations();

        // Modification ici : resolve_candidates génère maintenant un Vec d'index
        // pour que select_valid_declaration puisse travailler sur un slice.
        let mut resolve_candidates = |target_kind: SymbolKind| {
            let candidates: Vec<usize> = decls
                .values()
                .enumerate()
                .filter_map(|(idx, d)| {
                    if d.kind() == target_kind && scope.starts_with(d.scope()) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();

            // On passe le slice d'index &[usize] au sélectionneur
            Self::apply_semantic_matching(symbol_name, usage_kind, decls, &candidates)
        };

        // 3. Logique de priorité Task -> Action
        let best_idx = match usage_kind {
            SymbolKind::Task => {
                if let Some(idx) = resolve_candidates(SymbolKind::Task)? {
                    Some(idx)
                } else {
                    resolve_candidates(SymbolKind::Action)?
                }
            }
            _ => resolve_candidates(*usage_kind)?,
        };

        // 4. On transforme l'index choisi en référence réelle à la toute fin
        Ok(best_idx.and_then(|i| decls.get_index(i).map(|(_, d)| d)))
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
                *symbol_name,
                *usage_kind,
                scope.clone(),
            )),
        }
    }

    pub fn try_resolve_declaration_mut(
        &mut self,
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<&mut Declaration, SymbolTableError> {
        match self.resolve_declaration_mut(symbol_name, usage_kind, scope)? {
            Some(decl) => Ok(decl),
            None => Err(SymbolTableError::declaration_not_found(
                *symbol_name,
                *usage_kind,
                scope.clone(),
            )),
        }
    }

    pub fn resolve_declaration_mut(
        &mut self, // Changement en mut
        symbol_name: &SymbolId,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<Option<&mut Declaration>, SymbolTableError> {
        // Retourne &mut
        // 1. Accès direct O(1) à l'entrée du symbole (en mode mut)
        let entry = match self.symbols.get_mut(symbol_name) {
            Some(e) => e,
            None => return Ok(None),
        };

        // On récupère les déclarations en mode mutable
        let decls = entry.declarations_mut();

        // 2. La logique de recherche d'index reste IDENTIQUE
        // Note : On utilise .values() qui nous donne des &Declaration (immuables)
        // pour le filtrage, ce qui est autorisé même si decls est mut.
        let mut resolve_candidates = |target_kind: SymbolKind| {
            let candidates: Vec<usize> = decls
                .values() // .values() sur un &mut IndexMap renvoie des &T (immuables)
                .enumerate()
                .filter_map(|(idx, d)| {
                    if d.kind() == target_kind && scope.starts_with(d.scope()) {
                        Some(idx)
                    } else {
                        None
                    }
                })
                .collect();

            // On passe 'decls' directement.
            // Rust va "dégrader" la référence mutable en référence immuable
            // juste pour la durée de cet appel.
            Self::apply_semantic_matching(symbol_name, usage_kind, decls, &candidates)
        };

        // 3. Logique de priorité Task -> Action
        let best_idx = match usage_kind {
            SymbolKind::Task => {
                if let Some(idx) = resolve_candidates(SymbolKind::Task)? {
                    Some(idx)
                } else {
                    resolve_candidates(SymbolKind::Action)?
                }
            }
            _ => resolve_candidates(*usage_kind)?,
        };

        // 4. On transforme l'index choisi en référence MUTABLE
        // C'est ici que la magie opère : on ne demande le mut qu'à la fin.
        Ok(best_idx.and_then(|i| decls.get_index_mut(i).map(|(_, d)| d)))
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
            if let Some(decl) = declarations.values().next() {
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
        let mut matching_iter = declarations.values().filter(|decl| {
            let d_scope = decl.scope();
            // Le test de longueur ici aussi évite des comparaisons de vecteurs inutiles
            d_scope.len() <= u_len && decl.kind() == expected_kind && u_scope.starts_with(d_scope)
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
            for usage in entry.usages().values() {
                // L'index ne contient plus que le NodeId vers l'Usage
                self.usage_to_symbol.insert(usage.source(), usage.clone());
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
