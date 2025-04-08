use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::checkers::TypeChecker;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, SymbolKind, Usage};
use crate::aiplan4rust::semantic_analyser::symbol_table::SymbolTable;

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

pub fn check(
    tree: &AnnotatedSyntaxTree,
    type_checker: &TypeChecker,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let symbol_table = tree.symbol_table();
    let ast_table = tree.syntax_tree();
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
                    ast_table,
                    type_checker,
                )? {
                    no_error &= false;
                    let entry = ast_table.get_entry(usage.ast()).unwrap();
                    let (line, column) = entry.span().start_position();
                    let content = format!(
                        "{} '{}' does not match any declaration.",
                        usage.kind(),
                        symbol.name()
                    );
                    let error = ParsingError::new(
                        ParserErrorKind::ParseError, // Error kind can be customized.
                        Some(tree.filename().clone()),
                        line,
                        column,
                        content,
                    );
                    errors.add_error(error);
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
    ast: &HeapSyntaxTree,
    type_checker: &TypeChecker,
) -> Result<bool, ParserInternalError> {
    let ast_usage = ast.get_entry(usage.ast()).ok_or_else(|| {
        ParserInternalError::new(format!("AST entry not found for usage '{}'", usage.ast()))
    })?;

    for (index, argument) in ast_usage.children().iter().skip(1).enumerate() {
        let argument_entry = ast.get_entry(*argument).unwrap();
        let key = argument_entry.get_key(ast)?;
        let kind = match argument_entry.kind() {
            SyntaxNodeKind::Variable(_) => SymbolKind::Variable,
            SyntaxNodeKind::Constant(_) => SymbolKind::Constant,
            SyntaxNodeKind::FunctionTerm => SymbolKind::Function,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST kind encountered: {}",
                    argument_entry.kind()
                )))
            }
        };

        if !match_argument(
            declaration,
            usage,
            symbol_table,
            &key,
            kind,
            index,
            type_checker,
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
    name: &str,
    kind: SymbolKind,
    index: usize,
    type_checker: &TypeChecker,
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
    let symbol_declaration = declarations[0];

    let declared_arguments = declaration.arguments().ok_or_else(|| {
        ParserInternalError::new(format!(
            "Failed to retrieve arguments for declaration in scope {}",
            declaration.scope()
        ))
    })?;

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
    let ty2 = symbol_declaration.types().ok_or_else(|| {
        ParserInternalError::new(format!(
            "Failed to retrieve types for symbol '{}' in scope {}",
            name,
            usage.scope()
        ))
    })?;
    type_checker.match_type(ty1, ty2)
}
//}
