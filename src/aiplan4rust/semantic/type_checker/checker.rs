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
//! - [`TypeCheckerError`] — Error typing used when typing resolution fails unexpectedly.
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
//! - [`Ident`](crate::aiplan4rust::support::lang::SymbolId)
//! - [`Type`](crate::aiplan4rust::support::lang::Type)
//!
//! # Notes
//!
//! This module assumes that the symbol table has been fully populated before typing checking.

use crate::aiplan4rust::semantic::type_checker::{TypeCheckerError, TypeHierarchy};
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::SymbolId;
use crate::aiplan4rust::support::lang::Type;

use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};

/// PDDL Built-in symbols.
const PDDL_BUILTIN_TYPES: [SymbolId; 1] = [SymbolInterner::NUMBER_SYMBOL_ID];

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
    /// This method evaluates the subtyping relationship by traversing the type hierarchy.
    /// It returns `true` if there exists at least one pair (t2, t1) such that t2 is a descendant
    /// of t1 or t2 == t1 (reflexivity).
    ///
    /// # PDDL Semantics & Root Types
    /// - **Unconstrained Types**: An empty type set (`is_root()`) represents the universal
    ///   `object` type in PDDL/HDDL.
    /// - **STRIPS Support**: In non-typed domains (like Mystery), both `ty1` and `ty2` will
    ///   be root types. This method correctly identifies them as compatible.
    /// - **Universal Parent**: If the expected type (`ty1`) is the root type, any provided
    ///   type (`ty2`) is considered a valid subtype by definition.
    ///
    /// # Performance
    /// - **Ultra-Fast Path**: Immediate return if `ty1` is the root type (universal match).
    /// - **Fast Path**: Performs an O(N*M) direct comparison to catch identical types
    ///   without traversing the hierarchy or accessing the cache.
    /// - **Slow Path**: Uses memoized transitive closures (`ascending_type_closure`)
    ///   to check for ancestral relationships.
    ///
    /// # Arguments
    /// * `ty1` - The set of potential supertypes (e.g., required types by a predicate signature).
    /// * `ty2` - The set of potential subtypes (e.g., types of the provided argument/variable).
    ///
    /// # Errors
    /// Returns [`TypeCheckerError`] if a `SymbolId` cannot be resolved within the hierarchy.
    pub fn is_any_subtype_of(
        &self,
        ty1: &Type<SymbolId>,
        ty2: &Type<SymbolId>,
    ) -> Result<bool, TypeCheckerError> {
        // If the expected type (ty1) is the root (object/empty), any provided
        // type is a valid subtype. This also handles the STRIPS case ([] <: []).
        if ty1.is_root() {
            return Ok(true);
        }

        // If ty1 is not the root but ty2 is, then ty2 cannot be a subtype.
        // (One cannot provide a generic 'object' where a specific type is required).
        if ty2.is_root() {
            return Ok(false);
        }

        // 1. Fast path: Direct overlap check (identity/reflexivity).
        for t2 in ty2.iter() {
            if ty1.iter().any(|t1| t1 == t2) {
                return Ok(true);
            }
        }

        // 2. Slow path: Hierarchical traversal.
        for t2 in ty2.iter() {
            // Retrieve the transitive closure of supertypes (includes t2 itself).
            let closure = self.ascending_type_closure(*t2)?;

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
    ) -> Result<bool, TypeCheckerError> {
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
    ) -> Result<bool, TypeCheckerError> {
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
    ) -> Result<bool, TypeCheckerError> {
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
    ) -> Result<Ref<HashSet<SymbolId>>, TypeCheckerError> {
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
