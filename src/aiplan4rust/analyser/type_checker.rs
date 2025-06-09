use std::cell::{Ref, RefCell};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::lexer::token::NUMBER_TYPE;
use crate::aiplan4rust::parser::lexer::token::OBJECT_TYPE;
use crate::aiplan4rust::analyser::symbol::Scope;
use crate::aiplan4rust::analyser::symbol::SymbolKind;
use crate::aiplan4rust::analyser::symbol_table::SymbolTable;

use std::collections::{HashMap, HashSet};

/// PDDL Built-in symbols.
const PDDL_BUILTIN_TYPES: [&str; 2] = [OBJECT_TYPE, NUMBER_TYPE];

/// A struct for performing type checking within a given domain.
///
/// This struct holds a reference to the `domain_symbol_table`, which contains the symbol declarations
/// and type information used to verify the types of symbols within the domain. The `TypeChecker` is
/// responsible for checking type compatibility, resolving type hierarchies, and ensuring that symbols
/// are correctly used according to their defined types.
///
/// # Fields
///
/// * `domain_symbol_table` - A reference to the `SymbolTable` that contains the symbol declarations
///   for the domain. This is used to resolve type information and check the type hierarchy of symbols
///   during type checking.
///
/// # Derives
///
/// The `TypeChecker` struct derives the following traits:
/// - `Debug`: Enables the ability to format the `TypeChecker` instance for debugging purposes.
/// - `Clone`: Allows cloning of `TypeChecker` instances, enabling multiple instances to share
///   the same `domain_symbol_table` without ownership issues.
#[derive(Debug, Clone)]
pub struct TypeChecker<'a> {
    domain_symbol_table: &'a SymbolTable,
    type_closure_cache: RefCell<HashMap<String, HashSet<String>>>,
}

impl<'a> TypeChecker<'a> {
    /// Constructs a new `TypeChecker` instance.
    ///
    /// This function initializes a `TypeChecker` with a reference to the `domain_symbol_table`.
    /// The `domain_symbol_table` plays a crucial role in retrieving the type hierarchy associated
    /// with the symbols, which is essential for performing type checking and matching symbols
    /// with their expected types.
    ///
    /// # Arguments
    ///
    /// * `domain_symbol_table` - A reference to the `SymbolTable` that contains the symbol
    ///   declarations and is used to retrieve the type hierarchy. This table is key for resolving
    ///   type information  for symbols and their relationships in the context of the given domain.
    ///
    /// # Returns
    ///
    /// A new instance of `TypeChecker` initialized with the provided `domain_symbol_table`.
    pub fn new(domain_symbol_table: &'a SymbolTable) -> Self {
        TypeChecker {
            domain_symbol_table,
            type_closure_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Checks if any type in the second set (`ty2`) is a subtype of any type in the first set
    /// (`ty1`) within the given scope.
    ///
    /// This function determines whether there exists at least one type in `ty2` that is a subtype
    /// of any type in `ty1`. It uses the ascending type closure to consider all supertypes of each
    /// type in `ty2`, and checks if any of these supertypes match a type in `ty1`.
    ///
    /// # Arguments
    /// * `ty1` - A reference to a vector of strings representing the first set of types
    ///   (supertypes).
    /// * `ty2` - A reference to a vector of strings representing the second set of types (potential
    ///   subtypes).
    ///
    /// # Returns
    /// * `Ok(true)` if any type in `ty2` is a subtype of any type in `ty1`.
    /// * `Ok(false)` if no such subtype relation is found.
    /// * `Err(ParserInternalError)` if an error occurs while computing the ascending type closure.
    ///
    /// # Algorithm
    /// The function first converts `ty1` into a `HashSet` to optimize lookup performance.
    /// Then, for each type in `ty2`, it retrieves its ascending type closure (the type and all its
    /// supertypes). If any element of this closure is found in `ty1`, the function returns
    /// `Ok(true)`. If no matches are found after processing all types in `ty2`, it returns
    /// `Ok(false)`.
    ///
    /// # Example
    /// ```rust
    /// let ty1 = vec!["Animal".to_string(), "Vehicle".to_string()];
    /// let ty2 = vec!["Dog".to_string()];
    /// let result = type_checker.is_any_subtype_of(&ty1, &ty2);
    /// match result {
    ///     Ok(true) => { /* Dog is a subtype of Animal or Vehicle */ },
    ///     Ok(false) => { /* No subtype relation found */ },
    ///     Err(e) => { /* Handle error */ },
    /// }
    /// ```
    ///
    /// # Notes
    /// This function depends on the correctness of the `ascending_type_closure` method to properly
    /// reflect the type hierarchy and ensure accurate subtype detection.
    pub fn is_any_subtype_of(
        &self,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        let ty1_set: HashSet<_> = ty1.iter().collect(); // références, pas de clone

        for ty in ty2.iter() {
            let closure = self.ascending_type_closure(ty)?;

            // Check if any element in closure is in ty1_set
            if closure.iter().any(|closure_ty| ty1_set.contains(closure_ty)) {
                return Ok(true);
            }
        }

        Ok(false)
    }


    /// Checks if any type in `ty1` is a supertype of any type in `ty2` within the given scope.
    ///
    /// This function leverages `is_any_subtype_of` by inverting the parameters to determine
    /// if `ty1` contains any supertype of the types in `ty2`.
    ///
    /// # Arguments
    /// * `ty1` - A reference to a vector of strings representing the candidate supertype set.
    /// * `ty2` - A reference to a vector of strings representing the candidate subtype set.
    ///
    /// # Returns
    /// * `Ok(true)` if there exists at least one type in `ty1` that is a supertype of any type in
    ///   `ty2`.
    /// * `Ok(false)` if no such supertype relationship exists.
    /// * `Err(ParserInternalError)` if an error occurs during subtype checking.
    ///
    /// # Example
    /// ```rust
    /// let ty1 = vec!["TypeA".to_string()];
    /// let ty2 = vec!["TypeB".to_string(), "TypeC".to_string()];
    /// let result = type_checker.is_any_supertype_of(&ty1, &ty2);
    /// match result {
    ///     Ok(true) => { /* ty1 has a supertype of ty2 */ },
    ///     Ok(false) => { /* no supertype relationship found */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    pub fn is_any_supertype_of(
        &self,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        // We check if any type in ty2 is a subtype of any type in ty1
        self.is_any_subtype_of(ty2, ty1)
    }

    /// Checks if there is any subtype or supertype relationship between two sets of types.
    ///
    /// This function returns `Ok(true)` if any type in `ty1` is either a subtype or a supertype
    /// of any type in `ty2` within the given scope. It leverages the existing functions
    /// `is_any_subtype_of` and `is_any_supertype_of` to perform these checks.
    ///
    /// # Arguments
    /// * `ty1` - A reference to a vector of strings representing the first set of types.
    /// * `ty2` - A reference to a vector of strings representing the second set of types.
    ///
    /// # Returns
    /// * `Ok(true)` if any subtype or supertype relationship exists between the two sets.
    /// * `Ok(false)` if no such relationship exists.
    /// * `Err(ParserInternalError)` if an error occurs during the subtype or supertype checks.
    ///
    /// # Example
    /// ```rust
    /// let ty1 = vec!["TypeA".to_string()];
    /// let ty2 = vec!["TypeB".to_string()];
    /// let result = type_checker.is_any_sub_or_supertype_of(&ty1, &ty2);
    /// match result {
    ///     Ok(true) => { /* there is a subtype or supertype relationship */ },
    ///     Ok(false) => { /* no subtype or supertype relationship */ },
    ///     Err(e) => { /* handle error */ },
    /// }
    /// ```
    pub fn is_any_sub_or_supertype_of(
        &self,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        Ok(self.is_any_subtype_of(ty1, ty2)? || self.is_any_supertype_of(ty1, ty2)?)
    }

    /// Checks if two sets of types share at least one common supertype.
    ///
    /// For each type in `ty1` and `ty2`, the function retrieves the ascending closure of supertypes
    /// (i.e., all supertypes inherited directly or indirectly), then determines if there is any
    /// supertype present in both sets.
    ///
    /// # Arguments
    ///
    /// * `ty1` - A reference to a vector of strings representing the first set of types.
    /// * `ty2` - A reference to a vector of strings representing the second set of types.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if at least one common supertype is found.
    /// * `Ok(false)` otherwise.
    /// * `Err(ParserInternalError)` if an error occurs while retrieving the supertypes closure.
    ///
    /// # Example
    ///
    /// ```
    /// let types_a = vec!["TypeA".to_string(), "TypeB".to_string()];
    /// let types_b = vec!["TypeC".to_string()];
    /// let result = my_struct.have_common_supertype(&types_a, &types_b);
    /// match result {
    ///     Ok(true) => println!("Types share a common supertype"),
    ///     Ok(false) => println!("No common supertype found"),
    ///     Err(e) => println!("Error: {:?}", e),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// This function uses `ascending_type_closure` to get the full set of supertypes for a given
    /// type.
    pub fn have_common_supertype(
        &self,
        ty1: &Vec<String>,
        ty2: &Vec<String>,
    ) -> Result<bool, ParserInternalError> {
        let mut supertypes1 = HashSet::new();

        for t1 in ty1 {
            let closure1 = self.ascending_type_closure(t1)?;
            supertypes1.extend(closure1.iter().cloned());
        }

        for t2 in ty2 {
            let closure2 = self.ascending_type_closure(t2)?;
            if closure2.iter().any(|s| supertypes1.contains(s)) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Computes the ascending type closure for a given primitive type.
    ///
    /// This function returns the set of all supertypes (including the type itself)
    /// reachable by traversing the type hierarchy upwards starting from the specified
    /// primitive type. It uses a cache to avoid recomputing closures for previously
    /// processed types.
    ///
    /// The closure includes the type itself, its immediate supertypes, their supertypes,
    /// and so on, stopping at built-in PDDL types which are not further traversed.
    ///
    /// # Parameters
    /// - `primitive_type`: The name of the primitive type whose ascending closure is computed.
    ///
    /// # Returns
    /// - `Ok(Ref<HashSet<String>>)` containing the cached or newly computed set of supertypes as
    ///   strings.
    /// - `Err(ParserInternalError)` if an error occurs while resolving declarations in the symbol
    ///   table.
    ///
    /// # Caching
    /// The computed closure is cached internally in `type_closure_cache` to improve
    /// performance for repeated queries.
    ///
    /// # Behavior
    /// - If the closure for `primitive_type` is already cached, it is returned immediately.
    /// - Otherwise, the function performs a depth-first search up the type hierarchy,
    ///   collecting all reachable supertypes.
    /// - Built-in PDDL types are treated as terminal nodes and not traversed further.
    ///
    /// # Example
    /// ```rust
    /// let closure = symbol_table.ascending_type_closure("my_type")?;
    /// for supertype in closure.iter() {
    ///     println!("Supertype: {}", supertype);
    /// }
    /// ```
    ///
    /// # Errors
    /// Returns a `ParserInternalError` if resolving the declaration of any supertype
    /// fails unexpectedly.
    ///
    /// # Notes
    /// The returned reference is tied to the internal cache and should not outlive
    /// the `SymbolTable` instance.
    ///
    /// ```

    pub fn ascending_type_closure(
        &self,
        primitive_type: &str,
    ) -> Result<Ref<HashSet<String>>, ParserInternalError> {
        {
            // First, try to return the cached value without recalculating
            let cache_ref = self.type_closure_cache.borrow();
            if cache_ref.contains_key(primitive_type) {
                return Ok(Ref::map(cache_ref, |cache| {
                    cache.get(primitive_type).unwrap()
                }));
            }
        }

        // Otherwise, we need to compute the closure
        let mut super_types = HashSet::new();
        let mut to_visit = Vec::with_capacity(8);
        to_visit.push(primitive_type);

        // Traverse the type hierarchy upwards
        while let Some(current_type) = to_visit.pop() {
            // Insert the current type; if it was already visited, skip it
            if !super_types.insert(current_type.to_string()) {
                continue;
            }

            // Skip built-in PDDL types, no need to traverse further
            if TypeChecker::is_pddl_builtin_types(current_type) {
                continue;
            }

            // Resolve the declaration for the current type in the root scope
            let declaration = self.domain_symbol_table.resolve_declaration(
                current_type,
                &SymbolKind::PrimitiveType,
                &Scope::root(),
            )?;

            match declaration {
                Some(declaration) => {
                    // If the declaration has supertypes, add them to the stack for traversal
                    if let Some(s_types) = declaration.types() {
                        to_visit.extend(s_types.iter().map(String::as_str));
                    }
                }
                None => continue, // No declaration found, skip
            }
        }

        // Insert the computed closure into the cache
        self.type_closure_cache
            .borrow_mut()
            .insert(primitive_type.to_string(), super_types);

        // Return a reference to the cached closure
        let cache_ref = self.type_closure_cache.borrow();
        Ok(Ref::map(cache_ref, |cache| {
            cache.get(primitive_type).unwrap()
        }))
    }

    /// Checks if the given type is a PDDL built-in symbol.
    ///
    /// This function checks if the provided type string matches any of the predefined
    /// built-in symbols in the PDDL language. The set of PDDL built-in symbols is
    /// stored in the constant `PDDL_BUILTIN_SYMBOLS`.
    ///
    /// # Arguments
    /// * `ty` - A reference to a string representing the type to check.
    ///
    /// # Returns
    /// * `bool` - `true` if the type is a PDDL built-in symbol, `false` otherwise.
    ///
    pub fn is_pddl_builtin_types(ty: &str) -> bool {
        PDDL_BUILTIN_TYPES.contains(&ty)
    }
}
