use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Requirement::Adl;
use crate::aiplan4rust::syntax::elements::Requirement::DurativeActions;
use crate::aiplan4rust::syntax::elements::Requirement::NumericFluents;
use crate::aiplan4rust::syntax::elements::Requirement::Typing;
use crate::aiplan4rust::syntax::lexer::token::DURATION_VARIABLE;
use crate::aiplan4rust::syntax::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::syntax::lexer::token::OBJECT_TYPE;
use crate::aiplan4rust::syntax::lexer::token::TOTAL_TIME;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::hir::HirTree;

/// Checks if there are any undeclared symbols used in the given syntax tree.
///
/// This function scans all symbol usages in the provided `AnnotatedSyntaxTree`
/// and verifies that each usage has at least one valid declaration. If any usage
/// lacks a corresponding declaration, an `UndeclaredSymbolError` diagnostic
/// is generated and recorded in the `DiagnosticManager`.
///
/// Symbol kinds listed in the `skip_symbols` slice (e.g., intentionally undeclared
/// symbols like domain or problem names) are ignored.
///
/// # Parameters
/// - `ast`: Reference to the `AnnotatedSyntaxTree` containing symbols and AST.
/// - `skip_symbols`: Slice of `SymbolKind` to be ignored during undeclared symbol checking.
/// - `source`: The `DiagnosticSource` indicating the analysis phase performing the check.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where diagnostics are stored.
///
/// # Returns
/// - `Ok(true)` if no undeclared symbols were found.
/// - `Ok(false)` if undeclared symbols were detected (no internal errors occurred).
/// - `Err(ParserInternalError)` if an internal error occurs during checking.
///
/// # Example
/// ```rust
/// let result = check_undeclared_symbols(
///     &annotated_syntax_tree,
///     &[SymbolKind::ProblemName, SymbolKind::DomainName],
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
///
/// if let Ok(true) = result {
///     println!("No undeclared symbols found.");
/// } else {
///     println!("Undeclared symbols detected.");
/// }
/// ```
pub fn check_undeclared_symbols(
    syntax_tree: &HirTree,
    skip_symbols: &[SymbolKind],
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    let symbol_table = syntax_tree.symbol_table();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        // Iterate over all usages of the symbol.
        for usage in symbol.usages() {
            // Skip the symbol if it meets the criteria (e.g., already declared or needs to be skipped).
            if should_skip_symbol(symbol, syntax_tree, usage.kind(), skip_symbols) {
                continue;
            }

            // Check if the declaration for the symbol was found.
            if !is_declaration_found(symbol, usage) {
                checked = false;
                report_undeclared_symbol_error(
                    usage,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                );
            }
        }
    }

    Ok(checked)
}

/// Determines if a symbol should be skipped during the undeclared symbol check.
///
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
/// * `true` if the symbol should be skipped (either because it is a predefined PDDL symbol
///   or its kind is in the `skip_symbols` list), otherwise `false`.
///
/// # Example
///
/// ```rust
/// let should_skip = should_skip_symbol(
///     &symbol,
///     &annotated_syntax_tree,
///     &SymbolKind::Action,
///     &[SymbolKind::Action],
/// );
/// assert_eq!(should_skip, true);  // Assuming the symbol kind matches and is in the skip list.
/// ```
fn should_skip_symbol(
    symbol: &Symbol,
    annotated_syntax_tree: &HirTree,
    usage_kind: &SymbolKind,
    skip_symbols: &[SymbolKind],
) -> bool {
    // Skip if the symbol is predefined in PDDL or if it matches a symbol kind in the skip list.
    is_pddl_builtin_symbol(symbol, annotated_syntax_tree) || skip_symbols.contains(usage_kind)
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

    // For PrimitiveType, we also check usages at the root scope.
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
/// - `true` if the symbol is predefined and matches the enabled requirements.
/// - `false` if the symbol is not predefined based on the requirements.
///
/// # Examples
/// ```
/// let symbol = Symbol::new("object_type");
/// let result = is_pddl_builtin_symbol(&symbol, &ast_table);
/// assert_eq!(result, true);  // Assuming 'Typing' or 'Adl' requirements are enabled.
/// ```
///
/// # Predefined Symbols Based on Requirements
/// - `object_type`: Predefined when the `Typing` or `Adl` requirements are enabled.
/// - `number_type` and `total_time`: Predefined when the `NumericFluents` requirement is enabled.
/// - `duration_variable`: Predefined when the `DurativeActions` requirement is enabled.

fn is_pddl_builtin_symbol(
    symbol: &Symbol,
    annotated_syntax_tree: &HirTree,
) -> bool {
    match symbol.name().as_str() {
        // 'object_type' is a predefined symbol when 'Typing' or 'Adl' requirements are present.
        OBJECT_TYPE
            if annotated_syntax_tree.has_requirement(&Typing)
                || annotated_syntax_tree.has_requirement(&Adl) =>
        {
            true
        }

        // 'number_type' or 'total_time' are predefined when the 'NumericFluents' requirement is
        // present.
        NUMBER_TYPE | TOTAL_TIME if annotated_syntax_tree.has_requirement(&NumericFluents) => {
            true
        }

        // 'duration_variable' is predefined when the 'DurativeActions' requirement is present.
        DURATION_VARIABLE if annotated_syntax_tree.has_requirement(&DurativeActions) => true,

        // Default case for any other symbols.
        _ => false,
    }
}

/// Reports an error diagnostic for the use of an undeclared symbol.
///
/// This function is invoked when a symbol is referenced in the source code but has
/// not been previously declared within the appropriate scope. It constructs and
/// registers a diagnostic of kind [`Kind::UndeclaredSymbolError`], using
/// metadata extracted from the provided `Usage` object.
///
/// The diagnostic includes the symbol name, kind, and source location (via the usage’s `span`).
/// It uses the provided `filename` to indicate the source file where the error occurred.
/// The `source` parameter identifies the analysis phase (e.g., syntax, semantic analyzer)
/// responsible for detecting the undeclared symbol.
///
/// # Parameters
/// - `usage`: The symbol usage instance referring to the undeclared symbol.
/// - `filename`: The name of the source file where the usage occurs.
/// - `source`: The analysis phase responsible for the error (e.g., syntax, semantic analyzer).
/// - `diagnostic_manager`: The diagnostic system to which the error will be added.
///
/// # Returns
/// This function does not return a `Result` because it does not fail under normal conditions.
///
/// # Example
/// ```rust
/// report_undeclared_symbol_error(
///     &usage,
///     "source_file.pddl",
///     DiagnosticSource::SemanticAnalyzer,
///     &mut diagnostic_manager,
/// );
/// ```
///
/// # See Also
/// - [`Usage`]: Holds symbol name, kind, and span information.
/// - [`Kind::UndeclaredSymbolError`]: The diagnostic kind generated.
/// - [`Provider`]: Identifies the analysis phase that produced the /// - [`Kind::UndeclaredSymbolError`]: The diagnostic kind generated..
fn report_undeclared_symbol_error(
    usage: &Usage,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) {
    let error = Diagnostic::new(
        DiagnosticKind::UndeclaredSymbolError {
            usage: usage.clone(),
        },
        source,
        filename.to_string(),
        usage.span().clone(),
    );
    diagnostic_manager.add_diagnostic(error);
}
