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
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;

use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};

/// The maximum number of members allowed in a type union for optimized simplification.
/// This limit is defined by the size of the bitmask (u128) used in the algorithm.
const MAX_UNION_SIMPLIFICATION_CAPACITY: usize = 128;

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
    domain_symbol_table: &'a SymbolTable,
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
    pub fn new(domain_symbol_table: &'a SymbolTable) -> Self {
        TypeChecker {
            domain_symbol_table,
            type_closure_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns `true` if any typing in `ty2` is a subtype of any typing in `ty1`.
    ///
    /// This checks whether the second list of types (e.g., expected or declared types) contains
    /// any typing that is a descendant of at least one typing in the first list.
    ///
    /// # Arguments
    ///
    /// * `ty1` - Set of potential supertypes.
    /// * `ty2` - Set of potential subtypes.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if any typing in `ty2` is a subtype of any typing in `ty1`.
    /// * `Ok(false)` if no subtype relation exists.
    /// * `Err(TypeCheckError)` if an internal resolution error occurs.
    pub fn is_any_subtype_of(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckError> {
        let ty1_set: HashSet<_> = ty1.iter().collect();

        for ty in ty2.iter() {
            let closure = self.ascending_type_closure(*ty)?;
            if closure.iter().any(|t| ty1_set.contains(t)) {
                return Ok(true);
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

            let declaration = self.domain_symbol_table.resolve_declaration(
                &current,
                &SymbolKind::PrimitiveType,
                &self.domain_symbol_table.root_scope(),
            )?;

            if let Some(decl) = declaration {
                if let Some(supertypes) = decl.ty() {
                    to_visit.extend(supertypes.iter().cloned());
                }
            }
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

    /// Simplifies a type union by removing redundant super-types.
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
    ) -> Result<Option<Type<SymbolId>>, TypeCheckError> {
        let members = ty.members();
        let n = members.len();

        // 1. Fast path: 0 or 1 member cannot be redundant
        if n <= 1 {
            return Ok(None);
        }

        // 2. Safety guard: bitmask capacity check (u128)
        if n > MAX_UNION_SIMPLIFICATION_CAPACITY {
            return Err(TypeCheckError::type_union_capacity_exceeded(n));
        }

        // 3. Bitmask to mark redundant types for removal (0 heap allocation)
        // We use u128 to support up to 128 members.
        let mut to_remove_mask: u128 = 0;
        let mut changed = false;

        for i in 0..n {
            let t1 = members[i];
            for j in 0..n {
                if i == j {
                    continue;
                }

                let t2 = members[j];

                // t1 is redundant if it is an ancestor of t2.
                // We check if t1 exists within the ascending closure of t2.
                let closure = self.ascending_type_closure(t2)?;
                if closure.contains(&t1) {
                    to_remove_mask |= 1 << i;
                    changed = true;
                    break; // t1 is marked, skip to the next member (i)
                }
            }
        }

        // 4. If no redundancy detected, avoid any further allocation
        if !changed {
            return Ok(None);
        }

        // 5. Final construction: single perfectly-sized Vec allocation.
        // count_ones() on u128 is still a very fast intrinsic.
        let final_capacity = n - to_remove_mask.count_ones() as usize;
        let mut simplified_ids = Vec::with_capacity(final_capacity);

        for i in 0..n {
            // If bit i is not set, the type is kept
            if (to_remove_mask & (1 << i)) == 0 {
                simplified_ids.push(members[i]);
            }
        }

        Ok(Some(Type::from(simplified_ids)))
    }
}
