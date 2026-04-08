//! Symbol Resolution Engine for PDDL.
//!
//! This module implements a robust, two-phase resolution system designed to transform
//! raw syntax nodes into a semantic model where every identifier is correctly linked
//! to its declaration (Local, Domain, or Built-in).
//!
//! ## Architecture: The Resolve-Apply Pattern
//!
//! To maintain thread safety and prevent partial state corruption, the resolution
//! process is split into two distinct stages:
//!
//! 1. **Collection Phase**: The AST is traversed immutably. Potential matches are
//!    found and stored in a temporary [`Resolution`] buffer. No changes are made
//!    to the [`SymbolTable`] during this time.
//! 2. **Application Phase**: The collected resolutions are "applied" to the table.
//!    This involves updating bidirectional links between usages and declarations
//!    and managing "Proxy" symbols for cross-file (Domain-to-Problem) references.
//!
//! ## Two-Phase Resolution Strategy
//!
//! Because PDDL symbols have inter-dependencies (e.g., a Predicate's signature depends
//! on the existence of Types), resolution is executed in a strict order:
//!
//! ### Phase 1: Nominal Resolution
//! Resolves symbols whose identity is defined solely by their name:
//! * **Types**: Primitive or user-defined types.
//! * **Objects/Constants**: Individual entities.
//! * **Intrinsics**: Language-level keywords like `object`, `number`, or `#t`.
//!
//! ### Phase 2: Signature Resolution
//! Resolves complex symbols that require structural validation:
//! * **Predicates & Functions**: Validates that argument counts and types match.
//! * **Actions & Tasks**: Ensures hierarchical consistency.
//!
//! This phase is only triggered if Phase 1 succeeds, as structural matching
//! requires valid type information from the [`TypeChecker`].
//!
//! ## Handling of Implicit (Built-in) Symbols
//!
//! PDDL contains reserved symbols that are never explicitly declared by the user
//! (e.g., the variable `?duration` in durative actions). This module handles these
//! via [`Resolution::Implicite`], mapping them to virtual `NodeId` constants
//! within the [`ParseContext`]. This approach prevents "Unused Variable" warnings
//! while allowing the rest of the compiler to treat them as valid symbols.

use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::semantic::passes::error::SemanticPassError;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::rules::{find_shadowing_candidate, is_nominal_kind};
use crate::aiplan4rust::semantic::signature_checker::{MatchResult, SignatureChecker};
use crate::aiplan4rust::semantic::symbol::{
    Declaration, Signature, SymbolEntry, SymbolOrigin, Usage,
};
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::syntax::ParseContext;
use crate::aiplan4rust::tree::NodeId;

/// Performs a two-phase symbol resolution on the provided table.
///
/// This function coordinates the transition from raw syntax nodes to a fully
/// linked semantic model. It follows a strict dependency order:
///
/// ### Phase 1: Nominal Resolution
/// Resolves "leaf" symbols (Types, Objects, Constants) by name identity.
/// These must be resolved first because they form the basis of type signatures
/// used in the next phase.
///
/// ### Phase 2: Signature Resolution
/// Performs structural matching for complex symbols (Predicates, Actions, Functions).
/// This phase requires a [`TypeChecker`] and relies on the successful binding
/// of nominal symbols from Phase 1.
pub fn resolve_symbols(
    context: &PassContext,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
) -> Result<bool, SemanticPassError> {
    // --- PHASE 1: NOMINAL RESOLUTION (Types, Objects, Constants) ---
    // We resolve the "leaves" first. These are required to validate signatures later.
    let (nominal_res, nominal_ok) = collect_nominal_resolutions(table, domain_table)?;

    // Apply nominal resolutions immediately so they are available for Phase 2.
    if !nominal_res.is_empty() {
        apply_resolutions(table, nominal_res);
    }

    // CRITICAL: If nominal resolution failed to resolve everything it should
    // (e.g., an undefined type or object), we stop here.
    // We cannot check signatures if the underlying types are missing or broken.
    if !nominal_ok {
        return Ok(false);
    }

    // --- PHASE 2: SIGNATURE RESOLUTION (Predicates, Actions, Functions) ---
    // Signature matching requires a TypeChecker. If absent, we stop and return
    // the status of Phase 1.
    let Some(tc) = type_checker else {
        return Ok(nominal_ok);
    };

    // Now that nominal symbols are bound, we perform structural signature matching.
    let (signature_res, all_resolved) = {
        let checker = SignatureChecker::new(table, context.syntax_tree(), tc, domain_table);
        collect_signature_resolutions(table, &checker, domain_table)?
    };

    // Apply the final set of structural resolutions.
    if !signature_res.is_empty() {
        apply_resolutions(table, signature_res);
    }

    // Returns true only if both phases were successful.
    Ok(all_resolved)
}

/// Collects resolutions for nominal symbols.
///
/// Nominal symbols (such as Types, Objects, and Constants) are resolved
/// strictly by their identifier (name) rather than their internal structure
/// or parameter signatures.
///
/// # Returns
/// * `Ok((Vec<...>, bool))` - A collection of discovered nominal resolutions
///    and a flag indicating if all nominal symbols were successfully resolved.
/// * `Err(SemanticPassError)` - If a name resolution error occurs.
fn collect_nominal_resolutions(
    table: &SymbolTable,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SemanticPassError> {
    let mut resolutions = Vec::new();
    let mut all_resolved = true;

    for entry in table {
        for usage in entry.usages() {
            let kind = usage.symbol().kind();

            // Process ONLY unresolved symbols that belong to a nominal kind.
            // Nominal kinds typically include entities where identity is defined by name.
            if usage.resolution().is_some() || !is_nominal_kind(kind) {
                continue;
            }

            // 1. Local Priority: Check for parameters, action variables, etc.
            let mut resolution = resolve_local_match(entry, usage, None)?;

            // 2. Domain Fallback: If not found locally, check the Domain (Types, Constants, etc.)
            if resolution.is_none() {
                resolution = resolve_domain_match(entry.id(), domain_table, usage, None)
                    .ok()
                    .flatten();
            }

            // 3. Built-in Fallback: Finally, try resolving PDDL intrinsics (object, #t, ?duration)
            if resolution.is_none() {
                resolution = resolve_builtin(usage.symbol().id());
            }

            // 4. Final Result Handling
            if let Some(res) = resolution {
                // Successfully resolved: could be Local, Domain, or Implicite.
                resolutions.push((usage.symbol().id(), usage.source(), res));
            } else {
                // Mark as unresolved if the identifier is not found in any scope.
                all_resolved = false;
            }
        }
    }

    Ok((resolutions, all_resolved))
}

/// Resolves a PDDL built-in or intrinsic symbol into a virtual [`Resolution`].
///
/// This function handles symbols that are part of the PDDL language specification
/// (e.g., `object`, `#t`, `?duration`) and do not require an explicit declaration
/// in the user's source code.
///
/// # Arguments
///
/// * `symbol_id` - The unique identifier of the symbol to resolve.
///
/// # Returns
///
/// * `Some(Resolution::Implicite)` if the symbol is a recognized PDDL intrinsic,
///   pointing to a virtual [`NodeId`] in the [`ParseContext`].
/// * `None` if the symbol is not a built-in, signaling that it should be resolved
///   via standard scoping rules (Local or Domain).
fn resolve_builtin(symbol_id: SymbolId) -> Option<Resolution> {
    let reserved_id = match symbol_id {
        SymbolInterner::OBJECT_SYMBOL_ID => ParseContext::NODE_ID_OBJECT,
        SymbolInterner::NUMBER_SYMBOL_ID => ParseContext::NODE_ID_NUMBER,
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID => ParseContext::NODE_ID_DURATION,
        SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID => ParseContext::NODE_ID_CONTINUOUS_TIME,
        SymbolInterner::TOTAL_TIME_SYMBOL_ID => ParseContext::NODE_ID_TOTAL_TIME,
        SymbolInterner::TOTAL_COST_SYMBOL_ID => ParseContext::NODE_ID_TOTAL_COST,
        // Return None if the symbol is not a managed built-in.
        _ => return None,
    };

    Some(Resolution::Implicite(reserved_id, MatchResult::Match))
}

/// Collects resolutions for non-atomic symbols by performing signature matching.
///
/// This function identifies call sites (e.g., Predicates, Actions, Functions) that
/// require structural validation against their declarations. It attempts to resolve
/// them first within the local symbol table, then falls back to the domain table.
///
/// # Returns
/// * `Ok((Vec<...>, bool))` - A collection of discovered resolutions and a flag
///    indicating if all encountered non-atomic symbols were successfully resolved.
/// * `Err(SemanticPassError)` - If an error occurs during the matching process.
fn collect_signature_resolutions(
    table: &SymbolTable,
    checker: &SignatureChecker,
    domain_table: Option<&SymbolTable>,
) -> Result<(Vec<(SymbolId, NodeId, Resolution)>, bool), SemanticPassError> {
    let mut resolutions = Vec::new();
    let mut all_resolved = true;

    for entry in table {
        for usage in entry.usages() {
            let kind = usage.symbol().kind();
            let is_nominal = is_nominal_kind(kind);
            let has_resolution = usage.resolution().is_some();

            // Skip symbols that are already resolved or belong to atomic kinds
            // (atomic kinds are handled by nominal resolution passes).
            if has_resolution || is_nominal {
                continue;
            }

            // Perform Signature-based resolution:
            // 1. Attempt to find a matching declaration in the local scope.
            // 2. If not found, attempt to find a matching proxy/declaration in the domain.
            let best_match = resolve_local_match(entry, usage, Some(checker))?.or_else(|| {
                resolve_domain_match(entry.id(), domain_table, usage, Some(checker))
                    .ok()
                    .flatten()
            });

            if let Some(res) = best_match {
                // Record the resolution to be applied later in the 'Apply' phase.
                resolutions.push((usage.symbol().id(), usage.source(), res));
            } else {
                // Mark as unresolved if no valid signature match could be established.
                all_resolved = false;
            }
        }
    }

    Ok((resolutions, all_resolved))
}

/// Applies a list of semantic resolutions to the symbol table.
///
/// This function serves as the "Apply" phase of the resolve-apply pattern.
/// It synchronizes the decisions made during the immutable analysis phase
/// into the mutable [`SymbolTable`].
///
/// # Process
/// 1. **Proxy Fusion**: For symbols originating from a parent Domain, the function
///    checks for existing proxies to maintain idempotency and prevent duplicate
///    declarations within the local scope.
/// 2. **Bidirectional Linking**:
///    - Updates the **Declaration** to register the new usage.
///    - Updates the **Usage** to point to the final declaration and stores
///      the resulting match quality ([`MatchResult`]).
///
/// # Arguments
/// * `table` - The mutable symbol table to be updated.
/// * `resolutions` - A vector of triplets containing the Symbol ID,
///    the Usage Node ID, and the calculated Resolution.
pub fn apply_resolutions(
    table: &mut SymbolTable,
    resolutions: Vec<(SymbolId, NodeId, Resolution)>,
) -> Result<(), SemanticPassError> {
    for (sym_id, usage_id, resolution) in resolutions {
        // 1. DÉTERMINATION DE LA DÉCLARATION CIBLE ET DU STATUT
        let (final_decl_id, status) = match resolution {
            // Cas Local : L'ID de la déclaration existe déjà dans la table
            Resolution::Local(decl_id, status) => (decl_id, status),

            // Cas Domain : Gestion des Proxys (Importation du domaine vers le problème)
            Resolution::Domain(proxy, status) => {
                let proxy_source = proxy.source();

                // On utilise ton nouveau add_declaration.
                // Grâce à sa "Garde d'Idempotence", si le proxy existe déjà,
                // il ne fait rien. C'est ultra-efficace.
                table.add_declaration(sym_id, proxy)?;

                (proxy_source, status)
            }

            // Cas Implicite : Symboles réservés (ex: object, ?duration)
            Resolution::Implicite(reserved_id, status) => (reserved_id, status),
        };

        // 2. MISE À JOUR DES LIENS BIDIRECTIONNELS (Cache O(1))
        // On remplace tout l'ancien bloc 2.1 et 2.2 par ton "vissage" atomique.
        // Cette fonction gère seule l'accès aux index du cache.
        table.link_resolution(usage_id, final_decl_id, status);
    }

    Ok(())
}

/// Searches for an existing domain-originated symbol within the given entry.
///
/// This function is a key part of the "Proxy Pattern" used during the fusion
/// of a Domain and a Problem. It ensures idempotency by preventing the
/// creation of duplicate proxy declarations for the same external symbol.
///
/// # Arguments
/// * `entry` - The specific symbol table entry to inspect.
/// * `proxy_source` - The [`NodeId`] of the original declaration in the Domain's AST.
///
/// # Returns
/// * `Some(NodeId)` - The ID of the existing proxy if a match is found.
/// * `None` - If this domain symbol has not been imported into the local table yet.
fn find_domain_proxy(entry: &SymbolEntry, proxy_source: NodeId) -> Option<NodeId> {
    // Iterate through all existing declarations for this symbol name
    for declaration in entry.declarations() {
        // A duplicate is identified if:
        // 1. The declaration's origin is the Domain (it's a Proxy).
        // 2. The source NodeId matches the one we are trying to resolve.
        if declaration.origin() == SymbolOrigin::Domain && declaration.source() == proxy_source {
            return Some(declaration.source());
        }
    }

    // No matching proxy found
    None
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
/// * `checker` - An optional [`SignatureChecker`] for deep semantic validation.
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
    checker: Option<&SignatureChecker>,
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
    let status = if is_nominal_kind(winner.symbol_kind()) {
        // Atomic symbols (Types/Constants) are considered a match upon identification.
        MatchResult::Match
    } else {
        let expected = Signature::from_declaration(winner);
        let observed = Signature::from_usage(winner, usage);

        match checker {
            // Full semantic check if a matcher is available.
            Some(m) => m.match_signature(expected, observed)?,
            // Fallback to basic structural check (arity/kind) if no matcher is provided.
            None => match SignatureChecker::match_structure(expected, observed) {
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
/// * `checker` - An optional [`SignatureChecker`] to perform deep semantic validation.
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
    checker: Option<&SignatureChecker>,
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
    if is_nominal_kind(winner.symbol_kind()) {
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
enum Resolution {
    /// Found in the current [`SymbolTable`].
    /// Holds the [`NodeId`] of the target declaration and the [`MatchResult`].
    Local(NodeId, MatchResult),

    /// Found in an external/parent [`SymbolTable`] (e.g., Domain for a Problem).
    /// Holds a copy of the [`Declaration`] to facilitate cross-table linking (Proxy).
    Domain(Declaration, MatchResult),

    /// Found as a built-in or intrinsic symbol (e.g., `object`, `number`, `#t`).
    /// These symbols are valid but do not have a physical [`Declaration`] in the
    /// user's PDDL files. They point to virtual nodes in the [`ParseContext`].
    Implicite(NodeId, MatchResult),
}

impl Resolution {
    /// Returns the semantic match status of this resolution.
    fn status(&self) -> &MatchResult {
        match self {
            Resolution::Local(_, s) | Resolution::Domain(_, s) | Resolution::Implicite(_, s) => s,
        }
    }
}
