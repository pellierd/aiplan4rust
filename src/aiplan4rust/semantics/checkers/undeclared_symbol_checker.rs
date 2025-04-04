use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::symbol::Scope;
use crate::aiplan4rust::semantics::symbol::{Declaration, Symbol, SymbolKind, Usage};
use crate::aiplan4rust::syntax::ast::Requirement::{Adl, DurativeActions, NumericFluents, Typing};
use crate::aiplan4rust::syntax::token::{DURATION_VARIABLE, NUMBER_TYPE, OBJECT_TYPE, TOTAL_TIME};

/// Checks if there are any undeclared symbols used in the given symbol table.
/// This function scans all usages of symbols in the `symbol_table` and verifies if
/// each symbol has been declared correctly. If no declaration is found for a symbol,
/// an error is generated and added to the error manager.
///
/// # Arguments
///
/// * `symbol_table` - A reference to the `SymbolTable` that contains all the symbols.
/// * `ast_table` - A reference to the `AstTable` for looking up AST entries related to the usages.
/// * `skip_symbols` - A list of `SymbolKind`s to skip during the check.
///
/// # Returns
///
/// * `Result<bool, ParserInternalError>` - Returns `Ok(true)` if no undeclared symbol was found,
///   otherwise `Ok(false)`. Returns an error if an internal parser error occurs.
///
/// # Example
///
/// ```
/// let result = check(&symbol_table, &ast_table, &[SymbolKind::Action]);
/// match result {
///     Ok(true) => println!("No undeclared symbols found."),
///     Ok(false) => println!("Some undeclared symbols were found."),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub fn check(
    tree: &AnnotatedSyntaxTree,
    skip_symbols: &[SymbolKind],
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    let symbol_table = tree.symbol_table();
    let ast_table = tree.ast();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        // Iterate over all usages of the symbol.
        for usage in symbol.usages() {
            // Skip the symbol if it meets the criteria (e.g., already declared or needs to be skipped).
            if should_skip_symbol(symbol, ast_table, usage.kind(), skip_symbols)? {
                continue;
            }

            // Check if the declaration for the symbol was found.
            if !is_declaration_found(symbol, usage) {
                no_error = false;
                // If no declaration is found, report an undeclared symbol error.
                let entry = ast_table.get_entry(usage.ast()).unwrap();
                let (line, column) = entry.span().start_position();
                let content = format!(
                    "{} '{}' used but not declared at line {} column {}.",
                    usage.kind(),
                    symbol.name(),
                    line,
                    column
                );

                let error = ParsingError::new(
                    ParserErrorKind::ParseError,
                    Some(tree.filename().clone()),
                    line,
                    column,
                    content,
                );
                errors.add_error(error);
            }
        }
    }

    Ok(no_error)
}

/// Determines if a symbol should be skipped during the undeclared symbol check.
/// This decision is based on whether the symbol is predefined in PDDL (according to the
/// requirements) or if the symbol's kind matches any entry in the `skip_symbols` list.
///
/// # Arguments
///
/// * `symbol` - The symbol to check. This is typically a symbol from the symbol table that may
///   be used in the program or expression being analyzed.
/// * `ast_table` - The AST (Abstract Syntax Tree) table used to fetch relevant AST data,
///   including the problem's requirements (such as Typing, NumericFluents, etc.) that influence
///   whether a symbol is predefined in PDDL.
/// * `usage_kind` - The kind of symbol usage, which determines the context in which the symbol
///   is being used, e.g., a task, action, primitive type, etc.
/// * `skip_symbols` - A list of symbol kinds (e.g., `SymbolKind::Action`) that should be
///   skipped during the check.
///
/// # Returns
///
/// * `Result<bool, ParserInternalError>` - Returns `Ok(true)` if the symbol should be skipped
///   (either because it is a predefined PDDL symbol or its kind is in the `skip_symbols` list),
///   otherwise `Ok(false)`. If there is an error while checking if the symbol is a predefined
///   PDDL symbol, an `Err` is returned.
///
/// # Example
///
/// ```
/// let should_skip = should_skip_symbol(
///     &symbol,
///     &ast_table,
///     SymbolKind::Action,
///     &[SymbolKind::Action]
/// );
/// assert_eq!(should_skip, Ok(true));  // Assuming the symbol kind matches and is in the skip list.
/// ```
fn should_skip_symbol(
    symbol: &Symbol,
    ast_table: &AstTable,
    usage_kind: &SymbolKind,
    skip_symbols: &[SymbolKind],
) -> Result<bool, ParserInternalError> {
    // Skip if the symbol is predefined in PDDL or if it matches a symbol kind in the skip list.
    Ok(is_pddl_builtin_symbol(symbol, ast_table)? || skip_symbols.contains(usage_kind))
}

/// Checks if a declaration for the given symbol usage exists in the symbol's declarations.
/// This function handles different `SymbolKind`s and checks if the corresponding declaration
/// matches the usage within the given scope.
///
/// # Arguments
///
/// * `symbol` - The symbol to check for declaration.
/// * `usage` - The usage of the symbol that needs to be checked.
///
/// # Returns
///
/// * `bool` - Returns `true` if a matching declaration was found, otherwise `false`.
///
/// # Example
///
/// ```
/// let declaration_found = is_declaration_found(&symbol, &usage);
/// ```
fn is_declaration_found(symbol: &Symbol, usage: &Usage) -> bool {
    let usage_scope = usage.scope();

    // Common closure to check declarations for the given kind and scope
    // This closure checks if the declaration's scope starts with the usage scope and if the
    // declaration kind matches the usage kind.
    let check_declarations = |declaration: &Declaration| {
        usage_scope.starts_with(&declaration.scope()) && declaration.kind() == usage.kind()
    };

    // For PrimitiveType, we also check usages at the root scope
    // This is necessary because PrimitiveType can be used without declaration if it appears on
    // the right side of type declarations in PDDL.For example, types like "car" or "vehicle"
    // might not be explicitly declared but are understood in the domain context.
    let check_usages_at_root_scope = |usage: &Usage| {
        let root_scope = Scope::new(0, None);
        symbol
            .usages()
            .iter()
            .any(|u| usage.scope().starts_with(&root_scope) && u.kind() == usage.kind())
    };

    // For SymbolKind::Task, we also check if declaration.kind() is Action or Task.
    // This ensures we match tasks that are declared with Action or Task symbols.
    let check_primitive_task_declaration = |declaration: &Declaration| {
        usage_scope.starts_with(&declaration.scope())
            && (*declaration.kind() == SymbolKind::Action
                || *declaration.kind() == SymbolKind::Task)
    };

    match usage.kind() {
        SymbolKind::Task => symbol
            .declarations()
            .iter()
            .any(check_primitive_task_declaration),
        SymbolKind::PrimitiveType => {
            // For PrimitiveType, we check the common declaration logic and also include checks
            // or usages at the root scope. This ensures that PrimitiveTypes can be considered
            // even if they aren't explicitly declared in the current scope.
            symbol.declarations().iter().any(check_declarations)
                || symbol
                    .usages()
                    .iter()
                    .any(|u| check_usages_at_root_scope(u))
        }
        _ => symbol.declarations().iter().any(check_declarations),
    }
}
/// Checks if a symbol is a predefined PDDL symbol based on the requirements in the given
/// `ast_table`.
///
/// This function takes into account the context of the PDDL problem, i.e., which requirements
/// are enabled in the current problem (such as `Typing`, `Adl`, `NumericFluents`, and
/// `DurativeActions`), to determine if a symbol is considered a predefined symbol in PDDL.
///
/// # Arguments
/// - `symbol`: The symbol to check.
/// - `ast_table`: The abstract syntax tree table that holds information about the PDDL problem
///   requirements.
///
/// # Returns
/// - `Ok(true)` if the symbol is predefined and matches the requirements.
/// - `Ok(false)` if the symbol is not predefined.
/// - `Err(ParserInternalError)` if there is an internal error while checking the symbol.
///
/// # Examples
/// ```
/// let symbol = Symbol::new("object_type");
/// let result = is_pddl_builtin_symbol(&symbol, &ast_table);
/// assert_eq!(result, Ok(true));
/// ```
fn is_pddl_builtin_symbol(
    symbol: &Symbol,
    ast_table: &AstTable,
) -> Result<bool, ParserInternalError> {
    match symbol.name().as_str() {
        // 'object_type' is a predefined symbol when 'Typing' or 'Adl' requirements are present.
        OBJECT_TYPE
            if ast_table.requirements().contains(&Typing)
                || ast_table.requirements().contains(&Adl) =>
        {
            Ok(true)
        }

        // 'number_type' or 'total_time' are predefined when the 'NumericFluents' requirement is
        // present.
        NUMBER_TYPE | TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => Ok(true),

        // 'duration_variable' is predefined when the 'DurativeActions' requirement is present.
        DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => Ok(true),

        // Default case for any other symbols.
        _ => Ok(false),
    }
}
