use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::rules::check_kind_compatibility;
use crate::aiplan4rust::semantic::signature_matcher::matcher::SignatureMatcher;
use crate::aiplan4rust::semantic::signature_matcher::result::MatchResult;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};

/// Checks for errors in the symbol declarations and their usages in the given annotated syntax arena.
///
/// This function scans through the `symbol_table` of the provided `arena` to match each symbol's
/// declarations and usages. It ensures that symbols used in the arena are correctly declared and
/// that their types match the expected types. Errors are added to the provided `ErrorManager`
/// during the process.
///
/// # Arguments
///
/// * `arena` - An `AnnotatedSyntaxTree` that contains the symbols to check.
/// * `type_checker` - A `TypeChecker` used to validate types during the check.
/// * `errors` - A mutable reference to an `ErrorManager` where any errors found during the check
///   will be added.
///
/// # Returns
///
/// A `Result<bool, ParserInternalError>` where:
/// * `Ok(true)` indicates that no errors were found during the check.
/// * `Ok(false)` indicates that errors were found and added to the `ErrorManager`.
/// * `Err(ParserInternalError)` indicates an internal error occurred during the process.
///
/// # Example
///
/// ```rust
/// let mut errors = ErrorManager::new();
/// if atomic_formula_checker::check(&arena, &type_checker, &mut errors).is_ok() {
///     // Handle no errors
/// } else {
///     // Handle errors
///     self.error_manager.add_errors_from(&errors);
/// }
/// ```
pub fn check_symbol_signatures(
    context: &CheckContext,
    symbol_table: &mut SymbolTable,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;
    let mut bindings = Vec::new();

    // 1. Initialisation du Checker.
    // On passe None pour l'annex_table car ici on vérifie la cohérence interne d'un fichier.
    let checker = SignatureMatcher::new(symbol_table, context.syntax_tree(), type_checker, None);

    // Parcourir tous les symboles de la table.
    for symbol in symbol_table.values() {
        // Vérifier toutes les déclarations du symbole.
        for declaration in symbol.declarations().values() {
            if !matches!(
                declaration.symbol_kind(),
                SymbolKind::Predicate
                    | SymbolKind::Function
                    | SymbolKind::Task
                    | SymbolKind::Action
            ) {
                continue;
            }

            // Vérifier tous les usages du symbole.
            for usage in symbol.usages().values() {
                let decl_kind = declaration.symbol_kind();
                let usage_kind = usage.symbol_kind();

                // On ne garde que ce qui a une signature.
                if !matches!(
                    usage_kind,
                    SymbolKind::Predicate
                        | SymbolKind::Function
                        | SymbolKind::Task
                        | SymbolKind::Action
                ) {
                    continue;
                }

                // Vérification de la compatibilité des "genres" (ex: Predicate vs Action).
                if !check_kind_compatibility(decl_kind, usage_kind) {
                    continue;
                }

                // 2. Validation de la signature via le MatchResult
                match checker.match_signature(declaration, usage)? {
                    MatchResult::Match => {
                        // Succès parfait : on enregistre pour le "vissage" final.
                        bindings.push((symbol.ident(), declaration.source(), usage.source()));
                    }
                    MatchResult::UpcastMatch {
                        expected,
                        provided,
                        arg_decl,
                        arg_node_id,
                    } => {
                        // Succès avec réserve : on lie le symbole car c'est un candidat valide...
                        bindings.push((symbol.ident(), declaration.source(), usage.source()));

                        // ... mais on remonte un Warning à l'utilisateur.
                        let arg_node = context.syntax_tree().try_node(arg_node_id)?;
                        let warning = Diagnostic::warning_task_argument_is_supertype_of_declaration(
                            arg_decl,
                            expected,
                            provided,
                            context.provider(),
                            context.source(),
                            arg_node.span(),
                        );
                        diagnostic_manager.add_diagnostic(warning);
                    }
                    MatchResult::NoMatch(_failure) => {
                        // Échec de signature : on récupère la cause précise via 'failure'
                        no_error = false;

                        let entry_node = context.syntax_tree().try_node(usage.source())?;

                        // On passe maintenant 'failure' au diagnostic pour un message d'erreur précis
                        let error = Diagnostic::error_invalid_symbol_signature(
                            declaration.clone(),
                            usage.clone(),
                            context.provider(),
                            context.source(),
                            entry_node.span(),
                        );

                        diagnostic_manager.add_diagnostic(error);
                    }
                }
            }
        }
    }

    // --- PHASE 2 : LE VISSAGE (Mutation) ---
    // La boucle précédente est terminée, l'emprunt immuable sur symbol_table est libéré.
    // On peut maintenant demander l'accès mutable exclusif.
    /*for (symbol_id, decl_node_id, usage_node_id) in bindings {
        let mut entry = symbol_table.try_get_symbol_mut(symbol_id)?;
        // Lien Usage -> Declaration
        if let Some(u) = entry.usages_mut().get_mut(&usage_node_id) {
            u.set_declaration(decl_node_id);
        }
        // Lien Declaration -> Usage
        if let Some(d) = entry.declarations_mut().get_mut(&decl_node_id) {
            d.add_usage(usage_node_id);
        }
    }*/

    Ok(no_error)
}
