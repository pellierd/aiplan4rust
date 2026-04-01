use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::signature_matcher::{MatchResult, SignatureMatcher};
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Scope, SymbolEntry, SymbolKind, SymbolOrigin, Usage,
};
use crate::aiplan4rust::semantic::symbol_resolver::error::SymbolResolverError;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::SymbolTable;

pub struct SymbolResolver<'a> {
    local_table: &'a mut SymbolTable,
    annex_table: Option<&'a SymbolTable>,
    context: &'a CheckContext<'a>,
    type_checker: &'a TypeChecker<'a>,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(
        local_table: &'a mut SymbolTable,
        annex_table: Option<&'a SymbolTable>,
        context: &'a CheckContext<'a>,
        type_checker: &'a TypeChecker<'a>,
    ) -> Self {
        Self {
            local_table,
            annex_table,
            context,
            type_checker,
        }
    }

    pub fn resolve(&mut self) -> Result<bool, SymbolResolverError> {
        let mut all_resolved = true;
        let mut instructions = Vec::new();

        // 1. Initialisation du Checker
        // local = local_table, annex = annex_table (peut être None)
        let checker = SignatureMatcher::new(
            self.local_table,
            self.context.syntax_tree(),
            self.type_checker,
            self.annex_table,
        );

        // --- PHASE 1 : ANALYSE ---
        // On récupère les IDs pour éviter les problèmes d'emprunt mutable sur la table complète
        let symbol_ids: Vec<SymbolId> = self.local_table.keys().cloned().collect();

        for symbol_id in symbol_ids {
            let symbol_entry = self.local_table.get_symbol(symbol_id).unwrap();

            for usage in symbol_entry.usages().values() {
                if usage.declaration().is_some() {
                    continue;
                }

                // A. Recherche d'une déclaration LOCALE
                let local_decl = symbol_entry.declarations().values().find(|d| {
                    // Si on est dans un problème, l'origine est Problem.
                    // Si on check un domaine seul, l'origine sera Domain.
                    d.symbol_kind() == usage.symbol_kind()
                });

                // B. Recherche dans l'ANNEXE (le Domaine, si fourni)
                let mut domain_decl_proxy = None;
                if let Some(domain_table) = self.annex_table {
                    if let Some(dom_symbol) = domain_table.get_symbol(symbol_id) {
                        // On réutilise ta logique de filtrage (voir find_domain_declaration plus bas)
                        let candidates =
                            find_domain_declaration(dom_symbol, usage, &domain_table.root_scope());

                        if let Some(dom_decl) = candidates.first() {
                            // UTILISATION DU CHECKER
                            match checker.match_signature(dom_decl, usage)? {
                                MatchResult::Match | MatchResult::UpcastMatch { .. } => {
                                    let mut p = dom_decl.clone();
                                    p.set_origin(SymbolOrigin::Domain);
                                    p.set_alias(dom_decl.source());
                                    p.set_source(usage.source());
                                    domain_decl_proxy = Some(p);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // C. LOGIQUE DE DÉCISION
                match (local_decl, domain_decl_proxy) {
                    (Some(local), Some(_remote)) => {
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
                        all_resolved = false;
                        // On pourrait ajouter un diagnostic ici si besoin
                    }
                }
            }
        }

        // --- PHASE 2 : APPLICATION ---
        for (sym_id, usage_id, target_id, opt_proxy) in instructions {
            if let Some(entry) = self.local_table.get_symbol_mut(sym_id) {
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
