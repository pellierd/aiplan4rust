use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::rules::{
    check_kind_compatibility, is_atomic_kind, is_structural_mismatch,
};
use crate::aiplan4rust::semantic::signature_matcher::{MatchResult, SignatureMatcher};
use crate::aiplan4rust::semantic::symbol::{
    Declaration, SymbolEntry, SymbolKind, SymbolOrigin, Usage,
};
use crate::aiplan4rust::semantic::symbol_resolver::error::SymbolResolverError;
use crate::aiplan4rust::semantic::symbol_resolver::resolution::Resolution;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::Tree;
use crate::SymbolTable;

pub struct SymbolResolver<'a> {
    ast: &'a Tree<AstNode>,
    type_checker: Option<&'a TypeChecker<'a>>,
    domain_table: Option<&'a SymbolTable>,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(
        ast: &'a Tree<AstNode>,
        type_checker: Option<&'a TypeChecker<'a>>,
        domain_table: Option<&'a SymbolTable>,
    ) -> Self {
        Self {
            domain_table,
            ast,
            type_checker,
        }
    }

    pub fn resolve(self, table: &mut SymbolTable) -> Result<bool, SymbolResolverError> {
        let mut all_resolved = true;
        let mut instructions = Vec::new();

        // On ne crée le matcher QUE si on a un TypeChecker
        let matcher = self
            .type_checker
            .map(|tc| SignatureMatcher::new(table, self.ast, tc, self.domain_table));

        let symbol_ids: Vec<SymbolId> = table.keys().cloned().collect();

        for symbol_id in symbol_ids {
            let symbol_entry = table.get_symbol(symbol_id).unwrap();

            for usage in symbol_entry.usages().values() {
                if usage.resolution().is_some() {
                    continue;
                }

                // 1. CHERCHE EN LOCAL
                // Retourne Some((Resolution, None)) si trouvé
                let mut best_match = resolve_local_match(symbol_entry, usage, matcher.as_ref())?;

                // 2. CHERCHE DANS LE DOMAINE (Si pas trouvé en local)
                // Retourne Some((Resolution, Some(Proxy))) si trouvé
                if best_match.is_none() {
                    best_match = resolve_domain_match(
                        symbol_id,
                        self.domain_table,
                        usage,
                        matcher.as_ref(),
                    )?;
                }

                // 3. DÉCISION
                if let Some((res, proxy)) = best_match {
                    // On n'ajoute l'instruction que si on a un vrai résultat (Local ou Domaine)
                    instructions.push((symbol_id, usage.source(), res, proxy));
                } else {
                    // Si best_match est None, on ne fait RIEN.
                    // L'usage restera vierge (declaration = None).
                    // C'est ce qui permet au Linker de passer après sans erreur.
                    all_resolved = false;
                }
            }
        }

        // --- PHASE 2 : APPLICATION (DANS SymbolResolver::resolve) ---
        for (sym_id, usage_id, resolution, opt_proxy) in instructions {
            if let Some(entry) = table.get_symbol_mut(sym_id) {
                let mut final_decl_id = resolution.declaration();

                if let Some(p) = opt_proxy {
                    // STRATÉGIE DE FUSION :
                    // On cherche si un proxy vers la même déclaration du domaine existe déjà.
                    // On utilise l'alias (qui est l'ID source du domaine).
                    let existing_id = entry
                        .declarations()
                        .values()
                        .find(|d| d.origin() == SymbolOrigin::Domain && d.alias() == p.alias())
                        .map(|d| d.source());

                    if let Some(id) = existing_id {
                        // On réutilise l'ID du premier proxy créé pour ce symbole
                        final_decl_id = id;
                    } else {
                        // Premier usage de ce symbole du domaine : on l'ajoute
                        entry.add_declaration(p);
                        // final_decl_id reste celui du proxy (usage.source())
                    }
                }

                // Mise à jour de l'usage
                if let Some(u_mut) = entry.usages_mut().get_mut(&usage_id) {
                    u_mut.set_declaration(final_decl_id);

                    // IMPORTANT : On met à jour le status de l'usage !
                    // Si c'est un Match, l'usage devient 'Resolved' visuellement.
                    u_mut.set_resolution(resolution.status().clone());
                }
            }
        }

        Ok(all_resolved)
    }
}

pub fn resolve_domain_match(
    symbol_id: SymbolId,
    domain_table: Option<&SymbolTable>,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<(Resolution, Option<Declaration>)>, SymbolResolverError> {
    // 1. Extraction directe
    let Some(table) = domain_table else {
        return Ok(None);
    };
    let Some(dom_symbol) = table.get_symbol(symbol_id) else {
        return Ok(None);
    };
    let Some(winner) = find_shadowing_candidate(dom_symbol, usage) else {
        return Ok(None);
    };

    // 2. Calcul du status (Court-circuit si atomique)
    let status = if is_atomic_kind(winner.symbol_kind()) {
        MatchResult::Match
    } else {
        match checker {
            Some(m) => m.match_signature(winner, usage)?,
            None => match SignatureMatcher::match_structure(winner, usage) {
                Ok(_) => MatchResult::Match,
                Err(e) => MatchResult::NoMatch(e),
            },
        }
    };

    // 3. Retour immédiat si pas de match
    if !status.is_match() {
        return Ok(None);
    }

    // 4. Création du Proxy et de la Résolution
    let mut proxy = winner.clone();
    proxy.set_origin(SymbolOrigin::Domain);
    proxy.set_alias(winner.source());
    proxy.set_source(usage.source());

    Ok(Some((Resolution::new(proxy.source(), status), Some(proxy))))
}

/// Fonction utilitaire pour la résolution locale (Zero-allocation)
/// Fonction utilitaire pour la résolution locale (Zero-allocation).
/// Supporte un mode dégradé si le SignatureMatcher est absent.
pub fn resolve_local_match(
    symbol_entry: &SymbolEntry,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<(Resolution, Option<Declaration>)>, SymbolResolverError> {
    // 1. On cherche le candidat. Si rien, on sort tout de suite.
    let Some(winner) = find_shadowing_candidate(symbol_entry, usage) else {
        return Ok(None);
    };

    // 2. Cas "Court-circuit" : Si c'est atomique, on valide et on retourne immédiatement.
    if is_atomic_kind(winner.symbol_kind()) {
        return Ok(Some((
            Resolution::new(winner.source(), MatchResult::Match),
            None,
        )));
    }

    // 3. Cas "Complexe" : On calcule le statut (Signature ou Structure)
    let status = match checker {
        Some(sig_checker) => sig_checker.match_signature(winner, usage)?,
        None => match SignatureMatcher::match_structure(winner, usage) {
            Ok(_) => MatchResult::Match,
            Err(failure) => MatchResult::NoMatch(failure),
        },
    };

    // 4. Si ça match, on retourne la résolution, sinon on sort.
    if status.is_match() {
        return Ok(Some((Resolution::new(winner.source(), status), None)));
    }

    Ok(None)
}

fn find_shadowing_candidate<'a>(
    symbol_entry: &'a SymbolEntry,
    usage: &Usage,
) -> Option<&'a Declaration> {
    let mut best_candidate: Option<&'a Declaration> = None;

    for decl in symbol_entry.declarations().values() {
        // --- MODIFICATION ICI ---
        // Les Objets et Constantes sont visibles partout dans le fichier,
        // peu importe le scope de déclaration.
        let is_global_visibility = matches!(decl.symbol_kind(), SymbolKind::Constant);

        // 1. Filtres structurels
        if check_kind_compatibility(decl.symbol_kind(), usage.symbol_kind())
            && !is_structural_mismatch(decl.symbol_kind(), usage.symbol_kind())
            // On autorise si c'est global OU si le scope correspond (pour les variables ?x)
            && (is_global_visibility || usage.scope().starts_with(decl.scope()))
        {
            // 2. Logique de Shadowing (Inchangée et correcte)
            match best_candidate {
                Some(current_best) if decl.scope().len() > current_best.scope().len() => {
                    best_candidate = Some(decl);
                }
                None => {
                    best_candidate = Some(decl);
                }
                _ => {}
            }
        }
    }

    best_candidate
}
