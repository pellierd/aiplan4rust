use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement::Adl;
use crate::aiplan4rust::parser::elements::Requirement::DurativeActions;
use crate::aiplan4rust::parser::elements::Requirement::NumericFluents;
use crate::aiplan4rust::parser::elements::Requirement::Typing;
use crate::aiplan4rust::parser::lexer::token::DURATION_VARIABLE;
use crate::aiplan4rust::parser::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::parser::lexer::token::OBJECT_TYPE;
use crate::aiplan4rust::parser::lexer::token::TOTAL_TIME;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::Symbol;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::symbol::Usage;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

/// Checks if there are any undeclared symbols used in the given symbol table.
/// This function scans all usages of symbols in the `symbol_table` and verifies if
/// each symbol has been declared correctly. If no declaration is found for a symbol,
/// an error is generated and added to the error manager. The function also respects
/// a list of symbols to skip during the check (e.g., certain symbol kinds or those
/// that are exempt from declaration checks).
///
/// # Arguments
///
/// * `annotated_syntax_tree` - A reference to the `AnnotatedSyntaxTree` that contains
///   the symbol table and syntax tree to verify symbol usages.
/// * `skip_symbols` - A list of `SymbolKind`s to skip during the check, allowing for
///   exemptions where certain symbols should not trigger errors.
/// * `errors` - A mutable reference to the `ErrorManager` where any found errors will
///   be recorded.
///
/// # Returns
///
/// This function returns:
/// - `Ok(true)` if no undeclared symbols were found.
/// - `Ok(false)` if undeclared symbols were found, but no errors occurred during execution.
/// - `Err(ParserInternalError)` if an internal error occurs during the check process.
///
/// # Example
///
/// ```rust
/// let result = check(&annotated_syntax_tree, &[SymbolKind::Action], &mut error_manager);
/// match result {
///     Ok(true) => println!("No undeclared symbols found."),
///     Ok(false) => println!("Some undeclared symbols were found."),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```

pub fn check(
    syntax_tree: &AnnotatedSyntaxTree,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    let symbol_table = syntax_tree.symbol_table();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        // Iterate over all usages of the symbol.
        for usage in symbol.usages() {
            // Skip the symbol if it meets the criteria (e.g., already declared or needs to be skipped).
            if should_skip_symbol(symbol, syntax_tree, usage.kind(), skip_symbols)? {
                continue;
            }

            // Check if the declaration for the symbol was found.
            if !is_declaration_found(symbol, usage) {
                no_error = false;
                // If no declaration is found, report an undeclared symbol error.
                let entry = syntax_tree.get_entry(usage.ast()).unwrap();
                let error = Diagnostic::new(
                    DiagnosticKind::UndeclaredSymbol {
                        symbol: symbol.name().clone(),
                        kind: usage.kind().clone(),
                    },
                    DiagnosticSource::SemanticAnalyzer,
                    syntax_tree.filename().clone(),
                    entry.span().clone(),
                );
                diagnostic_manager.add_diagnostic(error);
            }
        }
    }

    Ok(no_error)
}

/// Determines if a symbol should be skipped during the undeclared symbol check.
/// This decision is based on whether the symbol is predefined in PDDL (according to the
/// requirements) or if the symbol's kind matches any entry in the `skip_symbols` list.
///
/// The function checks if the symbol is one of the predefined symbols in the Planning Domain
/// Definition Language (PDDL) or if the symbol’s usage kind matches an entry in the provided
/// `skip_symbols` list. The function also takes into account the requirements of the problem
/// as specified in the `annotated_syntax_tree` (such as whether Typing or NumericFluents are
/// required).
///
/// # Arguments
///
/// * `symbol` - The symbol to check. This is typically a symbol from the symbol table that may
///   be used in the program or expression being analyzed.
/// * `annotated_syntax_tree` - A reference to the `AnnotatedSyntaxTree` that provides access to
///   the problem’s requirements and other metadata affecting symbol definitions.
/// * `usage_kind` - The kind of symbol usage, which determines the context in which the symbol
///   is being used, such as a task, action, or primitive type.
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
/// ```rust
/// let should_skip = should_skip_symbol(
///     &symbol,
///     &annotated_syntax_tree,
///     SymbolKind::Action,
///     &[SymbolKind::Action]
/// );
/// assert_eq!(should_skip, Ok(true));  // Assuming the symbol kind matches and is in the skip list.
/// ```

fn should_skip_symbol(
    symbol: &Symbol,
    annotated_syntax_tree: &AnnotatedSyntaxTree,
    usage_kind: &SymbolKind,
    skip_symbols: &[SymbolKind],
) -> Result<bool, ParserInternalError> {
    // Skip if the symbol is predefined in PDDL or if it matches a symbol kind in the skip list.
    Ok(is_pddl_builtin_symbol(symbol, annotated_syntax_tree)? || skip_symbols.contains(usage_kind))
}

/// Checks if a declaration for the given symbol usage exists in the symbol's declarations.
/// This function handles different `SymbolKind`s and checks if the corresponding declaration
/// matches the usage within the given scope.
///
/// The function verifies whether a declaration for the symbol exists that matches the scope
/// and kind of the given usage. Special handling is provided for certain kinds of symbols,
/// such as `PrimitiveType` and `Task`, to allow for cases where the declaration may be implicit
/// or when certain symbols (like tasks or primitive types) can be used in special ways in PDDL.
///
/// # Arguments
///
/// * `symbol` - The symbol to check for declaration. This is the symbol that has potential
///   declarations and usages in the symbol table.
/// * `usage` - The usage of the symbol that needs to be checked. This indicates the context
///   in which the symbol is being used, including the scope and kind of usage.
///
/// # Returns
///
/// * `bool` - Returns `true` if a matching declaration was found for the symbol usage,
///   otherwise returns `false`.
///
/// # Example
///
/// ```rust
/// let declaration_found = is_declaration_found(&symbol, &usage);
/// assert_eq!(declaration_found, true);  // Assuming a matching declaration was found.
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
/// `annotated_syntax_tree`.
///
/// This function determines whether a symbol is predefined in PDDL, depending on the current
/// PDDL problem's enabled requirements (e.g., `Typing`, `Adl`, `NumericFluents`, and
/// `DurativeActions`). It checks the symbol's name against known predefined symbols that are
/// activated by these requirements.
///
/// # Arguments
/// - `symbol`: The symbol to check. This symbol will be matched against predefined PDDL symbols.
/// - `annotated_syntax_tree`: The annotated syntax tree, which contains information about the
///   enabled requirements in the PDDL problem. This is used to determine if a given symbol
///   is predefined based on the current problem's requirements.
///
/// # Returns
/// - `Ok(true)` if the symbol is predefined and matches the enabled requirements.
/// - `Ok(false)` if the symbol is not predefined based on the requirements.
/// - `Err(ParserInternalError)` if there is an internal error while checking the symbol.
///
/// # Examples
/// ```
/// let symbol = Symbol::new("object_type");
/// let result = is_pddl_builtin_symbol(&symbol, &ast_table);
/// assert_eq!(result, Ok(true));  // Assuming 'Typing' or 'Adl' requirements are enabled.
/// ```
///
/// # Predefined Symbols Based on Requirements
/// - `object_type`: Predefined when the `Typing` or `Adl` requirements are enabled.
/// - `number_type` and `total_time`: Predefined when the `NumericFluents` requirement is enabled.
/// - `duration_variable`: Predefined when the `DurativeActions` requirement is enabled.
fn is_pddl_builtin_symbol(
    symbol: &Symbol,
    annotated_syntax_tree: &AnnotatedSyntaxTree,
) -> Result<bool, ParserInternalError> {
    match symbol.name().as_str() {
        // 'object_type' is a predefined symbol when 'Typing' or 'Adl' requirements are present.
        OBJECT_TYPE
            if annotated_syntax_tree.has_requirement(&Typing)
                || annotated_syntax_tree.has_requirement(&Adl) =>
        {
            Ok(true)
        }

        // 'number_type' or 'total_time' are predefined when the 'NumericFluents' requirement is
        // present.
        NUMBER_TYPE | TOTAL_TIME if annotated_syntax_tree.has_requirement(&NumericFluents) => {
            Ok(true)
        }

        // 'duration_variable' is predefined when the 'DurativeActions' requirement is present.
        DURATION_VARIABLE if annotated_syntax_tree.has_requirement(&DurativeActions) => Ok(true),

        // Default case for any other symbols.
        _ => Ok(false),
    }
}
