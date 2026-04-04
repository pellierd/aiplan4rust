use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::passes::SymbolResolverError;
use crate::aiplan4rust::semantic::signature_matcher::{MatchResult, SignatureMatcher};
use crate::aiplan4rust::semantic::symbol::Signature;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::{NodeId, Tree};
use crate::SymbolTable;

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
