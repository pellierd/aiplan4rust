use crate::aiplan4rust::semantic::checks::symbol_signatures::match_declaration_with_usage;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Filterable, Scope, SymbolEntry, SymbolKind, SymbolOrigin, Usage,
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

    // --- PHASE 1 : ANALYSE ---
    for (symbol_id, symbol_entry) in problem_table.iter() {
        for usage in symbol_entry.usages().values() {
            if usage.resolved_declaration().is_some() {
                continue;
            }

            // 1. On cherche en LOCAL
            let local_decl = symbol_entry.declarations().values().find(|d| {
                d.origin() == SymbolOrigin::Problem && d.symbol_kind() == usage.symbol_kind()
            });

            // 2. On cherche dans le DOMAINE
            let mut domain_decl_proxy = None;
            if let Some(dom_symbol) = domain_table.get_symbol(*symbol_id) {
                let candidates =
                    find_domain_declaration(dom_symbol, usage, &domain_table.root_scope());
                if let Some(dom_decl) = candidates.first() {
                    if match_declaration_with_usage(
                        dom_decl,
                        usage,
                        problem_table,
                        context,
                        type_checker,
                        diagnostic_manager,
                    )? {
                        let mut p = dom_decl.clone();
                        p.set_origin(SymbolOrigin::Domain);
                        p.set_alias(dom_decl.source());
                        p.set_source(usage.source());
                        domain_decl_proxy = Some(p);
                    }
                }
            }

            // 3. LOGIQUE DE DÉCISION
            match (local_decl, domain_decl_proxy) {
                (Some(local), Some(_remote)) => {
                    // CAS CONFLIT : On a les deux.
                    // Option A : On logue un warning et on prend le local

                    instructions.push((*symbol_id, usage.source(), local.source(), None));
                }
                (Some(local), None) => {
                    instructions.push((*symbol_id, usage.source(), local.source(), None));
                }
                (None, Some(proxy)) => {
                    let proxy_source = proxy.source();
                    instructions.push((*symbol_id, usage.source(), proxy_source, Some(proxy)));
                }
                (None, None) => {
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
                u_mut.set_resolved_declaration(target_id);
            }
        }
    }

    Ok(all_resolved)
}

pub fn find_domain_declaration<'a>(
    dom_symbol: &'a SymbolEntry,
    usage_in_prob: &Usage,
    scope: &Scope,
) -> Vec<Declaration> {
    let usage_kind = usage_in_prob.symbol_kind();

    dom_symbol
        .declarations()
        .values()
        .filter(|declaration| {
            let decl_kind = declaration.symbol_kind();

            // 1. Filtrage des noms de structures
            if !matches!(usage_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
                && matches!(decl_kind, SymbolKind::DomainName | SymbolKind::ProblemName)
            {
                return false;
            }

            // 2. Visibilité globale
            let is_global = declaration.scope() == scope;

            // 3. Compatibilité des genres (ASSOUPLIE)
            let kind_match = if usage_kind == decl_kind {
                true
            } else {
                // On autorise un usage marqué "Task" à être résolu par une "Action" du domaine
                (usage_kind == SymbolKind::Task && decl_kind == SymbolKind::Action)
                    || (usage_kind == SymbolKind::Action && decl_kind == SymbolKind::Task)
            };

            is_global && kind_match
        })
        .cloned()
        .collect()
}
