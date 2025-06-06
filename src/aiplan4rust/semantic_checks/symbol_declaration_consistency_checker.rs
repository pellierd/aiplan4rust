use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::Scope;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;

use std::collections::HashMap;
use crate::aiplan4rust::parser::SymbolOrigin;

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
    domain: &AnnotatedSyntaxTree,
    problem: &AnnotatedSyntaxTree,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut all_ok = true;

    let domain_symbol_table = domain.symbol_table();
    let problem_symbol_table = problem.symbol_table();

    for symbol in problem_symbol_table.values() {
        for declaration in symbol.declarations() {
            if skip_duplicated_declaration(declaration)? {
                continue;
            }

            if *declaration.source() == SymbolOrigin::Problem {
                let domain_decls: Vec<_> = domain_symbol_table
                    .collect_declarations(Some(symbol.name()), None, Some(&Scope::root()))
                    .into_iter()
                    .filter(|d| {
                        !matches!(d.kind(), SymbolKind::DomainName | SymbolKind::ProblemName)
                    })
                    .collect();

                if !domain_decls.is_empty() {
                    let domain_kinds: Vec<SymbolKind> =
                        domain_decls.iter().map(|d| d.kind().clone()).collect();

                    let same_kind_exists =
                        domain_kinds.iter().any(|k| k == declaration.kind());

                    if !same_kind_exists {
                        let ast = problem.get_entry(declaration.ast()).ok_or_else(|| {
                            ParserInternalError::new(format!(
                                "Missing AST entry for declaration with id: {}",
                                declaration.ast()
                            ))
                        })?;

                        let error = Diagnostic::new(
                            DiagnosticKind::ErrorConflictSymbolDeclaration {
                                symbol: symbol.name().clone(),
                                problem_kind: declaration.kind().clone(),
                                domain_kinds,
                            },
                            DiagnosticSource::Linker,
                            problem.filename().clone(),
                            ast.span().clone(),
                        );
                        diagnostic_manager.add_diagnostic(error);
                        all_ok = false;
                    }
                }
            }
        }
    }

    Ok(all_ok)
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
