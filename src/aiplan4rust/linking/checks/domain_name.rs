use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::linking::error::LinkingError;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::SemanticContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;

/// Checks for consistency between the domain name declared in the domain AST
/// and the domain name referenced in the problem AST.
///
/// This function performs the following steps:
/// 1. Resolves the domain name declared in the domain file.
/// 2. Resolves the domain name referenced in the problem file.
/// 3. Compares the two names:
///     - If they match, nothing happens.
///     - If they differ, it emits a diagnostic warning indicating the mismatch.
/// 4. If any expected declaration or AST entry is missing, a `ParserInternalError` is returned.
///
/// # Arguments
/// * `domain` - The annotated syntax arena representing the domain file.
/// * `problem` - The annotated syntax arena representing the problem file.
///
/// # Returns
/// * `Ok(true)` if the check completes successfully (whether or not names match).
/// * `Err(ParserInternalError)` if a domain name declaration or AST entry is missing.
///
/// # Diagnostics
/// Emits a `DomainProblemNameMismatch` warning if the domain names differ.
pub fn check_domain_name(
    domain: &SemanticContext,
    problem: &CheckContext,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, LinkingError> {

    // --- 1. Resolve the domain name declared in the domain AST ---
    let declared = domain.symbol_table().try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 2. Resolve the domain name referenced in the problem AST ---
    let referenced =  problem.symbol_table().try_resolve_unique_declaration(SymbolKind::DomainName)?;

    // --- 3. Compare both domain names ---
    // If the names don't match, emit a diagnostic warning.
    if declared.ident() != referenced.ident() {

        // --- 4. Locate the AST syntax for the referenced domain name ---
        // Try to find the declaration in the problem's symbol table.
        match problem.symbol_table().resolve_declaration(
            &referenced.ident(),
            &SymbolKind::DomainName,
            &problem.symbol_table().root_scope(),
        )? {
            Some(domain_name_declaration) => {

                // --- 5. Retrieve the corresponding AST entry ---
                // Needed to determine the span (location) for the warning.
                match problem.syntax_tree().get_node(domain_name_declaration.node_id()) {
                    Some(ast) => {

                        // --- 6. Emit a warning about the mismatch ---
                        // Includes both names in the diagnostic message.

                        let domain_name = domain.interner().try_resolve(declared.ident())?;
                        let problem_domain_name = problem.interner().try_resolve(referenced.ident())?;
                        let warning = Diagnostic::new(
                            DiagnosticKind::DomainProblemNameMismatch {
                                domain_name: domain_name.to_string(),
                                problem_name: problem_domain_name.to_string(),
                            },
                            source,
                            problem.source_name().to_string(),
                            ast.span().clone(),
                        );
                        diagnostic_manager.add_diagnostic(warning);
                    }
                    None => {
                        // AST entry is missing for the declaration — this should not happen
                        return Err(LinkingError::InternalError(
                            "AST entry for domain name declaration not found in problem AST.".to_string(),
                        ));
                    }
                }
            }
            None => {
                // No declaration found for the domain name in the problem's symbol table
                return Err(LinkingError::InternalError(
                    "Domain name declaration not found in problem symbol table".to_string(),
                ));
            }
        }
    }

    // --- 7. Names match or warning has been emitted; return success ---
    Ok(true)
}
