//! Provides semantic type checking for domain-specific symbols.
//!
//! This module defines the [`TypeChecker`] struct and its associated logic used for validating type
//! relationships in PDDL-like languages. It ensures that type declarations and usages are consistent,
//! and computes type hierarchies such as subtype and supertype relations.
//!
//! # Overview
//!
//! The type checking logic relies on a [`SymbolTable`] containing type declarations and their
//! hierarchical relationships. The [`TypeChecker`] uses this information to answer questions like:
//!
//! - Is type `A` a subtype of `B`?
//! - Do types `A` and `B` share a common supertype?
//! - What is the transitive closure of supertypes for a given type?
//!
//! This is useful in validating domain semantics and ensuring type correctness across
//! predicates, actions, and object declarations.
//!
//! # Key Components
//!
//! - [`TypeChecker`] — The main struct performing the actual type hierarchy analysis.
//! - [`TypeCheckError`] — Error type used when type resolution fails unexpectedly.
//!
//! # Features
//!
//! - Type hierarchy traversal via `ascending_type_closure`.
//! - Built-in PDDL type recognition (e.g., `object`, `number`).
//! - Caching of computed type closures to improve performance.
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
//! - [`Ident`](crate::aiplan4rust::lang::StringID)
//! - [`Type`](crate::aiplan4rust::lang::Type)
//!
//! # Notes
//!
//! This module assumes that the symbol table has been fully populated before type checking.

use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::Type;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::lang::StringID;
use crate::aiplan4rust::semantic::type_checker::TypeCheckError;

use std::collections::{HashMap, HashSet};
use std::cell::{Ref, RefCell};


/// PDDL Built-in symbols.
const PDDL_BUILTIN_TYPES: [StringID; 2] = [StringInterner::IDENT_OBJECT, StringInterner::IDENT_NUMBER];

/// A struct for performing type checking within a given domain.
///
/// The `TypeChecker` is responsible for verifying subtype and supertype relationships between
/// types defined in a domain. It works in conjunction with a [`SymbolTable`] that holds symbol
/// declarations and type definitions, and provides utilities for validating type compatibility
/// between objects, actions, and predicates.
///
/// # Fields
///
/// * `domain_symbol_table` - A reference to the [`SymbolTable`] containing the domain's symbol
///   declarations and type hierarchy.
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
    type_closure_cache: RefCell<HashMap<StringID, HashSet<StringID>>>,
}

impl<'a> TypeChecker<'a> {
    /// Creates a new `TypeChecker` instance using the given domain symbol table.
    ///
    /// The symbol table provides access to all type declarations and their relationships.
    /// This is essential for checking subtyping relationships, common supertypes, and type closure.
    ///
    /// # Arguments
    ///
    /// * `domain_symbol_table` - A reference to the symbol table that defines the domain's type hierarchy.
    ///
    /// # Returns
    ///
    /// A new instance of `TypeChecker` with caching enabled for transitive type closure.
    pub fn new(domain_symbol_table: &'a SymbolTable) -> Self {
        TypeChecker {
            domain_symbol_table,
            type_closure_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns `true` if any type in `ty2` is a subtype of any type in `ty1`.
    ///
    /// This checks whether the second list of types (e.g., expected or declared types) contains
    /// any type that is a descendant of at least one type in the first list.
    ///
    /// # Arguments
    ///
    /// * `ty1` - Set of potential supertypes.
    /// * `ty2` - Set of potential subtypes.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if any type in `ty2` is a subtype of any type in `ty1`.
    /// * `Ok(false)` if no subtype relation exists.
    /// * `Err(TypeCheckError)` if an internal resolution error occurs.
    pub fn is_any_subtype_of(
        &self,
        ty1: &Type<StringID>,
        ty2: &Type<StringID>,
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

    /// Returns `true` if any type in `ty1` is a supertype of any type in `ty2`.
    ///
    /// This is equivalent to checking whether any type in `ty2` is a subtype of any type in `ty1`.
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
        ty1: &Type<StringID>,
        ty2: &Type<StringID>,
    ) -> Result<bool, TypeCheckError> {
        self.is_any_subtype_of(ty2, ty1)
    }

    /// Returns `true` if any type in `ty1` is a subtype or supertype of any type in `ty2`.
    ///
    /// Combines both [`is_any_subtype_of`] and [`is_any_supertype_of`] checks.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if at least one relationship exists.
    /// * `Ok(false)` otherwise.
    /// * `Err(TypeCheckError)` if an error occurs in type resolution.
    pub fn is_any_sub_or_supertype_of(
        &self,
        ty1: &Type<StringID>,
        ty2: &Type<StringID>,
    ) -> Result<bool, TypeCheckError> {
        Ok(self.is_any_subtype_of(ty1, ty2)? || self.is_any_supertype_of(ty1, ty2)?)
    }

    /// Returns `true` if two sets of types share at least one common supertype.
    ///
    /// This function computes the transitive closure of all supertypes for each type
    /// in `ty1` and `ty2`, and then checks if there's any intersection.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if a shared supertype is found.
    /// * `Ok(false)` if the sets are disjoint in the hierarchy.
    /// * `Err(TypeCheckError)` on resolution failure.
    pub fn have_common_supertype(
        &self,
        ty1: &Type<StringID>,
        ty2: &Type<StringID>,
    ) -> Result<bool, TypeCheckError> {
        let mut supertypes1 = HashSet::new();
        for t1 in ty1.iter() {
            supertypes1.extend(self.ascending_type_closure(*t1)?.iter().cloned());
        }

        for t2 in ty2.iter() {
            if self.ascending_type_closure(*t2)?
                .iter()
                .any(|t| supertypes1.contains(t))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns all supertypes (including itself) of the given type.
    ///
    /// Performs a depth-first traversal of the type hierarchy starting from
    /// the provided primitive type, and collects all supertypes.
    ///
    /// # Arguments
    ///
    /// * `primitive_type` - The base type from which the closure is computed.
    ///
    /// # Returns
    ///
    /// * `Ok(Ref<HashSet<Ident>>)` containing the closure.
    /// * `Err(TypeCheckError)` if type resolution fails.
    ///
    /// # Caching
    ///
    /// If the closure for the type has already been computed, the cached result is reused.
    pub fn ascending_type_closure(
        &self,
        primitive_type: StringID,
    ) -> Result<Ref<HashSet<StringID>>, TypeCheckError> {
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
                if let Some(supertypes) = decl.types() {
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

    /// Checks if the given type is a built-in PDDL type.
    ///
    /// Built-in types are terminal in the type hierarchy and should not be resolved
    /// or traversed further during closure computation.
    ///
    /// # Arguments
    ///
    /// * `types` - The identifier to check.
    ///
    /// # Returns
    ///
    /// * `true` if the type is built-in.
    /// * `false` otherwise.
    pub fn is_pddl_builtin_types(ty: StringID) -> bool {
        PDDL_BUILTIN_TYPES.contains(&ty)
    }
}
