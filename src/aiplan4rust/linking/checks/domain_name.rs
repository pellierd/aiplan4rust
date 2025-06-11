use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::hir::HirTree;
use crate::aiplan4rust::semantic::symbol::{Scope, SymbolKind};

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
/// * `domain` - The annotated syntax tree representing the domain file.
/// * `problem` - The annotated syntax tree representing the problem file.
///
/// # Returns
/// * `Ok(true)` if the check completes successfully (whether or not names match).
/// * `Err(ParserInternalError)` if a domain name declaration or AST entry is missing.
///
/// # Diagnostics
/// Emits a `DomainProblemNameMismatch` warning if the domain names differ.
pub fn check_domain_name(
    domain: &HirTree,
    problem: &HirTree,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {

    // --- 1. Resolve the domain name declared in the domain AST ---
    // Tries to extract the domain name from the domain's symbol table.
    // If not found, returns an internal syntax error.
    let declared_domain_name = match domain.symbol_table().resolve_domain_name_declaration()? {
        Some(name) => name,
        None => {
            return Err(ParserInternalError::new(
                "Domain name declaration not found in domain AST".to_string(),
            ));
        }
    };

    // --- 2. Resolve the domain name referenced in the problem AST ---
    // Tries to extract the expected domain name from the problem file.
    // If not found, returns an internal syntax error.
    let referenced_domain_name = match problem.symbol_table().resolve_domain_name_declaration()? {
        Some(name) => name,
        None => {
            return Err(ParserInternalError::new(
                "Domain name declaration not found in problem AST".to_string(),
            ));
        }
    };

    // --- 3. Compare both domain names ---
    // If the names don't match, emit a diagnostic warning.
    if declared_domain_name.name() != referenced_domain_name.name() {

        // --- 4. Locate the AST node for the referenced domain name ---
        // Try to find the declaration in the problem's symbol table.
        match problem.symbol_table().resolve_declaration(
            referenced_domain_name.name(),
            &SymbolKind::DomainName,
            &Scope::root(),
        )? {
            Some(domain_name_declaration) => {

                // --- 5. Retrieve the corresponding AST entry ---
                // Needed to determine the span (location) for the warning.
                match problem.get_entry(domain_name_declaration.ast()) {
                    Some(ast) => {

                        // --- 6. Emit a warning about the mismatch ---
                        // Includes both names in the diagnostic message.
                        let warning = Diagnostic::new(
                            DiagnosticKind::DomainProblemNameMismatch {
                                domain_name: declared_domain_name.name().clone(),
                                problem_name: referenced_domain_name.name().clone(),
                            },
                            source,
                            problem.filename().clone(),
                            ast.span().clone(),
                        );
                        diagnostic_manager.add_diagnostic(warning);
                    }
                    None => {
                        // AST entry is missing for the declaration — this should not happen
                        return Err(ParserInternalError::new(
                            "AST entry for domain name declaration not found in problem AST.".to_string(),
                        ));
                    }
                }
            }
            None => {
                // No declaration found for the domain name in the problem's symbol table
                return Err(ParserInternalError::new(
                    "Domain name declaration not found in problem symbol table".to_string(),
                ));
            }
        }
    }

    // --- 7. Names match or warning has been emitted; return success ---
    Ok(true)
}
