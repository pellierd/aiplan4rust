use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

use std::collections::HashMap;

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
pub fn check(
    syntax_tree: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;
    let symbol_table = syntax_tree.symbol_table();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        let mut seen_scopes: HashMap<Scope, Declaration> = HashMap::new();

        // Iterate over each declaration for the symbol.
        for declaration in symbol.declarations() {
            // Skip duplicate checks for symbols of kind DomainName or ProblemName.
            if skip_duplicated_declaration(declaration)? {
                continue;
            }

            let ast_entry = syntax_tree.get_entry(declaration.ast()).unwrap();
            let current_scope = declaration.scope();

            // Check if a declaration has already been encountered in this scope or any parent scope
            let maybe_conflict = seen_scopes.iter().find(|(s, _)| current_scope.starts_with(s));

            if let Some((conflicting_scope, previous_declaration)) = maybe_conflict {
                let current_kind = declaration.kind();
                let previous_kind = previous_declaration.kind();

                // Hack to allow a symbol declared as a PrimitiveType with the same name as a Predicate symbol
                // It's not good practice, but it's a hack and not an error
                if (current_kind == &SymbolKind::PrimitiveType && previous_kind == &SymbolKind::Predicate) ||
                    (current_kind == &SymbolKind::Predicate && previous_kind == &SymbolKind::PrimitiveType) {
                    let warning = Diagnostic::new(
                        DiagnosticKind::WarningAmbiguousTypePredicateSymbol {
                            symbol: symbol.name().clone(),
                        },
                        DiagnosticSource::SemanticAnalyzer,
                        syntax_tree.filename().clone(),
                        ast_entry.span().clone(),
                    );

                    diagnostic_manager.add_diagnostic(warning);


                } else {
                    // Otherwise, this is a normal error
                    checked = false;

                    let scope_index = conflicting_scope.iter().last().unwrap();
                    let scope = syntax_tree.get_entry(*scope_index).unwrap();

                    let error = Diagnostic::new(
                        DiagnosticKind::DuplicatedDeclarationInScope {
                            symbol: symbol.name().clone(),
                            declaration1: previous_declaration.clone(),
                            declaration2: declaration.clone(),
                            scope: scope.clone(),
                        },
                        DiagnosticSource::SemanticAnalyzer,
                        syntax_tree.filename().clone(),
                        ast_entry.span().clone(),
                    );

                    diagnostic_manager.add_diagnostic(error);
                }
            } else {
                // No conflict found, record this declaration's scope
                seen_scopes.insert(current_scope.clone(), declaration.clone());
            }
        }
    }

    Ok(checked)
}







/*pub fn check(
    syntax_tree: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;
    let symbol_table = syntax_tree.symbol_table();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        // Pour chaque couple (scope, kind) déjà vu pour ce symbole
        let mut seen_scope_kind = HashSet::new();

        for declaration in symbol.declarations() {
            //if skip_duplicated_declaration(declaration)? {
            //    continue;
            //}

            let scope = declaration.scope();
            let kind = declaration.kind();
            let scope_kind = (scope.clone(), kind);

            let ast_entry = syntax_tree.get_entry(declaration.ast()).unwrap();

            if seen_scope_kind.contains(&scope_kind) {
                checked = false;
                let scope_index = scope.iter().last().unwrap();
                let scope_entry = syntax_tree.get_entry(*scope_index).unwrap();

                let error = Diagnostic::new(
                    DiagnosticKind::DuplicatedDeclarationInScope {
                        symbol: symbol.name().clone(),
                        declaration: declaration.clone(),
                        scope: scope_entry.clone(),
                    },
                    DiagnosticSource::SemanticAnalyzer,
                    syntax_tree.filename().clone(),
                    ast_entry.span().clone(),
                );

                diagnostic_manager.add_diagnostic(error);
            } else {
                seen_scope_kind.insert(scope_kind);
            }
        }
    }

    Ok(checked)
}*/

fn skip_duplicated_declaration(declaration: &Declaration) -> Result<bool, ParserInternalError> {
    if matches!(
        declaration.kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return Ok(true);
    }

    Ok(false)
}
