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

/// A struct for performing type checking within a given domain.
///
/// This struct holds a reference to the `domain_symbol_table`, which contains the symbol declarations
/// and type information used to verify the types of symbols within the domain. The `TypeChecker` is
/// responsible for checking type compatibility, resolving type hierarchies, and ensuring that symbols
/// are correctly used according to their defined types.
///
/// # Fields
///
/// * `domain_symbol_table` - A reference to the `SymbolTable` that contains the symbol declarations
///   for the domain. This is used to resolve type information and check the type hierarchy of symbols
///   during type checking.
///
/// # Derives
///
/// The `TypeChecker` struct derives the following traits:
/// - `Debug`: Enables the ability to format the `TypeChecker` instance for debugging purposes.
/// - `Clone`: Allows cloning of `TypeChecker` instances, enabling multiple instances to share
///   the same `domain_symbol_table` without ownership issues.
#[derive(Debug, Clone)]
pub struct TypeChecker<'a> {
    domain_symbol_table: &'a SymbolTable,
}

impl<'a> TypeChecker<'a> {
    /// Constructs a new `TypeChecker` instance.
    ///
    /// This function initializes a `TypeChecker` with a reference to the `domain_symbol_table`.
    /// The `domain_symbol_table` plays a crucial role in retrieving the type hierarchy associated
    /// with the symbols, which is essential for performing type checking and matching symbols
    /// with their expected types.
    ///
    /// # Arguments
    ///
    /// * `domain_symbol_table` - A reference to the `SymbolTable` that contains the symbol
    ///   declarations and is used to retrieve the type hierarchy. This table is key for resolving
    ///   type information  for symbols and their relationships in the context of the given domain.
    ///
    /// # Returns
    ///
    /// A new instance of `TypeChecker` initialized with the provided `domain_symbol_table`.
    pub fn new(domain_symbol_table: &'a SymbolTable) -> Self {
        TypeChecker {
            domain_symbol_table,
        }
    }

    /// Checks that each symbol used in the `usage` matches its declaration in the `symbol_table`.
    ///
    /// This function iterates over the arguments in the usage's AST, skipping the first element,
    /// which typically represents the predicate or main function symbol. For each symbol in the
    /// usage, the function:
    ///
    /// - Uses `get_key()` to retrieve the key for the symbol. For a `FunctionTerm`, `get_key()`
    ///   formats the key as "functor/arity".
    /// - Determines the symbol's type based on the AST kind (e.g., variable, constant, function).
    /// - Calls `check_symbol_type` to verify that the symbol's type and usage match its
    ///   declaration.
    ///
    /// # Arguments
    ///
    /// * `declaration` - A reference to the declaration against which the usage is to be matched.
    /// * `usage` - A reference to the usage instance containing the AST of the symbol usage.
    /// * `symbol_table` - A reference to the symbol table used to resolve symbol declarations.
    /// * `ast` - A reference to the AST table containing the symbols used in the usage.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if all symbols in the usage match their declaration in the symbol table.
    /// * `Ok(false)` if any symbol's type does not match its declaration.
    /// * `Err(ParserInternalError)` if any error occurs during processing (e.g., unexpected AST
    ///   kind or failure to retrieve symbol information).
    ///
    /// # Errors
    /// This function may return an error if:
    /// - An unexpected AST kind is encountered (i.e., an unsupported symbol type).
    /// - The `get_key()` method fails to retrieve a valid key.
    /// - An issue occurs when looking up a symbol's type or matching it against its declaration.
    ///
    /// # Algorithm
    /// The function first retrieves the AST associated with the `usage`. Then it iterates over each
    /// argument in the usage's AST (skipping the first element) and uses the `get_key()` function
    /// to obtain the symbol's key. The symbol's type is determined based on its AST kind (variable,
    /// constant, or function term). The function then compares the type of each symbol with its
    /// corresponding declaration using `match_argument`. If any type mismatch is detected, the
    /// function returns `Ok(false)`; otherwise, it returns `Ok(true)`.
    ///
    /// # Example
    /// ```rust
    /// let result = type_checker.match_declaration_with_usage(&declaration, &usage, &symbol_table, &ast);
    /// match result {
    ///     Ok(true) => { /* all symbols match */ },
    ///     Ok(false) => { /* some symbol types do not match */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    ///
    /// # Notes
    /// - The first argument in the AST is typically not considered part of the symbol arguments.
    /// - The function handles symbols of different types, including variables, constants, and
    ///   function terms.
    /// - It assumes that the `ast` provides valid entries for the `usage` and its arguments.

    pub fn match_declaration_with_usage(
        &self,
        declaration: &Declaration,
        usage: &Usage,
        symbol_table: &SymbolTable,
        ast: &AstTable,
    ) -> Result<bool, ParserInternalError> {
        // Retrieve the AST associated with the usage.
        let ast_usage = ast.get_entry(usage.ast()).ok_or_else(|| {
            ParserInternalError::new(format!("AST entry not found for usage '{}'", usage.ast()))
        })?;

        // Iterate over the children of the AST starting from the second element.
        // The first element is typically not part of the symbol arguments.
        for (index, argument) in ast_usage.children().iter().skip(1).enumerate() {
            // Use get_key() to obtain the symbol's key (name) from the AST.
            let argument_entry = ast.get_entry(*argument).unwrap();
            let key = argument_entry.get_key(ast)?;
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
            if !self.match_argument(declaration, usage, symbol_table, &key, kind, index)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Compares the type of a specific argument in a declaration with the type associated
    /// with the corresponding symbol in the symbol table.
    ///
    /// This function retrieves the symbol declaration from the symbol table using the provided
    /// `name`, `kind`, and `usage` scope. It then extracts the expected type from the declaration's
    /// arguments at the given `index` and compares it with the actual type associated with the
    /// symbol in the symbol table. The comparison is performed by invoking `match_type` to verify
    /// that the two types are compatible.
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
    ///
    /// # Algorithm
    /// The function begins by looking up the symbol's declaration in the symbol table based on the
    /// provided `name`, `kind`, and `usage` scope. If exactly one declaration is found, it proceeds
    /// to retrieve the arguments of the provided `declaration` and the types associated with the
    /// symbol in its declaration. It then compares the types at the given `index` using the
    /// `match_type` function. If any of the retrieval steps fail, an error is returned with an
    /// appropriate message.
    ///
    /// # Example
    /// ```rust
    /// let result = type_checker.match_argument(&declaration, &usage, &symbol_table, "symbol_name", SymbolKind::Variable, 0);
    /// match result {
    ///     Ok(true) => { /* argument types match */ },
    ///     Ok(false) => { /* argument types do not match */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    ///
    /// # Notes
    /// - This function relies on the `match_type` function to perform the actual comparison of
    ///   types.
    /// - The provided `declaration` should contain arguments with types, and the provided `index`
    ///   should be valid for those arguments.
    fn match_argument(
        &self,
        declaration: &Declaration,
        usage: &Usage,
        symbol_table: &SymbolTable,
        name: &str,
        kind: SymbolKind,
        index: usize,
    ) -> Result<bool, ParserInternalError> {
        let declarations =
            symbol_table.get_declarations_by_filter(Some(name), Some(&kind), Some(usage.scope()));

        if declarations.is_empty() {
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
        self.match_type(ty1, ty2)
    }

    /// Compares two sets of types to determine if they are compatible within a given scope.
    ///
    /// This function checks if there is an overlap between two sets of types by looking for a match
    /// in the first type set (`ty1`) and any type from the second set (`ty2`). It performs this
    /// comparison based on the ascending type closure, which considers parent or derived types. The
    /// function checks if any type in `ty2` is compatible with types in `ty1` by looking for a
    /// common type in their type hierarchies within the given `scope`.
    ///
    /// # Arguments
    /// * `ty1` - A reference to a vector of strings representing the first set of types to be
    ///   checked.
    /// * `ty2` - A reference to a vector of strings representing the second set of types to be
    ///   checked.
    ///
    /// # Returns
    /// * `Ok(true)` if there is at least one type in `ty2` that is compatible with any type in `ty1`
    ///   based on their type closure.
    /// * `Ok(false)` if no compatible type is found after checking all types in `ty2`.
    /// * `Err(ParserInternalError)` if an error occurs while fetching the ascending type closure.
    ///
    /// # Algorithm
    /// This function first converts `ty1` into a `HashSet` to ensure that type lookups are
    /// efficient. It then iterates over each type in `ty2`, checking if any type from `ty2` is
    /// compatible with the types in `ty1`. For each type in `ty2`, the function retrieves the
    /// ascending type closure, which is a set of that type and all of its super-types.
    /// It checks for any overlap between the closure and `ty1` by looking for common elements.
    /// If a match is found, the function returns `Ok(true)`. If no match is found after iterating
    /// through all types in `ty2`, it returns `Ok(false)`.
    ///
    /// # Example
    /// ```rust
    /// let ty1 = vec!["TypeA".to_string(), "TypeB".to_string()];
    /// let ty2 = vec!["TypeC".to_string()];
    /// let result = type_checker.match_type(&ty1, &ty2);
    /// match result {
    ///     Ok(true) => { /* types are compatible */ },
    ///     Ok(false) => { /* types are not compatible */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    ///
    /// # Notes
    /// This function assumes that `ascending_type_closure` works as expected and properly handles
    /// type hierarchies. It relies on the type closure mechanism to determine compatibility.
    pub fn match_type(
        &self,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        let ty1_set: HashSet<_> = ty1.iter().cloned().collect();

        // Iterate over each type in ty2
        for ty in ty2.iter() {
            // Get the ascending type closure for the current type
            let closure = self.ascending_type_closure(ty)?;

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

    /// Collects the hierarchy of types for a given primitive type, including its super-types.
    ///
    /// This function gathers all types related to the given `primitive_type`, including its
    /// direct super-types and their super-types recursively. It returns a `HashSet` containing
    /// the `primitive_type` itself and all its super-types. The function works by exploring
    /// the type hierarchy in a depth-first manner, ensuring that all types encountered are unique
    /// by utilizing a `HashSet`.
    ///
    /// # Arguments
    /// * `primitive_type` - A reference to a `String` representing the primitive type for which
    ///   the hierarchy of types will be collected.
    ///
    /// # Returns
    /// * `Result<HashSet<String>, ParserInternalError>` - A result containing a set of all the
    ///   `primitive_type` and its super-types, or an error if no declaration is found for any type
    ///   in the hierarchy.
    ///
    /// # Algorithm
    /// The function begins with a stack initialized with the given `primitive_type`. It then
    /// iteratively explores the type hierarchy by looking for super-types associated with the
    /// current type. If a super-type is found, it is added to the stack for further processing.
    /// All encountered types are stored in a `HashSet` to ensure that duplicates are avoided. The
    /// search continues recursively until no more super-types are found.
    ///
    /// If no declaration is found for a symbol during the traversal, the function returns an error.
    /// Additionally, if multiple declarations are found for a single symbol, an error is raised,
    /// as only one declaration is expected per symbol.
    ///
    /// # Example
    /// ```rust
    /// let result = type_checker.ascending_type_closure("SomeType".to_string());
    /// match result {
    ///     Ok(types) => { /* process types */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    ///
    /// # Notes
    /// This function assumes that `domain_symbol_table` is populated with declarations for types
    /// and that `Scope::root()` is a valid starting point for the search. Built-in PDDL types are
    /// ignored during the search.
    pub fn ascending_type_closure(
        &self,
        primitive_type: &String,
    ) -> Result<HashSet<String>, ParserInternalError> {
        let mut super_types = HashSet::new();
        let mut to_visit = vec![primitive_type]; // Initialize with the current type

        while let Some(current_type) = to_visit.pop() {
            // Insert into the set if the type is not already present
            if super_types.insert(current_type.clone()) {
                if TypeChecker::is_pddl_builtin_types(&current_type.as_str()) {
                    continue;
                }
                let declarations = self.domain_symbol_table.get_declarations_by_filter(
                    Some(current_type),
                    Some(&SymbolKind::PrimitiveType),
                    Some(&Scope::root()),
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
