use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::semantic::passes::type_simplification::TypeSimplification;
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::type_checker::{TypeCheckerError, TypeHierarchy};
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::tree::NodeId;
use crate::{DiagnosticManager, SymbolTable};

/// The maximum number of members allowed in a type union for optimized simplification.
/// This limit is defined by the size of the bitmask (u128) used in the algorithm.
const MAX_UNION_SIMPLIFICATION_CAPACITY: usize = 128;

/// Simplifies union types (e.g., `either`) across all entries in a [`SymbolTable`].
///
/// This function identifies and removes redundant types within type unions based on the
/// hierarchy provided by the [`TypeChecker`]. For example, if a symbol is typed as
/// `(either dog animal)` and `dog` is a subtype of `animal`, it simplifies the type
/// to just `dog`.
///
/// # Architecture: Two-Phase Mutation
///
/// To comply with Rust's strict borrowing rules, this process is split into two
/// distinct phases:
/// 1. **Collection (Immutable)**: Iterates over the `target_table` to identify needed
///    changes. The `type_checker` is used for read-only hierarchy lookups, avoiding
///    any borrow conflicts with the table.
/// 2. **Application (Mutable)**: Applies the collected changes to the `target_table`.
///
/// This design allows you to simplify the same table that was used to build the
/// [`TypeHierarchy`] without hitting `E0502` (immutable/mutable borrow conflict).
///
/// # Arguments
///
/// * `type_checker` - The reference hierarchy used to resolve type relationships.
/// * `target_table` - The mutable symbol table to be optimized.
///
/// # Errors
///
/// Returns a [`TypeCheckerError`] if:
/// - A type union exceeds the internal bitmask capacity ([`MAX_UNION_SIMPLIFICATION_CAPACITY`]).
/// - A type in a union cannot be resolved within the current hierarchy.
///
/// # Performance
///
/// This is an $O(N)$ operation where $N$ is the total number of declarations.
/// The use of a stack-allocated bitmask and the `TypeChecker`'s internal
/// transitive closure cache makes this highly efficient even for massive domains.
pub fn finalize(
    context: &PassContext,
    type_checker: &TypeChecker,
    target_table: &mut SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<Vec<TypeSimplification>, TypeCheckerError> {
    // Étape 1 : Collecte (Phase Immuable)
    // On récupère une Box<[TypeSimplification]> (taille fixe, immuable)
    let changes =
        collect_type_simplifications(context, type_checker, target_table, diagnostic_manager)?;

    // Étape 2 : Application (Phase Mutable)
    if !changes.is_empty() {
        // On passe une RÉFÉRENCE car on veut garder 'changes' pour le retour.
        // Cela implique des .clone() internes dans apply_type_simplifications.
        apply_type_simplifications(target_table, &changes);
    }

    // On retourne la liste des changements pour le reste du pipeline
    Ok(changes)
}

/// Scans the provided [`SymbolTable`] to identify declarations that can be simplified.
///
/// This is the first phase of the simplification process. It performs a read-only
/// traversal of the table, comparing each type union against the established
/// hierarchy via the [`TypeChecker`].
///
/// # Process
/// For every declaration in the table, it checks if the associated type is a union
/// (e.g., `either`). If [`simplify_type`] returns a more concise version
/// (by removing ancestors), a [`TypeSimplification`] instruction is recorded.
///
/// # Performance
/// This function is highly efficient because:
/// 1. It operates in **read-only** mode on both the table and the checker,
///    allowing for optimal memory access patterns.
/// 2. It leverages the `type_closure_cache` within the `type_checker`, ensuring
///    that hierarchy lookups (transitive closures) are only computed once per type.
///
/// # Arguments
///
/// * `type_checker` - The reference hierarchy used to resolve type relationships.
/// * `target_table` - The symbol table to scan for redundant types.
///
/// # Returns
/// - `Ok(Vec<TypeSimplification>)`: A list of targeted update instructions to be
///   applied in the second phase.
/// - `Err(TypeCheckError)`: If a type resolution fails or exceeds simplification limits.
fn collect_type_simplifications(
    context: &PassContext,
    type_checker: &TypeChecker,
    target_table: &SymbolTable,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<Vec<TypeSimplification>, TypeCheckerError> {
    let mut changes = Vec::new();

    for (&symbol_id, entry) in target_table.iter() {
        for decl in entry.declarations().values() {
            if let Some(raw_ty) = decl.ty() {
                // Pass the type_checker to utilize its internal cache
                if let Some((new_type, kept_indices)) = simplify_type(type_checker, raw_ty)? {
                    changes.push(TypeSimplification::new(
                        symbol_id,
                        decl.source(),
                        new_type.clone(),
                        kept_indices,
                    ));

                    let warning = Diagnostic::warning_redundant_type_union(
                        symbol_id,
                        raw_ty.clone(),
                        new_type,
                        context.provider(),
                        context.source(),
                        decl.span(),
                    );

                    diagnostic_manager.add_diagnostic(warning);
                }
            }
        }
    }
    Ok(changes)
}

/// Simplifies a type union by removing redundant super-types.
///
/// If an 'either' type contains both a type and its ancestor (e.g., `satellite` and `object`),
/// the ancestor is considered redundant and is removed.
///
/// # Returns
///
/// * `Ok(Some((Type, Vec<usize>)))` - A simplified version of the type and the
///   original indices of the kept members.
/// * `Ok(None)` - If the type was already optimal (no changes needed).
/// * `Err(TypeCheckError)` - If the union exceeds [`MAX_UNION_SIMPLIFICATION_CAPACITY`]
///   members or if type resolution fails.
fn simplify_type(
    type_checker: &TypeChecker,
    ty: &Type<SymbolId>,
) -> Result<Option<(Type<SymbolId>, Vec<usize>)>, TypeCheckerError> {
    let members = ty.members();
    let n = members.len();
    if n <= 1 {
        return Ok(None);
    }

    let mut to_remove_mask: u128 = 0;
    let mut changed = false;

    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let closure = type_checker.ascending_type_closure(members[j])?;
            if closure.contains(&members[i]) {
                to_remove_mask |= 1 << i;
                changed = true;
                break;
            }
        }
    }

    if !changed {
        return Ok(None);
    }

    let mut simplified_ids = Vec::new();
    let mut kept_indices = Vec::new();
    for i in 0..n {
        if (to_remove_mask & (1 << i)) == 0 {
            simplified_ids.push(members[i]);
            kept_indices.push(i); // On stocke l'index d'origine
        }
    }

    Ok(Some((Type::from(simplified_ids), kept_indices)))
}

/// Applies the collected type simplifications to the [`SymbolTable`].
///
/// This is the second phase of the simplification process. It is designed as a
/// standalone function that only requires mutable access to the target table,
/// as all necessary transformation data is already contained within the
/// [`TypeSimplification`] instructions.
///
/// # Implementation Details: The `take` Pattern
///
/// Symbol declarations are typically stored in a `HashSet` to ensure uniqueness.
/// In Rust, you cannot mutate an element in place if it is part of a set's
/// identity (hash). To safely update a declaration, this function uses the
/// `take` pattern:
/// 1. **Extract**: Remove the original declaration from the set using `take(&key)`.
/// 2. **Update**: Modify the type metadata and synchronize the `NodeId` list
///    using the `kept_indices`.
/// 3. **Re-insert**: Put the updated declaration back into the set.
///
/// This ensures the internal integrity of the [`SymbolTable`] indices remains intact.
///
/// # Arguments
///
/// * `target_table` - The mutable symbol table where types and NodeIds will be updated.
/// * `changes` - A vector of [`TypeSimplification`] instructions generated during
///   the collection phase.
pub fn apply_type_simplifications(
    target_table: &mut SymbolTable,
    changes: &[TypeSimplification], // Référence : on ne consomme plus le Vec
) {
    for change in changes {
        // On utilise les accesseurs car on n'a qu'une vue en lecture seule
        if let Some(entry) = target_table.get_symbol_mut(change.symbol_id()) {
            if let Some(original) = entry.declarations_mut().get_mut(&change.node_id()) {
                // 1. Synchronisation des NodeIds
                if let Some(old_ids) = original.type_sources() {
                    // On itère sur les indices (copie d'entiers, donc pas de clone lourd)
                    let new_ids: Vec<NodeId> = change
                        .kept_indices()
                        .iter()
                        .filter_map(|&i| old_ids.get(i))
                        .copied()
                        .collect();

                    original.set_type_sources(new_ids);
                }

                // 2. Mise à jour du type sémantique
                // CLONE OBLIGATOIRE : On duplique le type pour l'insérer dans la table
                // tout en laissant l'original dans la liste des 'changes'.
                original.set_ty(change.new_type().clone());
            }
        }
    }
}
