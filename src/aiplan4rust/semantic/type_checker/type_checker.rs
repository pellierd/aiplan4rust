//! Provides semantic typing checking for domain-specific symbols.
//!
//! This module defines the [`TypeChecker`] struct and its associated ops used for validating typing
//! relationships in PDDL-like languages. It ensures that typing declarations and usages are consistent,
//! and computes typing hierarchies such as subtype and supertype relations.
//!
//! # Overview
//!
//! The typing checking ops relies on a [`SymbolTable`] containing typing declarations and their
//! hierarchical relationships. The [`TypeChecker`] uses this information to answer questions like:
//!
//! - Is typing `A` a subtype of `B`?
//! - Do types `A` and `B` share a common supertype?
//! - What is the transitive closure of supertypes for a given typing?
//!
//! This is useful in validating domain semantics and ensuring typing correctness across
//! predicates, actions, and object declarations.
//!
//! # Key Components
//!
//! - [`TypeChecker`] — The main struct performing the actual typing hierarchy analysis.
//! - [`TypeCheckError`] — Error typing used when typing resolution fails unexpectedly.
//!
//! # Features
//!
//! - Type hierarchy traversal via `ascending_type_closure`.
//! - Built-in PDDL typing recognition (e.g., `object`, `number`).
//! - Caching of computed typing closures to improve performance.
//!
//! # Example
//!
//! ```rust,no_run
//! use aiplan4rust::semantic::type_checker::TypeChecker;
//! use aiplan4rust::semantic::symbol_table::SymbolTable;
//!
//! let symbol_table = SymbolTable::new();
//! let checker = TypeChecker::new(&symbol_table);
//!
//! let ty1 = vec!["Animal".into()];
//! let ty2 = vec!["Dog".into()];
//! let result = checker.is_any_subtype_of(&ty1, &ty2)?;
//! assert!(result);
//! ```
//!
//! # See Also
//!
//! - [`SymbolTable`](crate::aiplan4rust::semantic::symbol_table::SymbolTable)
//! - [`SymbolKind`](crate::aiplan4rust::semantic::symbol::SymbolKind)
//! - [`Ident`](crate::aiplan4rust::lang::SymbolId)
//! - [`Type`](crate::aiplan4rust::lang::Type)
//!
//! # Notes
//!
//! This module assumes that the symbol table has been fully populated before typing checking.

use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::SymbolId;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::type_checker::{TypeCheckError, TypeHierarchy};

use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};

/// PDDL Built-in symbols.
const PDDL_BUILTIN_TYPES: [SymbolId; 1] = [
    //SymbolInterner::OBJECT_SYMBOL_ID,
    SymbolInterner::NUMBER_SYMBOL_ID,
];

/// A struct for performing typing checking within a given domain.
///
/// The `TypeChecker` is responsible for verifying subtype and supertype relationships between
/// types defined in a domain. It works in conjunction with a [`SymbolTable`] that holds symbol
/// declarations and typing definitions, and provides utilities for validating typing compatibility
/// between objects, actions, and predicates.
///
/// # Fields
///
/// * `domain_symbol_table` - A reference to the [`SymbolTable`] containing the domain's symbol
///   declarations and typing hierarchy.
/// * `type_closure_cache` - A cache for storing computed transitive closures of supertypes
///   for efficient repeated lookups.
///
/// # Derives
///
/// The `TypeChecker` struct derives the following traits:
/// - [`Debug`]: Enables formatting for debugging.
/// - [`Clone`]: Allows cloning instances while sharing the same symbol table reference.
///
/// # Example
/// ```rust
/// use aiplan4rust::semantic::type_checker::TypeChecker;
/// use aiplan4rust::semantic::symbol_table::SymbolTable;
///
/// let symbol_table = SymbolTable::new();
/// let checker = TypeChecker::new(&symbol_table);
/// let result = checker.is_any_subtype_of(&vec!["Vehicle".into()], &vec!["Car".into()]);
/// assert_eq!(result.unwrap(), true);
/// ```
#[derive(Debug, Clone)]
pub struct TypeChecker<'a> {
    hierarchy: &'a TypeHierarchy,
    type_closure_cache: RefCell<HashMap<SymbolId, HashSet<SymbolId>>>,
}

impl<'a> TypeChecker<'a> {
    /// Creates a new `TypeChecker` instance using the given domain symbol table.
    ///
    /// The symbol table provides access to all typing declarations and their relationships.
    /// This is essential for checking subtyping relationships, common supertypes, and typing closure.
    ///
    /// # Arguments
    ///
    /// * `domain_symbol_table` - A reference to the symbol table that defines the domain's typing hierarchy.
    ///
    /// # Returns
    ///
    /// A new instance of `TypeChecker` with caching enabled for transitive typing closure.
    pub fn new(hierarchy: &'a TypeHierarchy) -> Self {
        TypeChecker {
            hierarchy,
            type_closure_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Checks if any type in the second set (`ty2`) is a subtype of any type in the first set (`ty1`).
    ///
    /// This method evaluates the subtyping relationship by traversing the type hierarchy
    /// defined in the underlying Arena. It returns `true` if there is at least one pair (t2, t1)
    /// such that t2 is a descendant of t1 or t2 == t1.
    ///
    /// # Performance
    /// - **Fast Path**: Performs an O(N*M) direct comparison to catch identical types without
    ///   traversing the hierarchy or hitting the cache.
    /// - **Slow Path**: Uses the memoized `ascending_type_closure` to check for ancestral
    ///   relationships. Since closures are cached in the `TypeChecker`, repeated calls
    ///   for the same `SymbolId` are highly efficient.
    ///
    /// # Arguments
    /// * `ty1` - The set of potential supertypes (e.g., required types).
    /// * `ty2` - The set of potential subtypes (e.g., provided object types).
    ///
    /// # Errors
    /// Returns [`TypeCheckError`] if a `SymbolId` cannot be resolved within the hierarchy.
    pub fn is_any_subtype_of(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckError> {
        // 1. Fast path: Direct overlap check.
        // If ty2 contains an element present in ty1, it's an immediate match (reflexivity).
        // For small unions (common in PDDL), this linear scan is faster than hashing.
        for t2 in ty2.iter() {
            if ty1.iter().any(|t1| t1 == t2) {
                return Ok(true);
            }
        }

        // 2. Slow path: Hierarchical traversal.
        // We check if any ancestor of t2 (from the Arena) matches any type in ty1.
        for t2 in ty2.iter() {
            // Retrieve the transitive closure of supertypes (includes t2 itself).
            // This is O(1) if the result is already in the RefCell cache.
            let closure = self.ascending_type_closure(*t2)?;

            // Check if any required type t1 is an ancestor of the provided type t2.
            for t1 in ty1.iter() {
                if closure.contains(t1) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Returns `true` if any typing in `ty1` is a supertype of any typing in `ty2`.
    ///
    /// This is equivalent to checking whether any typing in `ty2` is a subtype of any typing in `ty1`.
    ///
    /// # Arguments
    ///
    /// * `ty1` - Set of potential supertypes.
    /// * `ty2` - Set of potential subtypes.
    ///
    /// # Returns
    ///
    /// Same as [`is_any_subtype_of`] but with arguments reversed.
    pub fn is_any_supertype_of(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckError> {
        self.is_any_subtype_of(ty2, ty1)
    }

    /// Returns `true` if any typing in `ty1` is a subtype or supertype of any typing in `ty2`.
    ///
    /// Combines both [`is_any_subtype_of`] and [`is_any_supertype_of`] checks.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if at least one relationship exists.
    /// * `Ok(false)` otherwise.
    /// * `Err(TypeCheckError)` if an error occurs in typing resolution.
    pub fn is_any_sub_or_supertype_of(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckError> {
        Ok(self.is_any_subtype_of(ty1, ty2)? || self.is_any_supertype_of(ty1, ty2)?)
    }

    /// Returns `true` if two sets of types share at least one common supertype.
    ///
    /// In PDDL semantics, an empty typing set is unconstrained and represents the root
    /// `object` typing. Therefore, if either set is empty, they are considered to share
    /// the universal root supertype.
    ///
    /// # Optimization
    ///
    /// This function implements a fast path for identical typing sets and unconstrained
    /// types before computing the full transitive closure of supertypes.
    ///
    /// # Arguments
    ///
    /// * `ty1` - The first set of types to check.
    /// * `ty2` - The second set of types to check.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if:
    ///     - Both sets are identical.
    ///     - Either set is empty (representing the universal `object` typing).
    ///     - An intersection is found between their respective transitive supertype closures.
    /// * `Ok(false)` if the typing hierarchies are strictly disjoint.
    /// * `Err(TypeCheckError)` if a typing identifier cannot be resolved in the symbol table.
    pub fn have_common_supertype(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckError> {
        // 1. Fast path: Direct equality or unconstrained types.
        // In PDDL, an empty typing list represents the root 'object' typing,
        // which is the universal supertype for all other types.
        if ty1 == ty2 || ty1.is_root() || ty2.is_root() {
            return Ok(true);
        }

        // 2. Comprehensive check for cross-hierarchy relationships.
        // Compute the transitive closure of all supertypes for the first typing set.
        let mut supertypes1 = HashSet::new();
        for t1 in ty1.iter() {
            // ascending_type_closure includes the typing itself.
            supertypes1.extend(self.ascending_type_closure(*t1)?.iter().cloned());
        }

        // Check if any supertype of the second typing set intersects with the first one.
        for t2 in ty2.iter() {
            if self
                .ascending_type_closure(*t2)?
                .iter()
                .any(|t| supertypes1.contains(t))
            {
                // A common ancestor was found in the hierarchy.
                return Ok(true);
            }
        }

        // No shared supertype found; the types belong to disjoint branches.
        Ok(false)
    }

    /// Returns all supertypes (including itself) of the given typing.
    ///
    /// Performs a depth-first traversal of the typing hierarchy starting from
    /// the provided primitive typing, and collects all supertypes.
    ///
    /// # Arguments
    ///
    /// * `primitive_type` - The base typing from which the closure is computed.
    ///
    /// # Returns
    ///
    /// * `Ok(Ref<HashSet<Ident>>)` containing the closure.
    /// * `Err(TypeCheckError)` if typing resolution fails.
    ///
    /// # Caching
    ///
    /// If the closure for the typing has already been computed, the cached result is reused.
    pub fn ascending_type_closure(
        &self,
        primitive_type: SymbolId,
    ) -> Result<Ref<HashSet<SymbolId>>, TypeCheckError> {
        {
            let cache_ref = self.type_closure_cache.borrow();
            if cache_ref.contains_key(&primitive_type) {
                return Ok(Ref::map(cache_ref, |c| c.get(&primitive_type).unwrap()));
            }
        }

        let mut super_types = HashSet::new();
        let mut to_visit = vec![primitive_type];

        while let Some(current) = to_visit.pop() {
            if !super_types.insert(current) {
                continue;
            }

            if TypeChecker::is_pddl_builtin_types(current) {
                continue;
            }

            let parents = self.hierarchy.get_parents(current);
            to_visit.extend(parents.iter().cloned());
        }

        self.type_closure_cache
            .borrow_mut()
            .insert(primitive_type, super_types);

        let cache_ref = self.type_closure_cache.borrow();
        Ok(Ref::map(cache_ref, |c| c.get(&primitive_type).unwrap()))
    }

    /// Checks if the given typing is a built-in PDDL typing.
    ///
    /// Built-in types are terminal in the typing hierarchy and should not be resolved
    /// or traversed further during closure computation.
    ///
    /// # Arguments
    ///
    /// * `types` - The identifier to check.
    ///
    /// # Returns
    ///
    /// * `true` if the typing is built-in.
    /// * `false` otherwise.
    pub fn is_pddl_builtin_types(ty: SymbolId) -> bool {
        PDDL_BUILTIN_TYPES.contains(&ty)
    }
}
/*/// Simplifies a type union by removing redundant super-types.
    ///
    /// If an 'either' type contains both a type and its ancestor (e.g., `satellite` and `object`),
    /// the ancestor is redundant and removed.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Type))` - A new simplified version of the type if redundancies were found.
    /// * `Ok(None)` - If the type was already optimal (no changes needed).
    /// * `Err(TypeCheckError)` - If the union exceeds [`MAX_UNION_SIMPLIFICATION_CAPACITY`]
    ///   members or if type resolution fails.
    pub fn simplify_type(
        &self,
        ty: &Type<SymbolId>,
    ) -> Result<Option<(Type<SymbolId>, Vec<usize>)>, TypeCheckError> {
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
                let closure = self.ascending_type_closure(members[j])?;
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

    /// Simplifies union types (e.g., `either`) across all entries in a [`SymbolTable`].
    ///
    /// This method identifies and removes redundant types within type unions based on the
    /// current hierarchy. For example, if a symbol is typed as `(either dog animal)` and
    /// `dog` is a subtype of `animal`, it simplifies the type to just `dog`.
    ///
    /// # Architecture: Two-Phase Mutation
    ///
    /// To comply with Rust's borrowing rules, this process is split into two distinct phases:
    /// 1. **Collection (Immutable)**: Iterates over the `target_table` to identify needed
    ///    changes. Since `self` (the hierarchy) is only read, no borrow conflicts occur.
    /// 2. **Application (Mutable)**: Applies the collected changes to the `target_table`.
    ///
    /// This design allows you to simplify the same table that was used to build the
    /// [`TypeHierarchy`] without hitting `E0502` (immutable/mutable borrow conflict).
    ///
    /// # Arguments
    ///
    /// * `target_table` - The mutable symbol table to be optimized.
    ///
    /// # Errors
    ///
    /// Returns a [`TypeCheckError`] if:
    /// - A type union exceeds the internal bitmask capacity ([`MAX_UNION_SIMPLIFICATION_CAPACITY`]).
    /// - A type in a union cannot be resolved within the current hierarchy.
    ///
    /// # Performance
    ///
    /// This is an $O(N)$ operation where $N$ is the number of symbols. The use of a
    /// stack-allocated bitmask and the internal transitive closure cache makes
    /// this highly efficient even for large domains.
    pub fn simplify_symbol_table(
        &self,
        target_table: &mut SymbolTable,
    ) -> Result<(), TypeCheckError> {
        // Step 1: Scan and collect (Immutable phase)
        // No conflict here: self (hierarchy) is used to read, target_table is used to scan.
        let changes = self.collect_type_simplifications(target_table)?;

        // Step 2: Apply changes (Mutable phase)
        // We can pass target_table as &mut because 'changes' owns its data.
        if !changes.is_empty() {
            Self::apply_type_simplifications(target_table, changes);
        }

        Ok(())
    }

    /// Scans the provided [`SymbolTable`] to identify declarations that can be simplified.
    ///
    /// This is the first phase of the simplification process. It performs a read-only
    /// traversal of the table, comparing each type union against the established
    /// [`TypeHierarchy`].
    ///
    /// # Process
    /// For every declaration in the table, it checks if the associated type is a union
    /// (e.g., `either`). If [`Self::simplify_type`] returns a more concise version
    /// (by removing ancestors), a [`TypeSimplification`] instruction is recorded.
    ///
    /// # Performance
    /// This method is highly efficient because:
    /// 1. It operates in **read-only** mode, allowing the CPU to optimize memory access.
    /// 2. It leverages the `type_closure_cache` within `self`, ensuring that hierarchy
    ///    lookups are only computed once per type.
    ///
    /// # Returns
    /// - `Ok(Vec<TypeSimplification>)`: A list of targeted updates to apply later.
    /// - `Err(TypeCheckError)`: If a type resolution fails or exceeds simplification limits.
    fn collect_type_simplifications(
        &self,
        target_table: &SymbolTable,
    ) -> Result<Vec<TypeSimplification>, TypeCheckError> {
        let mut changes = Vec::new();

        for (&symbol_id, entry) in target_table.iter() {
            for decl in entry.declarations().iter() {
                if let Some(raw_ty) = decl.ty() {
                    if let Some((new_type, kept_indices)) = self.simplify_type(raw_ty)? {
                        changes.push(TypeSimplification {
                            symbol_id,
                            node_id: decl.node_id(),
                            new_type,
                            kept_indices,
                        });
                    }
                }
            }
        }
        Ok(changes)
    }

    /// Applies the collected type simplifications to the [`SymbolTable`].
    ///
    /// This is the second phase of the simplification process. It is defined as a
    /// static method because it only requires mutable access to the target table
    /// and does not need to borrow the `TypeChecker` or its hierarchy.
    ///
    /// # Implementation Details: The `take` Pattern
    /// To update a declaration within a `HashSet` (which is typically used for
    /// symbol declarations), we cannot mutate the element in place if the change
    /// affects the hash. By using `declarations_mut().take(&key)`, we:
    /// 1. Remove the original declaration from the set.
    /// 2. Update its type metadata.
    /// 3. Re-insert the updated version.
    ///
    /// This ensures the internal integrity of the `SymbolTable` and its indices.
    ///
    /// # Arguments
    ///
    /// * `target_table` - The table where types will be updated.
    /// * `changes` - A vector of [`TypeSimplification`] instructions generated by
    ///   the collection phase.
    fn apply_type_simplifications(
        target_table: &mut SymbolTable,
        changes: Vec<TypeSimplification>,
    ) {
        for change in changes {
            if let Some(entry) = target_table.get_symbol_mut(change.symbol_id) {
                let key = entry
                    .declarations()
                    .iter()
                    .find(|d| d.node_id() == change.node_id)
                    .cloned();

                if let Some(decl_key) = key {
                    if let Some(mut original) = entry.declarations_mut().take(&decl_key) {
                        // On récupère les IDs qui sont MAINTENANT garantis (grâce à l'init)
                        if let Some(old_ids) = original.ty_node_ids() {
                            // On ne garde que les IDs des types qui n'ont pas été supprimés
                            let new_ids: Vec<NodeId> = change
                                .kept_indices
                                .iter()
                                .filter_map(|&i| old_ids.get(i))
                                .cloned()
                                .collect();

                            // Si l'init et la simplification sont bons,
                            // new_ids.len() sera TOUJOURS égal à change.new_type.len()
                            original.set_ty_node_ids(new_ids);
                        }

                        original.set_ty(change.new_type);
                        entry.declarations_mut().insert(original);
                    }
                }
            }
        }
    }
}

/// Private internal structure representing a type modification to be applied.
struct TypeSimplification {
    symbol_id: SymbolId,
    node_id: NodeId,
    new_type: Type<SymbolId>,
    kept_indices: Vec<usize>, // <--- Les indices originaux des types conservés
}*/

/*fn apply_type_simplifications(
    target_table: &mut SymbolTable,
    changes: Vec<TypeSimplification>,
) {
    for change in changes {
        if let Some(entry) = target_table.get_symbol_mut(change.symbol_id) {
            let target_node_id = change.node_id;

            // Find the specific declaration by its NodeId to update it
            let key = entry
                .declarations()
                .iter()
                .find(|d| d.node_id() == target_node_id)
                .cloned();

            if let Some(decl_key) = key {
                if let Some(mut original) = entry.declarations_mut().take(&decl_key) {
                    original.set_ty(change.new_type);
                    entry.declarations_mut().insert(original);
                }
            }
        }
    }
}*/
