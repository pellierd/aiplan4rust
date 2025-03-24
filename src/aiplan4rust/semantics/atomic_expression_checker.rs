use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::scope::Scope;
use crate::aiplan4rust::semantics::symbol::{Declaration, SymbolKind, Usage};
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::token::{NUMBER_TYPE, OBJECT_TYPE};
use std::collections::HashSet;

/// PDDL Built-in symbols.
const PDDL_BUILTIN_TYPES: [&str; 2] = [OBJECT_TYPE, NUMBER_TYPE];

#[derive(Debug, Clone)]
pub struct AtomicExpressionChecker<'a> {
    symbol_table: &'a SymbolTable,
    ast_table: &'a AstTable,
}

impl<'a> AtomicExpressionChecker<'a> {
    pub fn new(symbol_table: &'a SymbolTable, ast_table: &'a AstTable) -> Self {
        AtomicExpressionChecker {
            symbol_table,
            ast_table,
        }
    }

    pub fn check_domain_atomic_expression(
        &self,
        declaration: &Declaration,
        usage: &Usage,
    ) -> Result<bool, ParserInternalError> {
        self.match_declaration_with_usage(declaration, usage, self.symbol_table)
    }

    pub fn check_problem_atomic_expression(
        &self,
        declaration: &Declaration,
        usage: &Usage,
        domain_symbol_table: &SymbolTable,
    ) -> Result<bool, ParserInternalError> {
        self.match_declaration_with_usage(declaration, usage, domain_symbol_table)
    }

    /// Checks that each symbol used in the `usage` matches its declaration in the `symbol_table`.
    ///
    /// This function iterates over the arguments of the usage's AST (skipping the first element,
    /// which typically represents the predicate or main function symbol) and:
    ///
    /// - Uses `get_key()` to retrieve the key for the symbol. For a `FunctionTerm`, `get_key()`
    ///   handles formatting the key as "functor/arity".
    /// - Determines the symbol's type based on the AST kind (e.g., variable, constant, function).
    /// - Calls `check_symbol_type` to verify that the symbol's type and usage match its
    ///   declaration.
    ///
    /// # Arguments
    ///
    /// * `declaration` - A reference to the declaration against which usage is to be matched.
    /// * `usage` - A reference to the usage instance containing the AST of the symbol usage.
    /// * `symbol_table` - A reference to the symbol table used to resolve symbol declarations.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if all symbols in the usage match their declaration in the symbol table.
    /// * `Ok(false)` if any symbol's type does not match its declaration.
    /// * `Err(ParserInternalError)` if any error occurs during processing (e.g., unexpected AST
    ///   kind).
    fn match_declaration_with_usage(
        &self,
        declaration: &Declaration,
        usage: &Usage,
        domain_symbol_table: &SymbolTable,
    ) -> Result<bool, ParserInternalError> {
        // Retrieve the AST associated with the usage.
        let ast_usage = self.ast_table.get_entry(usage.ast()).unwrap();

        // Iterate over the children of the AST starting from the second element.
        // The first element is typically not part of the symbol arguments.
        for (index, argument) in ast_usage.children().iter().skip(1).enumerate() {
            // Use get_key() to obtain the symbol's key (name) from the AST.
            let argument_entry = self.ast_table.get_entry(*argument).unwrap();
            let key = argument_entry.get_key(&self.ast_table)?;
            // Determine the symbol's type based on the AST kind.
            // This is used later to check the symbol against its declaration.
            let kind = match argument_entry.kind() {
                AstKind::Variable(_) => SymbolKind::Variable,
                AstKind::Constant(_) => SymbolKind::Constant,
                AstKind::FunctionTerm => SymbolKind::Function,
                _ => {
                    return Err(ParserInternalError::new(format!(
                        "Unexpected AST kind encountered: {}",
                        argument_entry.kind()
                    )))
                }
            };

            // Check if the symbol's type matches its declaration using the key.
            // If check_symbol_type returns false, the usage does not match the declaration.
            if !self.match_argument(declaration, usage, domain_symbol_table, &key, kind, index)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Compares the type of a specific argument in a declaration with the type associated
    /// with the corresponding symbol in the symbol table.
    ///
    /// This function retrieves the symbol declaration from the symbol table using the provided
    /// name, kind, and usage scope. It then extracts the expected type from the declaration’s
    /// arguments at the given index and the actual type from the symbol’s declaration. Finally,
    /// it invokes `match_type` to verify that the two types are consistent.
    ///
    /// # Parameters
    ///
    /// * `declaration` - A reference to the declaration containing the expected argument types.
    /// * `usage` - A reference to the usage instance containing the symbol's usage context.
    /// * `symbol_table` - A reference to the symbol table used for resolving symbol declarations.
    /// * `name` - The key (name) of the symbol to be matched.
    /// * `kind` - The kind of the symbol (e.g., Variable, Constant, Function, etc.).
    /// * `index` - The argument index in the declaration that corresponds to the symbol usage.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the type of the argument in the declaration matches the type associated with
    ///   the symbol in the symbol table.
    /// * `Ok(false)` if the types do not match.
    /// * `Err(ParserInternalError)` if any error occurs during lookup or type retrieval, such as:
    ///   - The symbol declaration is not found in the symbol table.
    ///   - The declaration has no arguments.
    ///   - The provided index is out of bounds.
    ///   - The types cannot be retrieved for comparison.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - No declaration is found for the symbol in the symbol table.
    /// - The declaration's arguments cannot be retrieved.
    /// - The specified argument index is out of bounds.
    /// - The types for the symbol or the argument cannot be retrieved.
    fn match_argument(
        &self,
        declaration: &Declaration,
        usage: &Usage,
        domain_table: &SymbolTable,
        name: &str,
        kind: SymbolKind,
        index: usize,
    ) -> Result<bool, ParserInternalError> {
        let declarations = self.symbol_table.get_declarations_by_filter(
            Some(name),
            Some(&kind),
            Some(usage.scope()),
        );

        if declarations.is_empty() {
            println!("{}", self.symbol_table);
            return Err(ParserInternalError::new(format!(
                "No declaration found for symbol '{}' in scope {}.",
                name,
                usage.scope()
            )));
        }

        if declarations.len() > 1 {
            return Err(ParserInternalError::new(format!(
                "Expected exactly one declaration for symbol '{}' in scope {}. Found {} declarations.",
                name,
                usage.scope(),
                declarations.len()
            )));
        }
        let symbol_declaration = declarations[0]; // La déclaration unique

        // Retrieve the arguments from the declaration.
        let declared_arguments = declaration.arguments().ok_or_else(|| {
            ParserInternalError::new(format!(
                "Failed to retrieve arguments for declaration in scope {}",
                declaration.scope()
            ))
        })?;

        // Retrieve the expected types for the argument at the given index.
        let ty1 = declared_arguments
            .get(index)
            .ok_or_else(|| {
                ParserInternalError::new(format!(
                    "Argument index {} out of bounds for declaration in scope {}",
                    index,
                    declaration.scope()
                ))
            })?
            .types();
        // Retrieve the types for the symbol from its declaration.
        let ty2 = symbol_declaration.types().ok_or_else(|| {
            ParserInternalError::new(format!(
                "Failed to retrieve types for symbol '{}' in scope {}",
                name,
                usage.scope()
            ))
        })?;
        // Compare the two types using the match_type function.
        Self::match_type(ty1, ty2, domain_table, declaration.scope())
    }

    /// Compares two sets of types to determine if they are compatible within a given scope.
    ///
    /// This function checks if there is an overlap between two sets of types by looking for a match
    /// in the first type set (`ty1`) and any type from the second set (`ty2`). The comparison is
    /// based on the ascending type closure, which considers parent or derived types, and checks if
    /// any type in `ty2` is compatible with types in `ty1` within the given scope.
    ///
    /// # Arguments
    /// * `ty1` - A reference to a vector of strings representing the first set of types to be
    ///   checked.
    /// * `ty2` - A reference to a vector of strings representing the second set of types to be
    ///   checked.
    /// * `symbol_table` - A reference to the symbol table, used to resolve type hierarchies and
    ///   closures.
    /// * `scope` - The scope within which the types are evaluated, used to determine compatibility
    ///   based on parent/derived types.
    ///
    /// # Returns
    /// `true` if there is at least one type in `ty2` that is compatible with any type in `ty1`
    /// based on type closure, `false` otherwise.
    ///
    fn match_type(
        ty1: &Vec<String>,
        ty2: &Vec<String>,
        symbol_table: &SymbolTable,
        scope: &Scope,
    ) -> Result<bool, ParserInternalError> {
        let ty1_set: HashSet<_> = ty1.iter().cloned().collect();

        // Iterate over each type in ty2
        for ty in ty2.iter() {
            // Get the ascending type closure for the current type
            let closure = Self::ascending_type_closure(symbol_table, ty, scope)?;

            // Check if there is any overlap between the closure and ty1_set
            let mut is_disjoint = true;
            for closure_ty in closure.iter() {
                if ty1_set.contains(closure_ty) {
                    is_disjoint = false;
                    break; // Found a match, no need to check further
                }
            }

            // If a match is found, return Ok(true)
            if !is_disjoint {
                return Ok(true);
            }
        }

        // If no compatible type is found after checking all types in ty2
        Ok(false)
    }

    /// Collects the hierarchy of types for a given primitive type.
    ///
    /// This function gathers all types related to the given `primitive_type`, including
    /// its direct super-types and their super-types recursively. It returns a `HashSet`
    /// containing the primitive type itself and all its super-types.
    ///
    /// # Arguments
    /// * `symbol_table` - A reference to the `SymbolTable` used to look up symbol declarations.
    /// * `primitive_type` - A reference to a `String` representing the primitive type.
    /// * `scope` - A reference to the `Scope` in which the symbol is being searched.
    ///
    /// # Returns
    /// * `Result<HashSet<String>, ParserInternalError>` - A result containing a set of all the
    ///   primitive type and its super-types, or an error if no declaration is found for any type in
    ///   the hierarchy.
    ///
    /// # Algorithm
    /// The function initializes a stack with the `primitive_type`. It then iteratively
    /// explores the type hierarchy by looking for super-types associated with the current
    /// type. If a super-type is found, it is added to the stack and processed recursively.
    /// All encountered types are stored in a `HashSet` to avoid duplicates.
    ///
    /// If no declaration for a symbol is found during the traversal, an error is returned.
    ///
    /// # Example
    /// ```rust
    /// let result = ascending_type_closure(symbol_table, "SomeType".to_string(), &scope);
    /// match result {
    ///     Ok(types) => { /* process types */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    pub fn ascending_type_closure(
        symbol_table: &SymbolTable,
        primitive_type: &String,
        scope: &Scope,
    ) -> Result<HashSet<String>, ParserInternalError> {
        let mut super_types = HashSet::new();
        let mut to_visit = vec![primitive_type]; // Initialize with the current type

        while let Some(current_type) = to_visit.pop() {
            // Insert into the set if the type is not already present
            if super_types.insert(current_type.clone()) {
                if AtomicExpressionChecker::is_pddl_builtin_types(&current_type.as_str()) {
                    continue;
                }
                let declarations = symbol_table.get_declarations_by_filter(
                    Some(current_type),
                    Some(&SymbolKind::PrimitiveType),
                    Some(scope),
                );

                if !declarations.is_empty() {
                    if declarations.len() > 1 {
                        return Err(ParserInternalError::new(format!(
                            "Expected exactly one declaration for symbol '{}', but found {} declarations.",
                            current_type,
                            declarations.len()
                        )));
                    }

                    let ty_symbol = declarations[0];

                    // Retrieve the symbols corresponding to the type
                    if let Some(s_types) = ty_symbol.types() {
                        // Dereference s_types to get the actual HashSet<String> and extend the stack
                        to_visit.extend(s_types.iter());
                    }
                }
            }
        }

        Ok(super_types)
    }

    /// Checks if the given type is a PDDL built-in symbol.
    ///
    /// This function checks if the provided type string matches any of the predefined
    /// built-in symbols in the PDDL language. The set of PDDL built-in symbols is
    /// stored in the constant `PDDL_BUILTIN_SYMBOLS`.
    ///
    /// # Arguments
    /// * `ty` - A reference to a string representing the type to check.
    ///
    /// # Returns
    /// * `bool` - `true` if the type is a PDDL built-in symbol, `false` otherwise.
    ///
    pub fn is_pddl_builtin_types(ty: &str) -> bool {
        PDDL_BUILTIN_TYPES.contains(&ty)
    }
}
