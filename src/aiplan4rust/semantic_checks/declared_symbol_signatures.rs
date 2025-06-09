use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::analyser::symbol::Declaration;
use crate::aiplan4rust::analyser::symbol::SymbolKind;
use crate::aiplan4rust::analyser::symbol::Usage;
use crate::aiplan4rust::analyser::symbol_table::SymbolTable;
use crate::aiplan4rust::analyser::{AnnotatedSyntaxNode, AnnotatedSyntaxTree, TypeChecker};

/// Checks for errors in the symbol declarations and their usages in the given annotated syntax tree.
///
/// This function scans through the `symbol_table` of the provided `tree` to match each symbol's
/// declarations and usages. It ensures that symbols used in the tree are correctly declared and
/// that their types match the expected types. Errors are added to the provided `ErrorManager`
/// during the process.
///
/// # Arguments
///
/// * `tree` - An `AnnotatedSyntaxTree` that contains the symbols to check.
/// * `type_checker` - A `TypeChecker` used to validate types during the check.
/// * `errors` - A mutable reference to an `ErrorManager` where any errors found during the check
///   will be added.
///
/// # Returns
///
/// A `Result<bool, ParserInternalError>` where:
/// * `Ok(true)` indicates that no errors were found during the check.
/// * `Ok(false)` indicates that errors were found and added to the `ErrorManager`.
/// * `Err(ParserInternalError)` indicates an internal error occurred during the process.
///
/// # Example
///
/// ```rust
/// let mut errors = ErrorManager::new();
/// if atomic_formula_checker::check(&tree, &type_checker, &mut errors).is_ok() {
///     // Handle no errors
/// } else {
///     // Handle errors
///     self.error_manager.add_errors_from(&errors);
/// }
/// ```

pub fn check_declared_symbol_signatures(
    syntax_tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let symbol_table = syntax_tree.symbol_table();
    let mut no_error = true;

    // Loop over all symbols in the symbol table.
    for symbol in symbol_table.values() {
        // Check all declarations of the symbol.
        for declaration in symbol.declarations() {
            if !matches!(
                declaration.kind(),
                SymbolKind::Predicate
                    | SymbolKind::Function
                    | SymbolKind::Task
                    | SymbolKind::Action
            ) {
                continue;
            }

            // Check all usages of the symbol.
            for usage in symbol.usages() {
                if !match_declaration_with_usage(
                    declaration,
                    usage,
                    symbol_table,
                    syntax_tree,
                    type_checker,
                    diagnostic_manager,
                )? {
                    no_error &= false;
                    let entry = syntax_tree.get_entry(usage.ast()).unwrap();
                    let diagnostic_kind = match declaration.kind() {
                        SymbolKind::Predicate => DiagnosticKind::UnDefinedPredicate {
                            symbol: symbol.name().clone()
                        },
                        SymbolKind::Function => DiagnosticKind::UnDefinedFunction {
                            symbol: symbol.name().clone()
                        },
                        SymbolKind::Task => DiagnosticKind::UnDefinedCompoundTask {
                            symbol: symbol.name().clone()
                        },
                        SymbolKind::Action => DiagnosticKind::UnDefinedPrimitiveTask {
                            symbol: symbol.name().clone()
                        },
                        _ => unreachable!(),
                    };

                    let error = Diagnostic::new(
                        diagnostic_kind,
                        DiagnosticSource::SemanticAnalyzer,
                        syntax_tree.filename().clone(),
                        entry.span().clone(),
                    );

                    diagnostic_manager.add_diagnostic(error);
                }
            }
        }
    }

    Ok(no_error)
}

/// Matches a declaration to its usage, verifying that the argument types are correct and match.
///
/// This method ensures that the declaration and usage of a symbol are consistent with each other.
/// It checks if the argument types in the usage match the types in the declaration.
///
/// # Arguments
///
/// * `declaration` - The declaration of the symbol being used.
/// * `usage` - The usage of the symbol in the AST.
/// * `symbol_table` - The table containing the symbols for reference.
/// * `ast` - The AST table for resolving entries and their types.
/// * `type_checker` - A type checker used to validate the matching types.
///
/// # Returns
///
/// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the declaration and usage match,
/// `Ok(false)` if they don't, or a `ParserInternalError` if any error occurs.
fn match_declaration_with_usage(
    declaration: &Declaration,
    usage: &Usage,
    symbol_table: &SymbolTable,
    syntax_tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let ast_usage = syntax_tree.get_entry(usage.ast()).ok_or_else(|| {
        ParserInternalError::new(format!("AST entry not found for usage '{}'", usage.ast()))
    })?;

    for (index, argument_index) in ast_usage.children().iter().skip(1).enumerate() {
        let argument = syntax_tree.get_entry(*argument_index).unwrap();

        let kind = match argument.kind() {
            SyntaxNodeKind::Variable(_) => SymbolKind::Variable,
            SyntaxNodeKind::Constant(_) => SymbolKind::Constant,
            SyntaxNodeKind::FunctionTerm => SymbolKind::Function,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST kind encountered: {}",
                    argument.kind()
                )))
            }
        };

        if !match_argument(
            declaration,
            usage,
            symbol_table,
            syntax_tree,
            argument,
            kind,
            index,
            type_checker,
            diagnostic_manager
        )? {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Matches a specific argument in the declaration to its expected type.
///
/// This function verifies that the argument in the usage corresponds to the declaration,
/// ensuring that types match correctly and the argument is within valid bounds.
///
/// # Arguments
///
/// * `declaration` - The declaration of the symbol.
/// * `usage` - The usage of the symbol.
/// * `symbol_table` - The table containing symbols.
/// * `name` - The name of the argument being matched.
/// * `kind` - The kind of the argument, such as `SymbolKind::Variable` or `SymbolKind::Function`.
/// * `index` - The index of the argument in the argument list.
/// * `type_checker` - A type checker to validate type consistency.
///
/// # Returns
///
/// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the argument matches the expected
/// declaration, or `Err` with a `ParserInternalError` if any validation error occurs.
fn match_argument(
    declaration: &Declaration,
    usage: &Usage,
    symbol_table: &SymbolTable,
    syntax_tree: &AnnotatedSyntaxTree,
    argument: &AnnotatedSyntaxNode,
    kind: SymbolKind,
    index: usize,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Retrieve the symbol name associated with the argument from the annotated syntax tree
    let name = argument.get_symbol(syntax_tree)?;
    // Check that the symbol exists; return an error if it is missing
    let name = match name {
        Some(n) => n,
        None => {
            return Err(ParserInternalError::new(format!(
                "Symbol for argument at index {} not found",
                index
            )))
        }
    };

    // Look up the corresponding declaration in the symbol table,
    // given the expected kind and usage scope
    let symbol_declaration = match symbol_table.resolve_declaration(name, &kind, usage.scope())? {
        Some(decl) => decl,
        None => {
            return Err(ParserInternalError::new(format!(
                "No declaration found for symbol '{}' in scope {}.",
                name,
                usage.scope()
            )))
        }
    };

    // Get the declared arguments of the main declaration (the context declaration)
    let declared_arguments = match declaration.arguments() {
        Some(args) => args,
        None => {
            return Err(ParserInternalError::new(format!(
                "Failed to retrieve arguments for declaration in scope {}",
                declaration.scope()
            )))
        }
    };

    // Retrieve the type of the i-th declared argument (the one we are matching)
    let ty1 = match declared_arguments.get(index) {
        Some(arg) => arg.types(),
        None => {
            return Err(ParserInternalError::new(format!(
                "Argument index {} out of bounds for declaration in scope {}",
                index,
                declaration.scope()
            )))
        }
    };

    // Retrieve the type of the symbol from the declaration found in the symbol table
    let ty2 = match symbol_declaration.types() {
        Some(types) => types,
        None => {
            return Err(ParserInternalError::new(format!(
                "Failed to retrieve types for symbol '{}' in scope {}",
                name,
                usage.scope()
            )))
        }
    };

    // Special case: allow a primitive task `(t ?x)` declared in a method
    // where `?x` has type A to match an action `a` where `?x` has type B,
    // as long as B is a supertype of A. This permits upcasting at usage time.
    //
    // Semantically this is questionable and should be handled explicitly during grounding.
    // This occurs, for example, in the `ultralight_cockpit` domain.
    //
    // Outside this exception, strict subtype checking is applied.

    // Check if ty1 is a subtype of ty2 (ty1 <: ty2)
    let is_subtype = type_checker.is_any_subtype_of(ty1, ty2)?;

    // Special tolerated case: accept a primitive task matching an action/method with a supertype
    if !is_subtype
        && (*declaration.kind() == SymbolKind::Action
        || *declaration.kind() == SymbolKind::DASymbol
        || *declaration.kind() == SymbolKind::Method)
        && *usage.kind() == SymbolKind::Task
    {
        // Add a warning diagnostic for this special case
        let warning = Diagnostic::new(
            DiagnosticKind::WarningTaskArgumentIsSupertypeOfDeclaration {
                argument: name.clone(),
                type_declared: ty1.clone(),
                type_used: ty2.clone(),
            },
            DiagnosticSource::SemanticAnalyzer,
            syntax_tree.filename().clone(),
            argument.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);

        // Accept the match if ty1 is a supertype of ty2 (ty1 :> ty2)
        return type_checker.is_any_supertype_of(ty1, ty2);
    }

    // Normal case: return the result of the subtype check
    Ok(is_subtype)
}
