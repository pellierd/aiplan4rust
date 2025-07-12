use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::Provider;
use crate::aiplan4rust::diagnostic::DiagnosticKind;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::lang::Requirement::{Adl, Fluents};
use crate::aiplan4rust::lang::Requirement::DurativeActions;
use crate::aiplan4rust::lang::Requirement::NumericFluents;
use crate::aiplan4rust::lang::Requirement::Typing;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::syntax::ast::AstKind;


/// Checks for symbols that are declared but never used within their scope or any parent scope,
/// emitting warnings for such unused declarations.
///
/// This function iterates over all symbol declarations in the annotated syntax arena and verifies
/// whether each declaration has at least one usage within its scope or any parent scope. It skips
/// checking for symbols of kinds specified in `skip_symbols` or those determined to be skipped by
/// domain-specific rules.
///
/// Built-in PDDL symbols like `"object"` and `"number"` are always ignored as they are considered
/// inherently valid.
///
/// # Parameters
/// - `ast_old`: A reference to the `AnnotatedSyntaxTree` containing the symbol table,
///   declarations, and usages.
/// - `skip_symbols`: A slice of `SymbolKind` indicating symbol kinds to exclude from the
///   unused-symbol check.
/// - `source`: The `DiagnosticSource` from which the diagnostic originates.
/// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` where warning diagnostics
///   will be recorded.
///
/// # Returns
/// - `Ok(true)` if the check completes successfully; warnings for unused symbols are recorded
///   through `diagnostic_manager`.
/// - `Err(ParserInternalError)` if any internal error occurs during processing, such as missing
///   AST entries.
///
/// # Notes
/// - Symbols declared as built-in PDDL types or those matching skip rules are not checked.
/// - For each unused symbol declaration found, a warning diagnostic is emitted.
///
/// # Example
/// ```no_run
/// let result = check_unused_symbols_warning(
///     &ast_old,
///     &[SymbolKind::Requirement],
///     source,
///     &mut diagnostic_manager,
/// );
/// if let Err(e) = result {
///     eprintln!("Error during unused symbol check: {:?}", e);
/// }
/// ```
pub fn check_unused_symbols(
    context: &CheckContext,
    skip_symbols: &[SymbolKind],
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let symbol_table = context.symbol_table();

    for symbol in symbol_table.values() {
        for declaration in symbol.declarations() {
            let declaration_kind = declaration.symbol_kind();

            // Skip declarations that should not be analyzed
            if skip_unused_symbol_declaration(declaration, context)?
                || skip_symbols.iter().any(|kind| *kind == declaration_kind)
            {
                continue;
            }

            // Check if declaration refers to a PDDL built-in and report if so
            check_pddl_builtin_symbol_declaration(
                declaration,
                context,
                source,
                diagnostic_manager
            );

            let declaration_scope = declaration.scope();

            // Determine if this declaration has at least one valid usage
            let has_valid_usage = symbol.usages().iter().any(|usage| {
                usage.scope().starts_with(&declaration_scope)
            });

            // If no usage is found, emit a warning
            if !has_valid_usage {
                report_unused_symbol_warning(
                    declaration,
                    context.source_name(),
                    source,
                    diagnostic_manager
                );
            }
        }
    }

    Ok(true)
}

/// Emits a warning diagnostic for an unused symbol declaration.
///
/// This function is triggered when a declaration exists in the code but is
/// never referenced or used in any valid scope. This may indicate dead or
/// redundant code that can be removed to improve clarity or efficiency.
///
/// # Parameters
/// - `declaration`: The specific declaration that is unused.
/// - `filename`: The name of the source file containing the declaration.
/// - `source`: The `DiagnosticSource` from which the diagnostic originates.
/// - `diagnostic_manager`: The manager responsible for collecting diagnostics.
///
fn report_unused_symbol_warning(
    declaration: &Declaration,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) {

    let warning = Diagnostic::new(
        DiagnosticKind::UnusedSymbolWarning {
            declaration: declaration.clone(),
        },
        source,
        filename.to_string(),
        declaration.span().clone(),
    );

    diagnostic_manager.add_diagnostic(warning);
}

/// Determines whether a declaration should be skipped during unused symbol checking.
///
/// This function returns `true` if the declaration represents a symbol kind or context
/// that should not be considered when checking for unused symbols. This includes:
/// - Declarations of built-in domain constructs like `Requirement`, `Action`, `DASymbol`, or
///   `Method`.
/// - Declarations of symbols like `DomainName`, `ProblemName`, or those associated with
///   specific PDDL requirements (e.g., `Typing`, `NumericFluents`, `DurativeActions`).
/// - Variables that appear within the scope of atomic formula skeletons, atomic function skeletons,
///   or task definitions—these are typically considered local and not subject to unused symbol
///   diagnostics.
///
/// # Parameters
/// - `declaration`: A reference to the `Declaration` to evaluate.
/// - `annotated_syntax_tree`: A reference to the `AnnotatedSyntaxTree`, used for contextual
///   analysis.
///
/// # Returns
/// - `Ok(true)` if the declaration should be skipped during unused symbol checking.
/// - `Ok(false)` if it should be checked.
/// - `Err(ParserInternalError)` if the required AST context cannot be retrieved.
///
/// # Note
/// Skipping these declarations avoids false positives when analyzing domain-level constructs or
/// locally scoped variables that are not meant to be globally referenced.
fn skip_unused_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
) -> Result<bool, ParserInternalError> {
    // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol, or Method
    if matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName
            | SymbolKind::ProblemName
            | SymbolKind::Requirement
            | SymbolKind::Action
            | SymbolKind::DASymbol
            | SymbolKind::Method
    ) {
        return Ok(true);
    }

    let requirements = context.requirements();
    match declaration.symbol_ident() {
        StringInterner::IDENT_OBJECT
            if requirements.contains(&Typing)
                || requirements.contains(&Adl) =>
        {
            return Ok(true)
        }
        StringInterner::IDENT_NUMBER | StringInterner::IDENT_TOTAL_TIME if requirements.contains(&NumericFluents) => {
            return Ok(true)
        }
        StringInterner::IDENT_DURATION_VARIABLE if requirements.contains(&DurativeActions) => {
            return Ok(true)
        }
        _ => {}
    }

    // Skip if the declaration is a variable and its scope contains an atomic skeleton node.
    // Variables within such scopes are typically local and do not need to be checked for duplicates.
    if matches!(declaration.symbol_kind(), SymbolKind::Variable)
        && (declaration
            .scope()
            .contains_node_of_kind(AstKind::AtomicFormulaSkeleton, context.ast())?
            || declaration.scope().contains_node_of_kind(
        AstKind::AtomicFunctionSkeleton,
        context.ast(),
            )?
            || declaration // Add for HDDL
                .scope()
                .contains_node_of_kind(AstKind::TaskDef, context.ast())?)
    {
        return Ok(true);
    }

    Ok(false)
}

/// Validates whether a symbol is correctly declared as a built-in symbol according to PDDL
/// specifications.
///
/// This function checks if the given declaration corresponds to a reserved built-in symbol
/// (such as `object`, `number`, `total-time`, or `?duration`) and verifies whether its
/// kind matches the expected `SymbolKind` based on the domain's declared PDDL requirements.
///
/// It emits an error diagnostic if the declaration uses an incorrect kind for a reserved
/// built-in symbol, and emits a warning diagnostic if the symbol is correctly declared but
/// its usage may cause ambiguity or confusion due to keyword overlap.
///
/// # Parameters
/// - `declaration`: Reference to the `Declaration` to validate.
/// - `ast_old`: Reference to the `AnnotatedSyntaxTree` providing requirements and
///   structural context needed for validation.
/// - `checker`: The `Checker` context associated with this validation, used as diagnostic source.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where diagnostics
///   (errors or warnings) will be recorded.
///
/// # Returns
/// - `true` if the declaration either matches a known built-in symbol with the correct kind,
///   or if it does not correspond to any recognized built-in symbol (no validation needed).
/// - `false` if the declaration matches a known built-in symbol but is declared with
///   an incorrect kind, in which case an error diagnostic is emitted.
///
/// # Diagnostics
/// - Emits an error if a reserved built-in symbol is declared with a wrong kind.
/// - Emits a warning if the symbol is correctly declared but might cause ambiguity due to
///   keyword overlap.
///
/// # Example
/// ```no_run
/// let result = check_pddl_builtin_symbol_declaration(
///     &declaration,
///     &ast_old,
///     checker,
///     &mut diagnostic_manager,
/// );
/// if !result {
///     eprintln!("Symbol declared with incorrect kind.");
/// }
/// ```
fn check_pddl_builtin_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    let requirements = context.requirements();
    let (expected_kind, requirements) = match declaration.symbol_ident() {
        StringInterner::IDENT_OBJECT
        if requirements.contains(&Typing) || requirements.contains(&Adl) =>
            {
                (SymbolKind::PrimitiveType, vec![Typing, Adl])
            }
        StringInterner::IDENT_NUMBER if requirements.contains(&NumericFluents) => (
            SymbolKind::PrimitiveType,
            vec![NumericFluents, Fluents],
        ),
        StringInterner::IDENT_TOTAL_TIME if requirements.contains(&NumericFluents) => {
            (SymbolKind::Function, vec![NumericFluents, Fluents])
        }
        StringInterner::IDENT_DURATION_VARIABLE if requirements.contains(&DurativeActions) => (
            SymbolKind::Variable,
            vec![DurativeActions],
        ),
        _ => return true,
    };

    if declaration.symbol_kind() != expected_kind {
        report_symbol_declared_as_keyword_error(
            declaration,
            expected_kind,
            requirements,
            context.source_name(),
            source,
            diagnostic_manager,
        );
        false
    } else {
        report_symbol_declared_ambiguous_as_keyword_warning(
            declaration,
            requirements,
            context.source_name(),
            source,
            diagnostic_manager,
        );
        true
    }
}

/// Reports a warning diagnostic indicating that a symbol has been declared in a way that
/// may cause ambiguity with reserved keywords.
///
/// This function creates and adds a `SymbolDeclaredAmbiguouslyAsKeywordWarning` diagnostic
/// to the provided `DiagnosticManager`. It is used when a symbol’s declaration might
/// conflict with reserved keywords, leading to potential ambiguity but not necessarily an error.
///
/// # Parameters
/// - `declaration`: Reference to the `Declaration` of the symbol that is ambiguously declared.
/// - `requirements`: A vector of `Requirement`s relevant to the ambiguous keyword context.
/// - `filename`: The name of the source file where the declaration occurs, used for warning
///   reporting.
/// - `source`: The `DiagnosticSource` from which the diagnostic originates.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where the warning
///   diagnostic will be recorded.
///
/// # Behavior
/// This function clones the declaration and constructs a `SymbolDeclaredAmbiguouslyAsKeywordWarning`
/// diagnostic capturing details about the ambiguous declaration. It then registers this diagnostic
/// with the given `diagnostic_manager`.
///
/// # Examples
/// ```no_run
/// report_symbol_declared_ambiguous_as_keyword_warning(
///     &declaration,
///     vec![Requirement::Typing],
///     "domain.pddl",
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn report_symbol_declared_ambiguous_as_keyword_warning(
    declaration: &Declaration,
    requirements: Vec<Requirement>,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) {
    diagnostic_manager.add_diagnostic(
        Diagnostic::new(
            DiagnosticKind::SymbolDeclaredAmbiguouslyAsKeywordWarning {
                declaration: declaration.clone(),
                requirements,
            },
            source,
            filename.to_string(),
            declaration.span().clone(),
        )
    );
}

/// Reports an error diagnostic indicating that a symbol has been incorrectly declared
/// as a reserved keyword.
///
/// This function creates and adds a `SymbolDeclaredAsKeywordError` diagnostic to the provided
/// `DiagnosticManager`. It is used when a symbol declaration conflicts with reserved keywords
/// defined by the language or domain specification, typically due to an incorrect kind or misuse.
///
/// # Parameters
/// - `declaration`: Reference to the `Declaration` of the symbol that was improperly declared.
/// - `expected_kind`: The expected `SymbolKind` that the symbol should have had to avoid this error.
/// - `requirements`: A vector of `Requirement`s relevant to the reserved keyword context.
/// - `filename`: The name of the source file where the declaration occurs, used for error reporting.
/// - `source`: The `DiagnosticSource` from which the diagnostic originates.
/// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` where the error diagnostic
///   will be recorded.
///
/// # Behavior
/// This function clones the declaration and constructs a `SymbolDeclaredAsKeywordError` diagnostic
/// that captures details about the incorrect declaration. It then registers this diagnostic
/// with the given `diagnostic_manager`.
///
/// # Examples
/// ```no_run
/// report_symbol_declared_as_keyword_error(
///     &declaration,
///     SymbolKind::PrimitiveType,
///     vec![Requirement::Typing],
///     "domain.pddl",
///     source,
///     &mut diagnostic_manager,
/// );
/// ```
fn report_symbol_declared_as_keyword_error(
    declaration: &Declaration,
    expected_kind: SymbolKind,
    requirements: Vec<Requirement>,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) {
    diagnostic_manager.add_diagnostic(Diagnostic::new(
        DiagnosticKind::SymbolDeclaredAsKeywordError {
            declaration: declaration.clone(),
            expected_kind,
            requirements,
        },
        source,
        filename.to_string(),
        declaration.span().clone(),
    ));
}
