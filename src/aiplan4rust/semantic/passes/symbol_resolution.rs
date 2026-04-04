use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::passes::error::SemanticPassError;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::rules::{find_shadowing_candidate, is_atomic_kind};
use crate::aiplan4rust::semantic::signature_matcher::{MatchResult, SignatureMatcher};
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Signature, SymbolEntry, SymbolOrigin, Usage,
};
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::tree::NodeId;
use crate::SymbolTable;

pub fn resolve_symbols(
    context: &PassContext,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
) -> Result<bool, SemanticPassError> {
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

    // --- PHASE 2 : COMPLEXES (Prédicats, Actions) ---
    // Maintenant que les variables sont dans la table, on peut lancer le Matcher.
    let (complex_instructions, all_resolved) = {
        if let Some(tc) = type_checker {
            // Le matcher voit enfin les résolutions de la Phase 1 !
            let matcher = SignatureMatcher::new(table, context.syntax_tree(), tc, domain_table);
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
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SemanticPassError> {
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
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SemanticPassError> {
    let mut instructions = Vec::new();
    let mut all_resolved = true;

    for entry in table.values() {
        for usage in entry.usages().values() {
            let kind = usage.symbol().kind();
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
            }
        }
    }

    Ok((instructions, all_resolved))
}
fn collect_resolutions(
    table: &SymbolTable,
    matcher: Option<&SignatureMatcher>,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SemanticPassError> {
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
            }
        }
    }
}

/// Utility function for resolving symbols within the global Domain (external) context.
///
/// If a matching declaration is found in the domain, it returns a `Resolution::Domain`
/// containing a "Proxy" declaration. This proxy acts as a local representative of
/// the external symbol for the current problem context.
///
/// # Arguments
/// * `symbol_id` - The identifier of the symbol to resolve.
/// * `domain_table` - The global symbol table (Domain file).
/// * `usage` - The usage site (call site) triggering the resolution.
/// * `checker` - An optional [`SignatureMatcher`] for deep semantic validation.
///
/// # Returns
/// * `Ok(Some(Resolution))` - A domain resolution containing the proxy and match status.
/// * `Ok(None)` - If the symbol is missing from the domain or incompatible.
///
/// # Errors
/// * Returns [`SemanticPassError`] if signature matching or structural validation fails.
fn resolve_domain_match(
    symbol_id: SymbolId,
    domain_table: Option<&SymbolTable>,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<Resolution>, SemanticPassError> {
    // --- STEP 1: Domain Existence Check ---
    // Ensure the domain table is provided and contains the requested symbol identifier.
    let Some(table) = domain_table else {
        return Ok(None);
    };
    let Some(dom_symbol) = table.get_symbol(symbol_id) else {
        return Ok(None);
    };

    // Apply shadowing and filtering rules to find the best candidate within the domain.
    let Some(winner) = find_shadowing_candidate(dom_symbol, usage.symbol().kind(), usage.scope())
    else {
        return Ok(None);
    };

    // --- STEP 2: Match Status Calculation ---
    // Determine if the usage site matches the domain declaration.
    let status = if is_atomic_kind(winner.symbol_kind()) {
        // Atomic symbols (Types/Constants) are considered a match upon identification.
        MatchResult::Match
    } else {
        let expected = Signature::from_declaration(winner);
        let observed = Signature::from_usage(winner, usage);

        match checker {
            // Full semantic check if a matcher is available.
            Some(m) => m.match_signature(expected, observed)?,
            // Fallback to basic structural check (arity/kind) if no matcher is provided.
            None => match SignatureMatcher::match_structure(expected, observed) {
                Ok(_) => MatchResult::Match,
                Err(e) => MatchResult::NoMatch(e),
            },
        }
    };

    // --- STEP 3: Result Validation ---
    // If the structural or semantic matching failed, we discard the candidate.
    if !status.is_match() {
        return Ok(None);
    }

    // --- STEP 4: Proxy Creation & Import ---
    // We create a "Proxy" by cloning the domain declaration and tagging it
    // with metadata to track its external origin.
    let mut proxy = winner.clone();
    proxy.set_origin(SymbolOrigin::Domain);

    // Track the original Domain ID for cross-referencing.
    proxy.set_alias(winner.source());

    // Link the proxy to the local NodeId that triggered this import.
    proxy.set_source(usage.source());

    // Return the Domain resolution variant encapsulating the proxy and its match status.
    Ok(Some(Resolution::Domain(proxy, status)))
}

/// Attempts to resolve a symbol usage within the local scope context.
///
/// This function performs a local lookup by evaluating shadowing rules and,
/// if necessary, validating the structural and semantic compatibility of
/// the symbol's signature (e.g., for actions or predicates).
///
/// # Arguments
/// * `symbol_entry` - The entry in the symbol table containing all declarations for this identifier.
/// * `usage` - The specific usage site (call site) being resolved.
/// * `checker` - An optional [`SignatureMatcher`] to perform deep semantic validation.
///
/// # Returns
/// * `Ok(Some(Resolution))` - If a valid local declaration is found and matches the usage.
/// * `Ok(None)` - If no compatible local declaration exists.
///
/// # Errors
/// * Returns [`SemanticPassError`] if signature matching fails or AST access errors occur.
pub fn resolve_local_match(
    symbol_entry: &SymbolEntry,
    usage: &Usage,
    checker: Option<&SignatureMatcher>,
) -> Result<Option<Resolution>, SemanticPassError> {
    // --- STEP 1: Shadowing Candidate Lookup ---
    // We search for the best declaration candidate based on SymbolKind and Scope.
    // This handles cases where a local variable shadows a more global one.
    let candidate = find_shadowing_candidate(symbol_entry, usage.symbol().kind(), usage.scope());

    // If no candidate matches the basic scope/kind criteria, resolution fails.
    let Some(winner) = candidate else {
        return Ok(None);
    };

    // --- STEP 2: Atomic Symbol Handling ---
    // For "atomic" kinds (like Types or Constants), we don't need signature matching.
    // Finding the candidate is sufficient for a successful match.
    if is_atomic_kind(winner.symbol_kind()) {
        return Ok(Some(Resolution::Local(winner.source(), MatchResult::Match)));
    }

    // --- STEP 3: Deep Signature Validation ---
    // For complex symbols (Actions, Tasks, Predicates), we must verify that
    // the call site arguments align with the declaration's parameters.
    if let Some(matcher) = checker {
        // Create contextual signatures for both the definition and the call site.
        let arg_signature = Signature::from_usage(winner, usage);
        let decl_signature = Signature::from_declaration(winner);

        // Perform structural and semantic (type-checking) matching.
        let match_res = matcher.match_signature(decl_signature, arg_signature)?;

        // Only return a resolution if the signatures are compatible.
        if match_res.is_match() {
            return Ok(Some(Resolution::Local(winner.source(), match_res)));
        }
    }

    // If a signature check was required but failed or wasn't provided, no match is found.
    Ok(None)
}

/// Represents the intermediate result of a symbol resolution before it is applied to the table.
///
/// This enum acts as a temporary buffer during the "Collection Phase" of the
/// resolve-apply pattern. It captures where a symbol was found and the semantic
/// quality of the match.
///
/// # Variants
/// * `Local`: The symbol was found in the current scope/table. We only need the
///   [`NodeId`] of the declaration and the match status.
/// * `Domain`: The symbol was found in a parent domain. This carries a full
///   [`Declaration`] to allow for proxy generation during the application phase.
enum Resolution {
    /// Found in the current [`SymbolTable`].
    /// Holds the [`NodeId`] of the target declaration and the [`MatchResult`].
    Local(NodeId, MatchResult),

    /// Found in an external/parent [`SymbolTable`] (e.g., Domain for a Problem).
    /// Holds a copy of the [`Declaration`] to facilitate cross-table linking (Proxy).
    Domain(Declaration, MatchResult),
}

impl Resolution {
    /// Returns the semantic match status of this resolution.
    ///
    /// This is used to filter out incomplete matches or handle overloads
    /// during the collection process.
    fn status(&self) -> &MatchResult {
        match self {
            Resolution::Local(_, s) | Resolution::Domain(_, s) => s,
        }
    }
}
