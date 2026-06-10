//! Derived Predicate Resolution Pass
//!
//! This module implements the analysis pass responsible for linking PDDL axioms
//! (derived predicates) to their corresponding base predicate declarations.
//!
//! ## Overview
//! In PDDL, a derived predicate provides a logic-based definition for a predicate
//! declared in the `:predicates` section. While standard symbols are resolved by
//! name, derived predicates require a secondary pass to:
//! 1. Verify that the axiom's signature matches a formal declaration.
//! 2. Establish bidirectional pointers for efficient state evaluation.
//!
//! ## The Two-Phase Approach
//! To satisfy Rust's strict borrowing rules (Ownership/Borrowing), the resolution
//! is split into two distinct phases:
//!
//! 1. **Collection Phase**: The [`SymbolTable`] is scanned immutably to identify
//!    valid matches. These matches are stored as [`DerivedLink`] objects.
//! 2. **Application Phase**: The identified links are applied mutably to the
//!    [`SymbolTable`], "wiring" the declarations together.
//!
//! ## Technical Details
//! An axiom is considered a match if it shares the same symbol name and its
//! argument types are compatible with the base declaration, as determined by
//! the [`SignatureChecker`].

use crate::aiplan4rust::compiler::semantic::passes::context::PassContext;
use crate::aiplan4rust::compiler::semantic::passes::SemanticPassError;
use crate::aiplan4rust::compiler::semantic::signature_checker::{MatchResult, SignatureChecker};
use crate::aiplan4rust::compiler::semantic::symbol::Signature;
use crate::aiplan4rust::compiler::semantic::TypeChecker;
use crate::aiplan4rust::compiler::syntax::ast::tree::NodeId;
use crate::aiplan4rust::support::lang::SymbolId;
use crate::SymbolTable;

/// Resolves relationships between derived predicates (axioms) and their base declarations.
///
/// This function orchestrates the third phase of symbol resolution. It identifies which
/// `:derived` axioms correspond to which `:predicates` by matching their signatures
/// (names and argument types).
///
/// # Workflow
/// 1. **Context Access**: Utilizes the [`PassContext`] to access the immutable AST and
///    metadata without redundant parameter passing.
/// 2. **Validation**: Ensures a [`TypeChecker`] is available to perform semantic matching.
/// 3. **Collection Phase**: Performs an immutable pass over the [`SymbolTable`] to identify
///    valid base-to-axiom pairs using a [`SignatureChecker`].
/// 4. **Application Phase**: Performs a mutable pass to "wire" these links back into
///    the table, establishing bidirectional pointers.
///
/// # Arguments
/// * `context` - The shared, immutable compilation context containing the AST and interner.
/// * `table` - The mutable symbol table where resolutions are recorded.
/// * `type_checker` - Optional hierarchy manager; required for signature matching.
/// * `domain_table` - Optional parent domain table for requirements or external references.
///
/// # Errors
/// Returns [`SemanticPassError`] if signature matching fails or encounters structural
/// inconsistencies.
pub fn resolve_derived_predicates(
    context: &PassContext,
    table: &mut SymbolTable,
    type_checker: Option<&TypeChecker>,
    domain_table: Option<&SymbolTable>,
) -> Result<(), SemanticPassError> {
    // 1. Safety: Signature matching requires a TypeChecker for type-compatible resolution.
    let Some(tc) = type_checker else {
        return Ok(());
    };

    // 2. COLLECTION PHASE (Immutable)
    // We create the matcher and scan the table to find all valid links.
    // The scoped block ensures the immutable borrow of `table` is released
    // before the mutation phase begins.
    let derived_links = {
        let matcher = SignatureChecker::new(table, context.syntax_tree(), tc, domain_table);
        collect_derived_links(table, &matcher)?
    };

    // 3. APPLICATION PHASE (Mutable)
    // Only proceed to mutation if at least one valid link was discovered.
    if !derived_links.is_empty() {
        apply_derived_links(table, derived_links)?;
    }

    Ok(())
}

/// Collects all valid links between derived axioms and their base predicate declarations.
///
/// This function iterates through the symbol table to find matching signatures
/// between formal predicate definitions ("Bases") and axiom implementations ("Derived").
///
/// # Returns
/// A vector of [`DerivedLink`] objects representing validated connections.
///
/// # Errors
/// Returns [`SemanticPassError`] if signature matching fails during the process.
fn collect_derived_links(
    table: &SymbolTable,
    matcher: &SignatureChecker,
) -> Result<Vec<DerivedLink>, SemanticPassError> {
    let mut global_links = Vec::new();

    // OPTIMIZATION: Pre-allocate worklists outside the loop to reuse memory buffers.
    // This prevents frequent heap allocations/deallocations during table iteration.
    let mut bases_worklist = Vec::with_capacity(8);
    let mut derived_worklist = Vec::with_capacity(8);

    for entry in table {
        // Clear lists for the new symbol while keeping the allocated capacity.
        bases_worklist.clear();
        derived_worklist.clear();

        // --- STEP 1: Categorize and Pre-compute ---
        // We separate declarations and compute base signatures only once.
        for decl in entry.declarations() {
            if decl.is_derived() {
                derived_worklist.push((decl.source(), decl));
            } else {
                // OPTIMIZATION: Extract and old the signature once for each base predicate.
                let sig = Signature::from_declaration(decl);
                bases_worklist.push((decl.source(), sig));
            }
        }

        // Skip if either side is missing, as no link can be established.
        if bases_worklist.is_empty() || derived_worklist.is_empty() {
            continue;
        }

        // --- STEP 2: Signature Matching (Cross-product) ---
        for (d_id, d_decl) in &derived_worklist {
            // Compute the axiom signature once per axiom implementation.
            let provided = Signature::from_declaration(d_decl);

            for (b_id, expected_sig) in &bases_worklist {
                // Compare pre-computed signatures using the semantic matcher.
                // Note: Ensure matcher.match_signature handles clones or takes references.
                let match_result = matcher.match_signature(*expected_sig, provided)?;

                if let MatchResult::Match = match_result {
                    // Record the validated resolution link.
                    global_links.push(DerivedLink::new(entry.id(), *b_id, *d_id));

                    // In PDDL, an axiom typically resolves to a single formal signature.
                    break;
                }
            }
        }
    }

    Ok(global_links)
}

/// Materializes the links between derived axioms and their base predicate declarations.
///
/// This function performs the final "wiring" in the symbol table by establishing
/// bidirectional relationships between axioms (derived definitions) and the
/// original predicates they implement.
///
/// # Arguments
/// * `table` - The mutable [`SymbolTable`] where the links will be applied.
/// * `links` - A vector of [`DerivedLink`] objects representing validated connections.
///
/// # Implementation Details
/// For each link, the function:
/// 1. Locates the [`SymbolEntry`] corresponding to the `symbol_id`.
/// 2. Updates the **Axiom** declaration to point to its parent (base) declaration.
/// 3. Updates the **Base** declaration to include the axiom in its list of derivations.
pub fn apply_derived_links(
    table: &mut SymbolTable,
    links: Vec<DerivedLink>,
) -> Result<(), SemanticPassError> {
    for link in links {
        let axiom_id = link.axiom_id();
        let base_id = link.base_id();

        // 1. Link l'Axiome vers sa Signature parente (Upward link)
        // On utilise directement le NodeId de l'axiome via le cache
        if let Ok(axiom) = table.try_get_declaration_mut(axiom_id) {
            axiom.set_derived_source(base_id);
        }

        // 2. Link la Signature de base vers son Axiome enfant (Downward link)
        // On utilise directement le NodeId de la base via le cache
        if let Ok(base) = table.try_get_declaration_mut(base_id) {
            base.add_derivation(axiom_id);
        }
    }
    Ok(())
}

/// Represents the result of resolving a derived predicate.
///
/// It stores the connection between an axiom implementation and its
/// corresponding base predicate declaration.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct DerivedLink {
    /// The unique identifier of the symbol (e.g., 'at').
    symbol_id: SymbolId,
    /// The NodeId of the formal predicate declaration (The 'Base').
    base_id: NodeId,
    /// The NodeId of the derived axiom implementation (The 'Axiom').
    axiom_id: NodeId,
}

impl DerivedLink {
    /// Creates a new resolution link between a base predicate and its axiom.
    ///
    /// # Arguments
    /// * `symbol_id` - The identifier of the symbol (e.g., the interner ID for "at").
    /// * `base_id` - The `NodeId` of the predicate's formal declaration.
    /// * `axiom_id` - The `NodeId` of the derived axiom's implementation.
    pub fn new(symbol_id: SymbolId, base_id: NodeId, axiom_id: NodeId) -> Self {
        Self {
            symbol_id,
            base_id,
            axiom_id,
        }
    }
    /// Returns the unique identifier of the symbol.
    #[allow(dead_code)]
    pub fn symbol_id(&self) -> SymbolId {
        self.symbol_id
    }

    /// Returns the NodeId of the formal predicate declaration (the "Base").
    pub fn base_id(&self) -> NodeId {
        self.base_id
    }

    /// Returns the NodeId of the derived axiom implementation (the "Axiom").
    pub fn axiom_id(&self) -> NodeId {
        self.axiom_id
    }
}
