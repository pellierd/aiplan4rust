use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement::{Adl, Fluents};
use crate::aiplan4rust::parser::elements::Requirement::DurativeActions;
use crate::aiplan4rust::parser::elements::Requirement::NumericFluents;
use crate::aiplan4rust::parser::elements::Requirement::Typing;
use crate::aiplan4rust::parser::lexer::token::DURATION_VARIABLE;
use crate::aiplan4rust::parser::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::parser::lexer::token::OBJECT_TYPE;
use crate::aiplan4rust::parser::lexer::token::TOTAL_TIME;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Symbol;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

/// Checks for symbols that are declared but never used in the same or a parent scope.
/// This function reports warnings for any unused symbols found.
///
/// # Parameters
/// - `annotated_syntax_tree`: A reference to the annotated syntax tree, containing the symbol table and syntax tree.
/// - `skip_symbols`: A list of `SymbolKind` values representing symbols that should be ignored during checking.
/// - `errors`: A mutable reference to the `ErrorManager` where warnings and errors will be logged.
///
/// # Returns
/// - `Ok(())` if the check completes successfully. Warnings are logged through the error manager.
/// - `Err(ParserInternalError)` if an error occurs during processing.
///
/// # Note
/// - The built-in PDDL symbols `"object"` and `"number"` are ignored, as they are always valid.
/// - Symbols whose kind appears in `skip_symbols` are not checked.
pub fn check_unused_symbols(
    syntax_tree: &AnnotatedSyntaxTree,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let no_error = true;

    let symbol_table = syntax_tree.symbol_table();

    for symbol in symbol_table.values() {
        for declaration in symbol.declarations() {
            let declaration_kind = declaration.kind().clone();

            if skip_unused_symbol_declaration(symbol, declaration, syntax_tree)?
                || skip_symbols.contains(&declaration_kind)
            {
                continue;
            }

            check_pddl_builtin_symbol_declaration(symbol, declaration, syntax_tree, diagnostic_manager)?;

            let declaration_scope = declaration.scope();

            // Vérifie s'il y a au moins un usage valide
            let has_valid_usage = symbol
                .usages()
                .iter()
                .any(|usage| {
                    usage.scope().starts_with(&declaration_scope)
                });

            if !has_valid_usage {
                let entry = syntax_tree.get_entry(declaration.ast()).unwrap();
                let warning = Diagnostic::new(
                    DiagnosticKind::UnusedSymbol {
                        symbol: symbol.name().clone(),
                        kind: declaration_kind.clone(),
                    },
                    DiagnosticSource::SemanticAnalyzer,
                    syntax_tree.filename().clone(),
                    entry.span().clone(),
                );
                diagnostic_manager.add_diagnostic(warning);
            }
        }
    }

    Ok(no_error)
}


/// Determines whether a declaration should be skipped during duplicate checking.
///
/// This function returns `true` if the declaration's kind indicates that it is not
/// subject to duplicate checks. Specifically, it skips declarations of symbols of kind
/// `Requirement`, `Action`, `DASymbol`, or `Method`, as well as variables declared within the scope
/// of atomic formula or atomic function skeletons. Such symbols are typically declared
/// in the domain and are not intended to be checked for duplicates in problem files.
///
/// # Parameters
/// - `symbol`: A reference to the `Symbol` that owns the declaration to check.
/// - `declaration`: A reference to the `Declaration` to check.
/// - `annotated_syntax_tree`: A reference to the `AnnotatedSyntaxTree`, which contains the necessary
///   information for checking the declaration.
///
/// # Returns
/// - `true` if the declaration should be skipped (i.e., it is of a type that does not require
///   duplicate checking).
/// - `false` otherwise, indicating that the declaration should be checked for duplicates.
///
/// # Note
/// - Declarations of symbols with kinds `Requirement`, `Action`, `DASymbol`, or `Method` are skipped
///   because these are typically domain-level constructs that don't require duplicate checking in the problem file.
/// - Variables declared within atomic formula or function skeletons, or within task definitions, are also skipped
///   because they are considered local and don't need to be checked for duplicates.
fn skip_unused_symbol_declaration(
    symbol: &Symbol,
    declaration: &Declaration,
    annotated_syntax_tree: &AnnotatedSyntaxTree,
) -> Result<bool, ParserInternalError> {
    // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol, or Method
    if matches!(
        declaration.kind(),
        SymbolKind::DomainName
            | SymbolKind::ProblemName
            | SymbolKind::Requirement
            | SymbolKind::Action
            | SymbolKind::DASymbol
            | SymbolKind::Method
    ) {
        return Ok(true);
    }

    match symbol.name().as_str() {
        OBJECT_TYPE
            if annotated_syntax_tree.has_requirement(&Typing)
                || annotated_syntax_tree.has_requirement(&Adl) =>
        {
            return Ok(true)
        }
        NUMBER_TYPE | TOTAL_TIME if annotated_syntax_tree.has_requirement(&NumericFluents) => {
            return Ok(true)
        }
        DURATION_VARIABLE if annotated_syntax_tree.has_requirement(&DurativeActions) => {
            return Ok(true)
        }
        _ => {}
    }

    // Skip if the declaration is a variable and its scope contains an atomic skeleton node.
    // Variables within such scopes are typically local and do not need to be checked for duplicates.
    if matches!(declaration.kind(), SymbolKind::Variable)
        && (declaration
            .scope()
            .contains_ast_of_kind(SyntaxNodeKind::AtomicFormulaSkeleton, annotated_syntax_tree)?
            || declaration.scope().contains_ast_of_kind(
                SyntaxNodeKind::AtomicFunctionSkeleton,
                annotated_syntax_tree,
            )?
            || declaration // Add for HDDL
                .scope()
                .contains_ast_of_kind(SyntaxNodeKind::TaskDef, annotated_syntax_tree)?)
    {
        return Ok(true);
    }

    Ok(false)
}

/// Checks if a symbol is properly declared as a built-in symbol according to the PDDL
/// specifications.
///
/// This function verifies whether a given symbol matches the expected type of a built-in symbol
/// based on the PDDL requirements present in the `ast_table`. For example, it checks if a
/// symbol like `OBJECT_TYPE`, `NUMBER_TYPE`, or `TOTAL_TIME` is correctly declared with the
/// appropriate kind (e.g., `PrimitiveType`, `Function`, `Variable`) based on the domain's
/// requirements.
///
/// # Arguments
/// * `symbol`: A reference to the `Symbol` that needs to be checked.
/// * `declaration`: A reference to the `Declaration` of the symbol, which contains type
///   information.
/// * `annotated_syntax_tree`: A reference to the `AnnotatedSyntaxTree`, which contains the
///   domain's requirements and other metadata.
/// * `errors`: A mutable reference to the `ErrorManager`, which will log any errors encountered.
///
/// # Returns
/// * `Ok(true)` if the symbol's declaration matches the expected type and is correct according
///   to the PDDL requirements.
/// * `Ok(false)` if the symbol's declaration is incorrect or doesn't match any recognized
///   built-in symbol declaration.
///
/// # Errors
/// If the symbol's declaration is invalid, an error is logged with the line and column number
/// of the invalid declaration.
fn check_pddl_builtin_symbol_declaration(
    symbol: &Symbol,
    declaration: &Declaration,
    syntax_tree: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    // Match the symbol name with the expected built-in symbols and requirements
    let (expected_kind, requirements) = match symbol.name().as_str() {
        OBJECT_TYPE
            if syntax_tree.has_requirement(&Typing) || syntax_tree.has_requirement(&Adl) =>
        {
            (SymbolKind::PrimitiveType, vec![Typing, Adl])
        }
        NUMBER_TYPE if syntax_tree.has_requirement(&NumericFluents) => (
            SymbolKind::PrimitiveType,
            vec![NumericFluents, Fluents]
        ),
        TOTAL_TIME if syntax_tree.has_requirement(&NumericFluents) => {
            (SymbolKind::Function, vec![NumericFluents, Fluents])
        }
        DURATION_VARIABLE if syntax_tree.has_requirement(&DurativeActions) => (
            SymbolKind::Variable,
            vec![DurativeActions]
        ),
        _ => return Ok(true),
    };

    let entry = syntax_tree.get_entry(declaration.ast())
        .ok_or(ParserInternalError::new("Entry not found".to_string()))?;

    // Verify if the symbol has the correct type
    if *declaration.kind() != expected_kind {
       let error = Diagnostic::new(
           DiagnosticKind::ReservedSymbolUsedAs {
                    symbol: symbol.name().clone(),
                    actual_kind: declaration.kind().clone(),
                    expected_kind: expected_kind.clone(),
                    requirements
                },
                DiagnosticSource::SemanticAnalyzer,
                syntax_tree.filename().clone(),
                entry.span().clone(),
            );
            diagnostic_manager.add_diagnostic(error);
            return Ok(false);
    } else {
        let warning = Diagnostic::new(
            DiagnosticKind::AmbiguousSymbolUsageWithKeyword {
                symbol: symbol.name().clone(),
                actual_kind: declaration.kind().clone(),
                requirements
            },
            DiagnosticSource::SemanticAnalyzer,
            syntax_tree.filename().clone(),
            entry.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);

        Ok(true)
    }
}
