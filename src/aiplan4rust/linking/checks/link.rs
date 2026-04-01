use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::rules::resolve_declaration;
use crate::aiplan4rust::semantic::signature_matcher::matcher::SignatureMatcher;
use crate::aiplan4rust::semantic::signature_matcher::result::MatchResult;
use crate::aiplan4rust::semantic::symbol::{
    Filterable, SymbolOrigin,
};
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::{DiagnosticManager, SymbolTable};

pub fn perform_linking(
    problem_table: &mut SymbolTable,
    domain_table: &SymbolTable,
    context: &CheckContext,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut all_resolved = true;
    let mut instructions = Vec::new();

    // 1. Initialisation du Checker en mode "Inter-fichiers"
    // local = problem, annex = domain
    let checker = SignatureMatcher::new(
        problem_table,
        context.syntax_tree(),
        type_checker,
        Some(domain_table),
    );

    // --- PHASE 1 : ANALYSE ---
    for (&symbol_id, symbol_entry) in problem_table.iter() {
        for usage in symbol_entry.usages().values() {
            if usage.declaration().is_some() {
                continue;
            }

            // A. Recherche d'une déclaration LOCALE (dans le Problème)
            // Note : Ici on pourrait aussi utiliser le checker si on voulait valider la signature locale
            /*let local_decl = symbol_entry.declarations().values().find(|d| {
                d.origin() == SymbolOrigin::Problem && d.symbol_kind() == usage.symbol_kind()
            });*/

            let local_decl = resolve_declaration(symbol_entry, usage.symbol_kind(), usage.scope());

            // B. Recherche dans le DOMAINE
            let mut domain_decl_proxy = None;
            if let Some(dom_symbol) = domain_table.get_symbol(symbol_id) {
                // On récupère les candidats potentiels du domaine (souvent un seul en HDDL)
                if let Some(dom_decl) =
                    resolve_declaration(dom_symbol, usage.symbol_kind(), &domain_table.root_scope())
                {
                    // UTILISATION DU CHECKER
                    match checker.match_signature(dom_decl, usage)? {
                        MatchResult::Match => {
                            // On a un lien valide (parfait ou avec upcast)
                            let mut p = dom_decl.clone();
                            p.set_origin(SymbolOrigin::Domain);
                            p.set_alias(dom_decl.source());
                            p.set_source(usage.source());
                            domain_decl_proxy = Some(p);
                        }
                        _ => {
                            // La signature ne colle pas, on ne lie pas au domaine
                        }
                    }
                }
            }

            // C. LOGIQUE DE DÉCISION (inchangée mais plus fiable)
            match (local_decl, domain_decl_proxy) {
                (Some(local), Some(_remote)) => {
                    // Conflit : on priorise le local (objets/variables du problème)
                    instructions.push((symbol_id, usage.source(), local.source(), None));
                }
                (Some(local), None) => {
                    instructions.push((symbol_id, usage.source(), local.source(), None));
                }
                (None, Some(proxy)) => {
                    let proxy_source = proxy.source();
                    instructions.push((symbol_id, usage.source(), proxy_source, Some(proxy)));
                }
                (None, None) => {
                    // Si on n'a rien trouvé, le symbole n'est pas résolu
                    all_resolved = false;
                }
            }
        }
    }

    // --- PHASE 2 : APPLICATION ---
    for (sym_id, usage_id, target_id, opt_proxy) in instructions {
        if let Some(entry) = problem_table.get_symbol_mut(sym_id) {
            if let Some(p) = opt_proxy {
                if !entry.declarations().contains_key(&target_id) {
                    entry.add_declaration(p);
                }
            }
            if let Some(u_mut) = entry.usages_mut().get_mut(&usage_id) {
                u_mut.set_declaration(target_id);
            }
        }
    }

    Ok(all_resolved)
}
