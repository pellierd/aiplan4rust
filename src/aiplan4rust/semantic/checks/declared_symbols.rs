use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::Scope;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::SemanticContext;

use std::collections::{HashMap, HashSet};


pub fn check_declared_symbols(
    context: &SemanticContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    check_symbol_declarations(context, diagnostic_manager, None)
}

pub fn check_declared_symbols_of_kinds(
    context: &SemanticContext,
    diagnostic_manager: &mut DiagnosticManager,
    kinds_to_check: Vec<SymbolKind>,
) -> Result<bool, ParserInternalError> {
    let kinds_set: HashSet<SymbolKind> = kinds_to_check.into_iter().collect();
    check_symbol_declarations(context, diagnostic_manager, Some(&kinds_set))
}

/// Checks for duplicate symbol declarations in the symbol table and logs errors
/// to the error manager if duplicates are found.
///
/// Duplicate checking is skipped for symbols of kind `DomainName` and
/// `ProblemName` because these kinds are often used as symbols for types or
/// predicates, where duplicates may be allowed.
///
/// # Parameters
/// - `symbol_table`: A reference to the symbol table to check for duplicates.
///
/// # Returns
/// - `Ok(())` if the check completes (errors are logged via the error manager).
/// - `Err(ParserInternalError)` if an error occurs during processing.
fn check_symbol_declarations(
    context: &SemanticContext,
    diagnostic_manager: &mut DiagnosticManager,
    kinds_to_check: Option<&HashSet<SymbolKind>>,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;
    let symbol_table = context.symbol_table();

    for symbol in symbol_table.values() {

        let mut seen_scopes: HashMap<Scope, Declaration> = HashMap::new();

        for declaration in symbol.declarations() {
            if let Some(kinds) = kinds_to_check {
                if !kinds.contains(declaration.kind()) {
                    continue;
                }
            }

            if skip_duplicated_declaration(declaration)? {
                continue;
            }

            let ast_entry = context.get(declaration.ast()).unwrap();
            let current_scope = declaration.scope();

            let maybe_conflict = seen_scopes.iter().find(|(s, _)| current_scope.starts_with(s));

            if let Some((conflicting_scope, previous_declaration)) = maybe_conflict {
                let current_kind = declaration.kind();
                let previous_kind = previous_declaration.kind();

                if (current_kind == &SymbolKind::PrimitiveType && previous_kind == &SymbolKind::Predicate) ||
                    (current_kind == &SymbolKind::Predicate && previous_kind == &SymbolKind::PrimitiveType) {
                    let warning = Diagnostic::new(
                        DiagnosticKind::WarningAmbiguousTypePredicateSymbol {
                            symbol: symbol.name().clone(),
                        },
                        Provider::Analyzer,
                        context.source_name().clone(),
                        ast_entry.span().clone(),
                    );
                    diagnostic_manager.add_diagnostic(warning);
                } else {
                    checked = false;

                    let scope_index = conflicting_scope.iter().last().unwrap();
                    let scope = context.get(*scope_index).unwrap();

                    let error = Diagnostic::new(
                        DiagnosticKind::DuplicatedSymbolDeclarationInScopeError {
                            symbol: symbol.name().clone(),
                            declaration1: previous_declaration.clone(),
                            declaration2: declaration.clone(),
                            scope: scope.clone(),
                        },
                        Provider::Analyzer,
                        context.source_name().to_string(),
                        ast_entry.span().clone(),
                    );
                    diagnostic_manager.add_diagnostic(error);
                }
            } else {
                seen_scopes.insert(current_scope.clone(), declaration.clone());
            }
        }
    }

    Ok(checked)
}

fn skip_duplicated_declaration(declaration: &Declaration) -> Result<bool, ParserInternalError> {
    if matches!(
        declaration.kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return Ok(true);
    }

    Ok(false)
}
