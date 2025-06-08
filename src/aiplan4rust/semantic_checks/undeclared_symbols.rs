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
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::Symbol;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::symbol::Usage;
use crate::aiplan4rust::semantic_analyser::{AnnotatedSyntaxNode, AnnotatedSyntaxTree, SymbolTable};
use crate::aiplan4rust::semantic_analyser::checkers::TypeChecker;
use crate::aiplan4rust::semantic_checks::checker_context::CheckerContext;

/// Checks if there are any undeclared symbols used in the given syntax tree.
///
/// This function scans all symbol usages in the provided `AnnotatedSyntaxTree`
/// and verifies that each symbol has a corresponding declaration. If a symbol
/// usage is found without any valid declaration, an `UndeclaredSymbol` diagnostic
/// is generated and recorded in the `DiagnosticManager`.
///
/// The check skips any symbol kinds listed in the `skip_symbols` array (e.g.,
/// symbols that are intentionally undeclared, like domain/problem names).
///
/// # Arguments
///
/// * `syntax_tree` - The `AnnotatedSyntaxTree` containing the symbol table and AST.
/// * `skip_symbols` - A list of `SymbolKind`s to ignore during the undeclared check.
/// * `diagnostic_manager` - A mutable reference to the `DiagnosticManager` that will store any
///   diagnostics produced.
/// * `context` - A `CheckerContext` indicating which phase of analysis is performing the check
///   (e.g., `SemanticAnalyzer`, `Linker`).
///
/// # Returns
///
/// - `Ok(true)` if no undeclared symbols were found.
/// - `Ok(false)` if some undeclared symbols were detected, but no internal error occurred.
/// - `Err(ParserInternalError)` if an internal error occurred during the process (e.g., missing
///   AST entry).
///
/// # Example
///
/// ```rust
/// let result = check_undeclared_symbols(
///     &annotated_syntax_tree,
///     &[SymbolKind::ProblemName, SymbolKind::DomainName],
///     &mut diagnostic_manager,
///     CheckerContext::SemanticAnalyzer,
/// );
///
/// match result {
///     Ok(true) => println!("No undeclared symbols."),
///     Ok(false) => println!("Undeclared symbols found."),
///     Err(e) => eprintln!("Internal error: {}", e),
/// }
/// ```
pub fn check_undeclared_symbols(
    syntax_tree: &AnnotatedSyntaxTree,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
    context: CheckerContext,
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
                report_undeclared_symbol_error(
                    diagnostic_manager,
                    symbol.name(),
                    usage.kind().clone(),
                    syntax_tree,
                    usage.ast(),
                    context,
                )?;
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

/// Reports an error diagnostic for an undeclared symbol usage.
///
/// This function creates and adds a diagnostic error indicating that a symbol
/// was used in the code without a corresponding declaration. It retrieves the
/// AST entry associated with the usage to obtain source code location information
/// (span) for accurate error reporting.
///
/// # Parameters
///
/// - `diagnostic_manager`: Mutable reference to the diagnostic manager where the error will be
///   recorded.
/// - `symbol_name`: The name of the symbol that was used but not declared.
/// - `kind`: The kind of the symbol (e.g., variable, function) that is undeclared.
/// - `syntax_tree`: Reference to the annotated syntax tree containing the AST entries.
/// - `usage_ast_id`: The AST node ID corresponding to the symbol usage.
/// - `context`: The context of the checker, used to indicate the source of the diagnostic.
///
/// # Returns
///
/// Returns `Ok(())` if the diagnostic was successfully added.
/// Returns `Err(ParserInternalError)` if the AST entry for the usage could not be found,
/// indicating an internal inconsistency.
///
/// # Errors
///
/// This function returns an error if the AST entry corresponding to `usage_ast_id`
/// is missing from the syntax tree. This typically indicates a serious internal error
/// in the parsing or symbol resolution process.
///
/// # Example
///
/// ```rust
/// report_undeclared_symbol_error(
///     &mut diagnostic_manager,
///     &symbol.name(),
///     usage.kind().clone(),
///     &syntax_tree,
///     usage.ast(),
///     context,
/// )?;
/// ```
fn report_undeclared_symbol_error(
    diagnostic_manager: &mut DiagnosticManager,
    symbol_name: &String,
    kind: SymbolKind,
    syntax_tree: &AnnotatedSyntaxTree,
    usage_ast_id: usize,
    context: CheckerContext,
) -> Result<(), ParserInternalError> {
    let entry = syntax_tree.get_entry(usage_ast_id).ok_or_else(|| {
        ParserInternalError::new(format!(
            "Missing AST entry for usage with id: {}",
            usage_ast_id
        ))
    })?;

    let error = Diagnostic::new(
        DiagnosticKind::UndeclaredSymbolError {
            symbol: symbol_name.clone(),
            kind,
        },
        context.into(),
        syntax_tree.filename().clone(),
        entry.span().clone(),
    );

    diagnostic_manager.add_diagnostic(error);
    Ok(())
}

////////////////////////////////////////////////////////////////////////////////////////////////////

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
