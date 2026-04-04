use crate::aiplan4rust::interner::{InternerDisplay, SymbolInterner};
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::rules::{find_shadowing_candidate, is_atomic_kind};
use crate::aiplan4rust::semantic::signature_matcher::{MatchResult, SignatureMatcher};
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Signature, SymbolEntry, SymbolOrigin, Usage,
};
use crate::aiplan4rust::semantic::symbol_resolver::error::SymbolResolverError;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

pub struct SymbolResolver<'a> {
    ast: &'a Tree<AstNode>,
    type_checker: Option<&'a TypeChecker<'a>>,
    domain_table: Option<&'a SymbolTable>,
    interner: &'a SymbolInterner,
}

impl<'a> SymbolResolver<'a> {
    pub fn new(
        ast: &'a Tree<AstNode>,
        type_checker: Option<&'a TypeChecker<'a>>,
        domain_table: Option<&'a SymbolTable>,
        interner: &'a SymbolInterner,
    ) -> Self {
        Self {
            domain_table,
            ast,
            type_checker,
            interner,
        }
    }

    pub fn resolve(mut self, table: &mut SymbolTable) -> Result<bool, SymbolResolverError> {
        // Phase 1 & 2 : Symboles (Usages -> Déclarations)
        let all_resolved = resolve_symbols(
            self.ast,
            table,
            self.type_checker,
            self.domain_table,
            self.interner,
        )?;

        println!("{}", all_resolved);

        // Phase 3 : Prédicats dérivés (Axiomes -> Signatures)
        resolve_derived_predicates(self.ast, table, self.type_checker, self.domain_table)?;

        Ok(all_resolved)
    }
}

/*pub fn resolve_symbols(
    ast: &Tree<AstNode>,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
    _interner: &SymbolInterner,
) -> Result<bool, SymbolResolverError> {
    // On prépare le Matcher (si le TypeChecker est là)
    // Note : Le matcher doit être capable de gérer des arguments non encore "vissés"
    // en regardant dans les instructions temporaires si besoin.
    let matcher = type_checker.map(|tc| SignatureMatcher::new(table, ast, tc, domain_table));

    // 1. UNE SEULE COLLECTE
    // On passe le matcher directement. La fonction doit gérer l'ordre interne.
    let (instructions, all_resolved) = collect_resolutions(table, matcher.as_ref(), domain_table)?;

    // 2. UN SEUL APPLY
    if !instructions.is_empty() {
        apply_resolutions(table, instructions);
    }

    Ok(all_resolved)
}*/

pub fn resolve_symbols(
    ast: &Tree<AstNode>,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
    interner: &SymbolInterner,
) -> Result<bool, SymbolResolverError> {
    /*println!(
        "{}",
        ast.try_root()
            .unwrap()
            .to_string_with_interner(ast, interner)
    );*/
    // --- PHASE 1 : ATOMIQUES (Variables, Objets) ---
    // On résout les feuilles d'abord, sans Matcher.
    let (atomic_instructions, _) = collect_atomic_resolutions(table, domain_table)?;

    // On "visse" les variables immédiatement dans la table.
    if !atomic_instructions.is_empty() {
        apply_resolutions(table, atomic_instructions);
    }

    print!("{}", table.to_string_with_interner(interner));
    // --- PHASE 2 : COMPLEXES (Prédicats, Actions) ---
    // Maintenant que les variables sont dans la table, on peut lancer le Matcher.
    let (complex_instructions, all_resolved) = {
        if let Some(tc) = type_checker {
            // Le matcher voit enfin les résolutions de la Phase 1 !
            let matcher = SignatureMatcher::new(table, ast, tc, domain_table);
            collect_complex_resolutions(table, &matcher, domain_table)?
        } else {
            // Si pas de type_checker, on ne peut pas résoudre de complexes
            (Vec::new(), true)
        }
    };

    // On "visse" les prédicats.
    if !complex_instructions.is_empty() {
        apply_resolutions(table, complex_instructions);
    }

    Ok(all_resolved)
}

fn collect_atomic_resolutions(
    table: &SymbolTable,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SymbolResolverError> {
    let mut instructions = Vec::new();
    let mut all_resolved = true;

    for entry in table.values() {
        for usage in entry.usages().values() {
            // On ne traite QUE les atomiques non résolus
            if usage.resolution().is_some() || !is_atomic_kind(usage.symbol().kind()) {
                continue;
            }

            // Résolution simple (le matcher est None)
            let best_match = resolve_local_match(entry, usage, None)?.or_else(|| {
                resolve_domain_match(entry.ident(), domain_table, usage, None)
                    .ok()
                    .flatten()
            });

            if let Some(res) = best_match {
                instructions.push((usage.symbol().id(), usage.source(), res));
            } else {
                all_resolved = false;
            }
        }
    }
    Ok((instructions, all_resolved))
}
fn collect_complex_resolutions(
    table: &SymbolTable,
    matcher: &SignatureMatcher,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SymbolResolverError> {
    let mut instructions = Vec::new();
    let mut all_resolved = true;

    for entry in table.values() {
        for usage in entry.usages().values() {
            if entry.ident().as_usize() == 1865 {
                // L'ID de keep_engine_turning
                println!(
                    "👋 [BOUCLE] Je passe sur l'entrée keep_engine_turning ! Usages: {}",
                    entry.usages().len()
                );
            }
            let kind = usage.symbol().kind();
            let node_id = usage.source();
            let is_atomic = is_atomic_kind(kind);
            let has_resolution = usage.resolution().is_some();

            if has_resolution || is_atomic {
                continue;
            }

            // Résolution avec Signature Matching
            let best_match = resolve_local_match(entry, usage, Some(matcher))?.or_else(|| {
                resolve_domain_match(entry.ident(), domain_table, usage, Some(matcher))
                    .ok()
                    .flatten()
            });

            if let Some(res) = best_match {
                instructions.push((usage.symbol().id(), usage.source(), res));
            } else {
                all_resolved = false;
                // AJOUTE CECI :
                println!(
                    "❌ ÉCHEC FINAL : L'usage #{} ({}) n'a trouvé aucun match valide",
                    usage.source(),
                    entry.ident()
                );
            }
        }
    }

    Ok((instructions, all_resolved))
}
fn collect_resolutions(
    table: &SymbolTable,
    matcher: Option<&SignatureMatcher>,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SymbolResolverError> {
    let mut instructions = Vec::new();
    let mut all_resolved = true;

    // On itère sur les symboles de la table de manière immuable
    for entry in table.values() {
        for usage in entry.usages().values() {
            // Si l'usage est déjà résolu (par une passe précédente), on passe
            if usage.resolution().is_some() {
                continue;
            }

            // Tentative de résolution :
            // 1. On cherche d'abord en LOCAL
            // 2. Si non trouvé (None), on cherche dans le DOMAINE via `or_else`
            let best_match = resolve_local_match(entry, usage, matcher)?.or_else(|| {
                // On ne cherche dans le domaine que si on a un identifiant de symbole
                resolve_domain_match(entry.ident(), domain_table, usage, matcher)
                    .ok()
                    .flatten()
            });

            // Enregistrement de la décision
            if let Some(res) = best_match {
                instructions.push((entry.ident(), usage.source(), res));
            } else {
                // Si aucune des deux sources n'a pu résoudre le symbole
                all_resolved = false;
            }
        }
    }

    Ok((instructions, all_resolved))
}

fn apply_resolutions(table: &mut SymbolTable, instructions: Vec<(SymbolId, NodeId, Resolution)>) {
    for (sym_id, usage_id, resolution) in instructions {
        if let Some(entry) = table.get_symbol_mut(sym_id) {
            // 1. DÉTERMINER LA DÉCLARATION CIBLE ET LE STATUT
            let (final_decl_id, status) = match resolution {
                // Cas Local : On a déjà l'ID de la déclaration
                Resolution::Local(decl_id, status) => (decl_id, status),

                // Cas Domaine : On gère le Proxy
                Resolution::Domain(proxy, status) => {
                    let proxy_source = proxy.source();

                    // STRATÉGIE DE FUSION : On évite de dupliquer le même symbole du domaine
                    let existing_id = entry
                        .declarations()
                        .values()
                        .find(|d| {
                            d.origin() == SymbolOrigin::Domain && d.source() == proxy.source()
                        })
                        .map(|d| d.source());

                    if let Some(id) = existing_id {
                        (id, status)
                    } else {
                        // Premier usage de ce symbole du domaine dans ce fichier : on l'ajoute
                        entry.add_declaration(proxy);
                        (proxy_source, status)
                    }
                }
            };

            // 2. MISE À JOUR DES LIENS BIDIRECTIONNELS

            // Lien : Déclaration -> Usage
            if let Some(decl_mut) = entry.declarations_mut().get_mut(&final_decl_id) {
                decl_mut.add_usage(usage_id);
            }

            // Dans apply_resolutions, juste avant le set_declaration
            if let Some(u_mut) = entry.usages_mut().get_mut(&usage_id) {
                u_mut.set_declaration(final_decl_id);
                u_mut.set_resolution(status);
                println!("✅ Usage {} lié à Décl {}", usage_id, final_decl_id);
            }
        }
    }
}

pub fn resolve_derived_predicates(
    ast: &Tree<AstNode>,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
) -> Result<(), SymbolResolverError> {
    // 1. Sécurité : Si on n'a pas de TypeChecker, on ne peut pas matcher les signatures
    let Some(tc) = type_checker else {
        return Ok(());
    };

    // 2. ÉTAPE DE COLLECTE (Lecture seule / Immuable)
    // On crée le matcher une seule fois pour toute la table.
    // Le bloc { } assure que l'emprunt immuable de `table` est relâché à la fin.
    let all_links = {
        let matcher = SignatureMatcher::new(table, ast, tc, domain_table);
        collect_all_derived_links(table, &matcher)?
    };

    // 3. ÉTAPE D'APPLICATION (Ecriture / Mutable)
    // On n'exécute la mutation que si on a trouvé des liens à créer.
    if !all_links.is_empty() {
        apply_derived_links(table, all_links);
    }

    Ok(())
}

fn collect_all_derived_links(
    table: &SymbolTable,
    matcher: &SignatureMatcher,
) -> Result<Vec<(SymbolId, NodeId, NodeId)>, SymbolResolverError> {
    let mut global_links = Vec::new();

    for entry in table.values() {
        let mut bases = Vec::new();
        let mut derived = Vec::new();

        // On trie les déclarations de cette entrée spécifique
        for (&node_id, decl) in entry.declarations() {
            if decl.is_derived() {
                derived.push((node_id, decl));
            } else {
                bases.push((node_id, decl));
            }
        }

        if bases.is_empty() || derived.is_empty() {
            continue;
        }

        // Pour chaque axiome de cette entrée, on cherche sa base correspondante
        for (d_id, d_decl) in derived {
            for &(b_id, b_decl) in &bases {
                let expected = Signature::from_declaration(b_decl);
                let provided = Signature::from_declaration(d_decl);

                if let MatchResult::Match = matcher.match_signature(expected, provided)? {
                    // On enregistre le lien complet
                    global_links.push((entry.ident(), b_id, d_id));
                    // On ne break pas ici si un axiome peut avoir plusieurs bases,
                    // mais en PDDL on s'arrête généralement à la première valide.
                    break;
                }
            }
        }
    }
    Ok(global_links)
}

pub fn apply_derived_links(table: &mut SymbolTable, links: Vec<(SymbolId, NodeId, NodeId)>) {
    for (symbol_id, base_id, axiom_id) in links {
        // On récupère l'entrée correspondante au symbole (ex: "at")
        if let Some(entry) = table.get_symbol_mut(symbol_id) {
            // On verrouille l'accès aux déclarations de cette entrée
            let decls = entry.declarations_mut();

            // 1. On "visse" l'Axiome vers sa Signature parente
            if let Some(axiom) = decls.get_mut(&axiom_id) {
                axiom.set_derived_source(base_id);
            }

            // 2. On "visse" la Signature vers son Axiome enfant
            if let Some(base) = decls.get_mut(&base_id) {
                base.add_derivation(axiom_id);
            }
        }
    }
}

/// Fonction utilitaire pour la résolution dans le domaine (externe).
/// Retourne `Resolution::Domain` contenant le Proxy si un match est trouvé.
fn resolve_domain_match(
    symbol_id: SymbolId,
    domain_table: Option<&SymbolTable>,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<Resolution>, SymbolResolverError> {
    // 1. Extraction et vérification de l'existence dans le domaine
    let Some(table) = domain_table else {
        return Ok(None);
    };
    let Some(dom_symbol) = table.get_symbol(symbol_id) else {
        return Ok(None);
    };

    // Recherche du candidat (Shadowing rules)
    let Some(winner) = find_shadowing_candidate(dom_symbol, usage.symbol().kind(), usage.scope())
    else {
        return Ok(None);
    };

    // 2. Calcul du statut (Court-circuit si atomique ou Match complet)
    let status = if is_atomic_kind(winner.symbol_kind()) {
        MatchResult::Match
    } else {
        let expected = Signature::from_declaration(winner);
        let observed = Signature::from_usage(winner, usage);

        match checker {
            Some(m) => m.match_signature(expected, observed)?,
            None => match SignatureMatcher::match_structure(expected, observed) {
                Ok(_) => MatchResult::Match,
                Err(e) => MatchResult::NoMatch(e),
            },
        }
    };

    // 3. Si pas de match, on sort proprement
    if !status.is_match() {
        return Ok(None);
    }

    // 4. Création du Proxy
    // On clone la déclaration du domaine pour l'importer localement
    let mut proxy = winner.clone();
    proxy.set_origin(SymbolOrigin::Domain);
    proxy.set_alias(winner.source()); // L'ID original dans le domaine
    proxy.set_source(usage.source()); // Le NodeId de l'usage qui a provoqué l'import

    // On retourne la variante Domain qui encapsule le proxy et le statut
    Ok(Some(Resolution::Domain(proxy, status)))
}

/// Fonction utilitaire pour la résolution locale (Zero-allocation)
/// Fonction utilitaire pour la résolution locale (Zero-allocation).
/// Supporte un mode dégradé si le SignatureMatcher est absent.
/// Fonction utilitaire pour la résolution locale.
/// Retourne `Resolution::Local` encapsulé dans un `Option`.
pub fn resolve_local_match(
    symbol_entry: &SymbolEntry,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<Resolution>, SymbolResolverError> {
    // TRACE CIBLÉE SUR LE NŒUD 1865
    let is_target = usage.source().as_usize() == 1865;

    if is_target {
        println!(
            "🎯 [RESOLVE LOCAL] Tentative pour #1865 ('{}')",
            symbol_entry.ident()
        );
        println!("   - Kind Usage: {:?}", usage.symbol().kind());
        println!("   - Scope Usage: {:?}", usage.scope());
        println!(
            "   - Nb Décl dans l'entrée: {}",
            symbol_entry.declarations().len()
        );
    }

    // 1. Recherche du candidat (Shadowing)
    let candidate = find_shadowing_candidate(symbol_entry, usage.symbol().kind(), usage.scope());

    let Some(winner) = candidate else {
        if is_target {
            println!("   ❌ SHADOWING ÉCHOUÉ : find_shadowing_candidate a renvoyé None");
            // On liste les décl pour voir pourquoi elles ont été rejetées
            for d in symbol_entry.declarations().values() {
                println!(
                    "     -> Décl disponible #{} (Kind: {:?}, Scope: {:?})",
                    d.source(),
                    d.symbol_kind(),
                    d.scope()
                );
            }
        }
        return Ok(None);
    };

    if is_target {
        println!(
            "   ✅ WINNER TROUVÉ : Décl #{} (Kind: {:?})",
            winner.source(),
            winner.symbol_kind()
        );
    }

    // 2. Cas "Atomique"
    if is_atomic_kind(winner.symbol_kind()) {
        if is_target {
            println!("   ⚡ ATOMIQUE : Match direct");
        }
        return Ok(Some(Resolution::Local(winner.source(), MatchResult::Match)));
    }

    // 3. Signature Matching
    if let Some(matcher) = checker {
        if is_target {
            println!("   🧪 Lancement du SignatureMatcher...");
        }

        let arg_signature = Signature::from_usage(winner, usage);
        let decl_signature = Signature::from_declaration(winner);

        let match_res = matcher.match_signature(decl_signature, arg_signature)?;

        if is_target {
            println!("   📊 Résultat Signature: {:?}", match_res);
        }

        if match_res.is_match() {
            return Ok(Some(Resolution::Local(winner.source(), match_res)));
        }
    }

    Ok(None)
}

/// Représente le résultat d'une décision de résolution avant application.
/// Cet enum est interne au module car il sert uniquement de "tampon".
enum Resolution {
    /// Trouvé localement.
    Local(NodeId, MatchResult),
    /// Trouvé dans le domaine (nécessite un Proxy).
    Domain(Declaration, MatchResult),
}

impl Resolution {
    fn status(&self) -> &MatchResult {
        match self {
            Resolution::Local(_, s) | Resolution::Domain(_, s) => s,
        }
    }
}
