use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::semantic::checks::undeclared_symbols::is_pddl_builtin_symbol;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::rules::check_kind_compatibility;
use crate::{DiagnosticManager, SymbolTable};

pub fn check_unresolved_usages(
    problem_table: &SymbolTable,
    domain_table: &SymbolTable,
    problem_context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    let mut no_error = true;

    for entry in problem_table.values() {
        // --- NOUVEAU : FILTRE BUILT-IN ---
        // Si le symbole est un mot-clé réservé (total-time, number, etc.),
        // on ne vérifie pas s'il est déclaré.
        if is_pddl_builtin_symbol(entry, problem_context) {
            continue;
        }
        for usage in entry.usages().values() {
            // On ne traite que ce qui n'a pas été lié par le perform_linking
            if usage.declaration().is_none() {
                no_error = false;
                let symbol_id = usage.symbol_id();

                if let Some(dom_symbol) = domain_table.get_symbol(symbol_id) {
                    // 1. LE SYMBOLE EXISTE DANS LE DOMAINE
                    let potential_decl = dom_symbol
                        .declarations()
                        .values()
                        .find(|d| check_kind_compatibility(usage.symbol_kind(), d.symbol_kind()));

                    if let Some(declaration) = potential_decl {
                        // CAS : SIGNATURE INVALIDE (Paramètres ou types incorrects)
                        diagnostic_manager.add_diagnostic(
                            Diagnostic::error_invalid_symbol_signature(
                                declaration.clone(),
                                usage.clone(),
                                problem_context.provider(),
                                problem_context.source(),
                                usage.span(),
                            ),
                        );
                    } else {
                        // CAS : MAUVAIS GENRE (ex: Predicate utilisé comme Action)
                        // On considère cela comme "non déclaré" pour ce genre spécifique
                        diagnostic_manager.add_diagnostic(Diagnostic::error_undeclared_symbol(
                            usage.clone(),
                            problem_context.provider(),
                            problem_context.source(),
                            usage.span(),
                        ));
                    }
                } else {
                    // 2. SYMBOLE TOTALEMENT INCONNU (Pas dans le domaine, pas en local)
                    diagnostic_manager.add_diagnostic(Diagnostic::error_undeclared_symbol(
                        usage.clone(),
                        problem_context.provider(),
                        problem_context.source(),
                        usage.span(),
                    ));
                }
            }
        }
    }
    no_error
}
