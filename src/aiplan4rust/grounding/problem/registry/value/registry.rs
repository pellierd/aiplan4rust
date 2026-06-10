//! This module provides a high-performance registry for object domains in grounded problems.
//!
//! # Technical Choice: Flattened Memory Layout
//!
//! The `ValueRegistry` uses a **contiguous flat-buffer** approach (`all_values`) combined
//! with index-based ranges (`ranges`).
//!
//! ### Inheritance & Multiple Inheritance Handling
//! To achieve $O(1)$ access to any type's domain (including all its subtypes), this
//! implementation deliberately uses **data duplication**.
//!
//! * **Flat Slices**: Every type is associated with a single, contiguous slice of [`ObjectId`].
//!     This allows the grounding engine to iterate over a type's domain without traversing
//!     graphs or performing multiple lookups.
//! * **Redundancy for Performance**: In cases of multiple inheritance (e.g., a "Diamond"
//!     hierarchy), an object will appear multiple times in the `all_values` buffer—once
//!     for each branch of the inheritance tree it belongs to.
//!
//! ### Benefits
//! 1.  **Cache Locality**: Iterating over a type's objects is a simple linear memory scan.
//! 2.  **No Recursion at Runtime**: All hierarchy resolution is pre-computed during the
//!     [`ValueRegistry::build`] phase.
//! 3.  **Zero Allocation during Retrieval**: Functions like [`get_type_domain`] return
//!     borrows (`&[ObjectId]`), avoiding any runtime overhead during the heavy grounding
//!     process.
//!
//! ### Cycle Detection
//! While the PDDL specification implies a Directed Acyclic Graph (DAG) for types,
//! this registry explicitly checks for and rejects circular dependencies during
//! construction to ensure system stability.

use crate::aiplan4rust::grounding::problem::registry::value::error::ValueRegistryError;
use crate::aiplan4rust::grounding::problem::registry::value::range::TypeRange;
use crate::aiplan4rust::support::lang::{
    ObjectId, Type, TypeId, TypedList, TypedSymbol, VariableId,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A high-performance, lookup-optimized registry for object domains.
///
/// This structure acts as the central repository for all objects in a grounded problem,
/// organizing them by their resolved type hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ValueRegistry {
    /// Contiguous storage of all object IDs.
    ///
    /// Objects are grouped by type and flattened so that any type's domain—including
    /// all its subtypes—forms a single, continuous slice. This layout maximizes
    /// CPU cache hits and enables zero-allocation domain retrieval.
    all_values: Vec<ObjectId>,

    /// A mapping from [`TypeId`] to its corresponding range in `all_values`.
    ///
    /// The index of the vector corresponds to the `as_usize()` value of the [`TypeId`].
    /// Each [`TypeRange`] provides the `[start, end)` offsets used to slice the
    /// `all_values` buffer in $O(1)$ time.
    ranges: Vec<TypeRange>,

    /// The count of unique objects defined in the problem before hierarchy flattening.
    ///
    /// This value is distinct from `all_values.len()` because objects are duplicated
    /// across multiple type ranges to satisfy inheritance requirements while
    /// maintaining contiguous memory access.
    unique_objects_count: usize,
}

impl ValueRegistry {
    /// Creates an empty registry with no types or objects.
    ///
    /// Useful for initializing a placeholder registry or for testing purposes
    /// where no grounding data is required.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            all_values: Vec::new(),
            ranges: Vec::new(),
            unique_objects_count: 0,
        }
    }

    /// Creates a registry pre-allocated for a specific number of types.
    ///
    /// This initializes the range mapping but leaves the value buffer empty.
    ///
    /// # Arguments
    ///
    /// * `num_types` - The number of type slots to allocate in the range vector.
    #[must_use]
    pub fn with_types(num_types: usize) -> Self {
        Self {
            all_values: Vec::new(),
            ranges: vec![TypeRange::default(); num_types],
            unique_objects_count: 0,
        }
    }

    /// Returns the total number of object entries stored in memory.
    ///
    /// Note that this count includes duplicates across different type domains
    /// due to the flattened inheritance structure. For the count of unique
    /// physical objects, use [`unique_objects_count`].
    #[inline]
    pub fn storage_size(&self) -> usize {
        self.all_values.len()
    }

    /// Returns the number of unique objects defined in the problem.
    ///
    /// Unlike [`storage_size`], this value represents the actual count of
    /// distinct constants (symbols) before they were propagated through
    /// the type hierarchy.
    #[inline]
    pub fn unique_objects_count(&self) -> usize {
        self.unique_objects_count
    }

    /// Retrieves the domains corresponding to a list of typed variables.
    ///
    /// This is typically used to initialize iterators for grounding operations,
    /// such as quantifier expansion or action instantiation.
    ///
    /// # Arguments
    ///
    /// * `variables` - A [`TypedList`] of variables whose types define the domains to retrieve.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<&[ObjectId]>)` - A vector where each entry is a slice of all [`ObjectId`]s
    ///   belonging to the corresponding variable's type.
    /// * `Err(RegistryError)` - If any variable's type cannot be resolved (e.g., non-primitive
    ///   type or out-of-bounds index).
    ///
    /// # Errors
    ///
    /// This function will propagate errors from [`get_type_domain`] if a variable
    /// references an invalid or unnormalized type.
    pub fn get_variable_domains(
        &self,
        variables: &TypedList<VariableId, TypeId>,
    ) -> Result<Vec<&[ObjectId]>, ValueRegistryError> {
        variables
            .iter()
            .map(|var| self.get_type_domain(var.ty()))
            .collect()
    }

    /// Retrieves the value domain for a specific [`Type`].
    ///
    /// This is a convenience wrapper around [`get_primitive_type_domain`]. It assumes
    /// the type has been normalized to a single primitive member.
    ///
    /// # Arguments
    ///
    /// * `ty` - The type whose domain is being requested.
    ///
    /// # Returns
    ///
    /// * `Ok(&[ObjectId])` - A contiguous slice of all objects belonging to this type.
    /// * `Err(RegistryError::NotPrimitiveType)` - If the provided type is a union (either)
    ///   and cannot be mapped to a single contiguous range.
    /// * `Err(RegistryError::RootType)` - If the type has no members.
    pub fn get_type_domain(&self, ty: &Type<TypeId>) -> Result<&[ObjectId], ValueRegistryError> {
        let members = ty.members();
        match members.len() {
            1 => self.get_primitive_type_domain(members[0]),
            0 => Err(ValueRegistryError::root_type(TypeId::from(0))),
            n => Err(ValueRegistryError::not_primitive(members[0], n)),
        }
    }

    /// Returns the domain of a type as a slice of [`ObjectId`].
    ///
    /// This provides direct $O(1)$ access to the flattened domain (including inherited objects)
    /// stored in the global buffer.
    ///
    /// # Arguments
    ///
    /// * `type_id` - The unique identifier of the type whose domain is being requested.
    ///
    /// # Returns
    ///
    /// * `Ok(&[ObjectId])` - A contiguous slice of all objects belonging to this type.
    /// * `Err(RegistryError::TypeIdOutOfBounds)` - If the `type_id` is invalid for this registry.
    ///
    /// # Performance
    ///
    /// This operation is $O(1)$ as it only involves a range lookup and a slice re-borrow.
    pub fn get_primitive_type_domain(
        &self,
        type_id: TypeId,
    ) -> Result<&[ObjectId], ValueRegistryError> {
        let idx = type_id.as_usize();
        if idx >= self.ranges.len() {
            return Err(ValueRegistryError::type_out_of_bounds(
                type_id,
                self.ranges.len(),
            ));
        }

        let r = &self.ranges[idx];
        Ok(&self.all_values[r.start()..r.end()])
    }

    /// Returns the index range associated with a specific type within the global storage.
    ///
    /// This provides $O(1)$ retrieval of the boundaries within the global value buffer.
    /// It is primarily used to identify the contiguous slice of [`ObjectId`] that
    /// represents the domain of a given type.
    ///
    /// # Arguments
    ///
    /// * `type_id` - The unique identifier of the type to query.
    ///
    /// # Returns
    ///
    /// * `Ok(TypeRange)` - The start and end indices within the global buffer.
    /// * `Err(RegistryError::TypeIdOutOfBounds)` - If the provided ID exceeds the registry capacity.
    ///
    /// # Performance
    ///
    /// This operation is a simple bounds-checked array access with $O(1)$ complexity.
    #[inline]
    pub fn get_range(&self, type_id: TypeId) -> Result<TypeRange, ValueRegistryError> {
        let idx = type_id.as_usize();
        if idx >= self.ranges.len() {
            return Err(ValueRegistryError::type_out_of_bounds(
                type_id,
                self.ranges.len(),
            ));
        }
        Ok(self.ranges[idx])
    }

    /// Constructs a new `ValueRegistry` by processing type hierarchies and object definitions.
    ///
    /// This associated function orchestrates the three-pass initialization of the registry:
    /// 1. **Graph Extraction**: Builds a subtype adjacency list from the provided type definitions.
    /// 2. **Object Mapping**: Distributes objects into their respective atomic types, ensuring
    ///    no invalid union types are assigned to constants.
    /// 3. **Domain Flattening**: Uses a post-order traversal to propagate objects through the
    ///    inheritance hierarchy, ensuring parent types contain all objects of their subtypes.
    ///
    /// # Arguments
    ///
    /// * `type_defs` - A slice of [`TypedSymbol`] representing the inheritance relationships.
    /// * `object_defs` - A slice of [`TypedSymbol`] defining the constants and their direct types.
    ///
    /// # Returns
    ///
    /// * `Ok(Self)` - A fully initialized registry with flattened domains ready for grounding.
    /// * `Err(RegistryError)` - If the input is malformed (e.g., circular inheritance,
    ///   non-primitive object types, or out-of-bounds references).
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * A cycle is detected in the type hierarchy during the flattening phase.
    /// * An object is mapped to a non-primitive (union) type.
    /// * A type has no members (RootType error) and cannot be resolved.
    pub fn build(
        type_defs: &[TypedSymbol<TypeId, TypeId>],
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
    ) -> Result<Self, ValueRegistryError> {
        let num_types = type_defs.len();

        // Step 1: Structural analysis of the inheritance graph
        let adj = Self::extract_subtypes_graph(num_types, type_defs)?;

        // Step 2: Initial distribution of raw objects to their direct types
        let direct_objects = Self::map_objects_to_types(num_types, object_defs)?;

        // Step 3: Final domain computation (Flattening via DFS)
        let mut all_values = Vec::with_capacity(object_defs.len() * 2);
        let mut ranges = vec![TypeRange::default(); num_types];
        let mut computed = vec![false; num_types];

        for i in 0..num_types {
            // Propagates objects from subtypes to ancestors and detects cycles
            Self::fill_registry(
                TypeId::from(i),
                &adj,
                &direct_objects,
                &mut all_values,
                &mut ranges,
                &mut computed,
            )?;
        }

        Ok(Self {
            all_values,
            ranges,
            unique_objects_count: object_defs.len(),
        })
    }

    /// Extracts the subtype relations to build an inheritance adjacency graph.
    ///
    /// This function constructs a directed graph where each parent type points to its
    /// direct subtypes. This structure is essential for the subsequent flattening phase,
    /// allowing object instances to propagate from specific subtypes up to their ancestors.
    ///
    /// # Arguments
    ///
    /// * `num_types` - The total number of unique types defined in the system.
    ///   Used to initialize and bounds-check the adjacency list.
    /// * `type_defs` - A slice of type definitions containing the symbol (child)
    ///   and its members (parents).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Vec<TypeId>>)` - An adjacency list where `list[parent_index]`
    ///   contains all its direct children.
    /// * `Err(ValueRegistryError::TypeIdOutOfBounds)` - If a child or parent ID exceeds `num_types`.
    ///
    /// # Errors
    ///
    /// This function performs strict bounds checking on all `TypeId`s encountered in
    /// `type_defs`. If any ID is greater than or equal to `num_types`, it returns
    /// a [`ValueRegistryError::type_out_of_bounds`] instead of panicking.
    fn extract_subtypes_graph(
        num_types: usize,
        type_defs: &[TypedSymbol<TypeId, TypeId>],
    ) -> Result<Vec<Vec<TypeId>>, ValueRegistryError> {
        let mut adj = vec![Vec::new(); num_types];

        for def in type_defs {
            let child = def.symbol();
            let child_idx = child.as_usize();

            // Vérification de l'ID de l'enfant
            if child_idx >= num_types {
                return Err(ValueRegistryError::type_out_of_bounds(child, num_types));
            }

            for &parent in def.ty().members() {
                let parent_idx = parent.as_usize();

                // Vérification de l'ID du parent (L'index 99 qui faisait paniquer)
                if parent_idx >= num_types {
                    return Err(ValueRegistryError::type_out_of_bounds(parent, num_types));
                }

                adj[parent_idx].push(child);
            }
        }
        Ok(adj)
    }

    /// Maps each object to its primary atomic type.
    ///
    /// This function acts as the initial distribution phase, placing each object into
    /// its specific type container before any inheritance or flattening logic is applied.
    ///
    /// # Arguments
    ///
    /// * `num_types` - The total number of unique types defined in the problem.
    ///   Used to pre-allocate the distribution vectors.
    /// * `object_defs` - A slice of symbols containing the mapping between
    ///   [`ObjectId`] and their declared [`Type`].
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Vec<ObjectId>>)` - A vector of size `num_types`, where each sub-vector
    ///   contains the objects directly associated with that type index.
    /// * `Err(RegistryError)` - If an object is malformed or its type is not primitive.
    ///
    /// # Errors
    ///
    /// * [`ValueRegistryError::TypeIdOutOfBounds`]: Occurs if an object refers to a type index
    ///   larger than `num_types`.
    /// * [`ValueRegistryError::NotPrimitiveType`]: Occurs if an object is associated with
    ///   a union type (more than one member). After normalization, every object must
    ///   belong to exactly one atomic type.
    /// * [`ValueRegistryError::RootType`]: Occurs if an object has no members in its type
    ///   definition, making it impossible to assign to a domain.
    fn map_objects_to_types(
        num_types: usize,
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
    ) -> Result<Vec<Vec<ObjectId>>, ValueRegistryError> {
        let mut direct_objects = vec![Vec::new(); num_types];

        for obj_def in object_defs {
            let obj_id = obj_def.symbol();
            let members = obj_def.ty().members();

            match members.len() {
                1 => {
                    let t_id = members[0];
                    let idx = t_id.as_usize();
                    if idx >= num_types {
                        return Err(ValueRegistryError::type_out_of_bounds(t_id, num_types));
                    }
                    direct_objects[idx].push(obj_id);
                }
                0 => {
                    // Object has no defined type
                    return Err(ValueRegistryError::root_type(TypeId::from(0)));
                }
                n => {
                    // Object has an "either" type, which is forbidden after normalization
                    return Err(ValueRegistryError::not_primitive(members[0], n));
                }
            }
        }

        Ok(direct_objects)
    }

    /// Populates the value registry for a specific type and its descendants using an iterative DFS.
    ///
    /// This function computes the flattened domain of a type by merging its direct objects
    /// with the objects of all its subtypes. It performs a post-order traversal to guarantee
    /// that child domains are finalized before their parents are processed.
    ///
    /// # Arguments
    ///
    /// * `start_node` - The root [`TypeId`] from which to begin the population.
    /// * `adj` - The inheritance adjacency list (Parent -> Subtypes).
    /// * `direct_objects` - A collection of objects explicitly mapped to each type.
    /// * `all_values` - The global buffer where consolidated domains are stored.
    /// * `ranges` - A slice of [`TypeRange`] updated as each type is finalized.
    /// * `computed` - A bitset tracking types that have been fully processed (Black nodes).
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the hierarchy was successfully processed.
    /// * `Err(RegistryError::CycleDetected)` - If a circular inheritance dependency is found.
    ///
    /// # Cycle Detection & Safety
    ///
    /// The algorithm implements a non-recursive DFS with cycle detection:
    /// - **`in_stack` (Grey nodes)**: Tracks types currently being traversed. If an "Enter"
    ///   state encounters a type already in this set, a cycle is detected.
    /// - **`computed` (Black nodes)**: Tracks types whose domains are fully built, allowing
    ///   the algorithm to skip previously calculated branches in complex DAGs.
    fn fill_registry(
        start_node: TypeId,
        adj: &[Vec<TypeId>],
        direct_objects: &[Vec<ObjectId>],
        all_values: &mut Vec<ObjectId>,
        ranges: &mut [TypeRange],
        computed: &mut Vec<bool>,
    ) -> Result<(), ValueRegistryError> {
        enum State {
            /// First visit to a node: push children to the stack.
            Enter(TypeId),
            /// Final visit to a node: compute domain after children are done.
            Exit(TypeId),
        }

        // Represents the "Grey" state in DFS (nodes currently in the recursion stack)
        let mut in_stack = vec![false; computed.len()];
        let mut stack = vec![State::Enter(start_node)];

        while let Some(state) = stack.pop() {
            match state {
                State::Enter(u) => {
                    let u_idx = u.as_usize();

                    // Skip if already finalized (Black node)
                    if computed[u_idx] {
                        continue;
                    }

                    // Check for cycles (re-entering a Grey node)
                    if in_stack[u_idx] {
                        return Err(ValueRegistryError::cycle_detected(u));
                    }

                    in_stack[u_idx] = true;
                    stack.push(State::Exit(u));

                    for &child in &adj[u_idx] {
                        let c_idx = child.as_usize();
                        if !computed[c_idx] {
                            // Early cycle detection for children
                            if in_stack[c_idx] {
                                return Err(ValueRegistryError::cycle_detected(child));
                            }
                            stack.push(State::Enter(child));
                        }
                    }
                }
                State::Exit(u) => {
                    let u_idx = u.as_usize();
                    if computed[u_idx] {
                        continue;
                    }

                    // All children are now guaranteed to be in `ranges`
                    ranges[u_idx] =
                        Self::compute_type_domain(u_idx, adj, direct_objects, all_values, ranges);

                    computed[u_idx] = true;
                    in_stack[u_idx] = false; // Mark as finalized (Black node)
                }
            }
        }
        Ok(())
    }

    /// Computes the complete value domain for a type by merging its own objects with those of its subtypes.
    ///
    /// This function implements the support logic of the domain hierarchy:
    /// 1. It flattens the hierarchy for a specific node in the type graph.
    /// 2. It collects direct objects associated with the type.
    /// 3. It copies all objects from previously computed child domains (subtypes).
    /// 4. It ensures the resulting domain is sorted and unique.
    ///
    /// # Arguments
    ///
    /// * `u_idx` - The index of the type currently being finalized.
    /// * `adj` - The adjacency list representing the type inheritance graph.
    /// * `direct_objects` - Objects explicitly declared for each type.
    /// * `all_values` - The global flat buffer where all domains are stored.
    /// * `ranges` - The slice of [`TypeRange`] used to locate already computed child domains.
    ///
    /// # Returns
    ///
    /// A [`TypeRange`] representing the contiguous segment in `all_values` dedicated to this type.
    ///
    /// # Panics
    ///
    /// This function assumes that all children of `u_idx` have already been computed (post-order traversal).
    /// It may panic if a child's range is invalid or out of bounds for `all_values`.
    ///
    /// # Performance
    ///
    /// This method performs an allocation (`to_vec()`) for each child domain to avoid borrow checker
    /// issues when copying from `all_values` into itself.
    fn compute_type_domain(
        u_idx: usize,
        adj: &[Vec<TypeId>],
        direct_objects: &[Vec<ObjectId>],
        all_values: &mut Vec<ObjectId>,
        ranges: &mut [TypeRange],
    ) -> TypeRange {
        let start = all_values.len();

        // 1. Add direct objects defined for this type
        all_values.extend_from_slice(&direct_objects[u_idx]);

        // 2. Inherit objects from children (subtypes)
        for &child in adj[u_idx].iter().rev() {
            let child_range = ranges[child.as_usize()];
            if !child_range.is_empty() {
                // We copy the child's domain into a temporary buffer to circumvent
                // simultaneous mutable and immutable borrows of `all_values`.
                let child_objs = all_values[child_range.start()..child_range.end()].to_vec();
                all_values.extend(child_objs);
            }
        }

        // 3. Sort and deduplicate the newly created segment in the global buffer
        let new_len = Self::sort_and_dedup(&mut all_values[start..]);
        all_values.truncate(start + new_len);

        TypeRange::new(start, start + new_len)
    }

    /// Sorts and deduplicates a slice of [`ObjectId`]s in-place.
    ///
    /// This function uses an unstable sort for maximum performance and then
    /// compacts the slice by moving unique elements to the front.
    ///
    /// # Arguments
    ///
    /// * `slice` - A mutable reference to the segment of the global vector to process.
    ///
    /// # Returns
    ///
    /// The number of unique elements remaining in the slice. The elements after this
    /// index in the original slice are left in an unspecified state.
    ///
    /// # Complexity
    ///
    /// * **Time complexity**: $O(n \log n)$ due to the unstable sort.
    /// * **Space complexity**: $O(1)$ (in-place) or $O(\log n)$ stack space depending
    ///   on the sort implementation, but it does not perform any heap allocations.
    ///
    /// # Performance Note
    ///
    /// `sort_unstable` is generally faster than `sort` because it does not maintain
    /// the relative order of equal elements, which is irrelevant for deduplication.
    fn sort_and_dedup(slice: &mut [ObjectId]) -> usize {
        if slice.is_empty() {
            return 0;
        }

        // 1. Sort: O(n log n)
        slice.sort_unstable();

        // 2. Deduplication: O(n) using a two-pointer approach
        let mut j = 0;
        for i in 1..slice.len() {
            if slice[i] != slice[j] {
                j += 1;
                slice[j] = slice[i];
            }
        }

        j + 1 // New length
    }
}

/// **Implements a visual representation of the ValueRegistry.**
///
/// This implementation provides a structured view of the object domains for each type.
///
/// * **Small domains** (≤ 15 objects) are displayed on a single line for readability.
/// * **Large domains** (> 15 objects) are automatically chunked into multiple lines
///   to prevent terminal overflow.
/// * **Range information** is included for each type to assist in debugging
///   the flat memory layout.
impl fmt::Display for ValueRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "======================= ValueRegistry ({} objects total): =======================",
            self.unique_objects_count()
        )?;

        for (type_idx, range) in self.ranges.iter().enumerate() {
            let type_id = TypeId::from(type_idx);

            // Safely extract the domain using the pre-calculated range
            let domain = &self.all_values[range.start()..range.end()];

            // Convert ObjectIds to strings for formatting
            let values: Vec<String> = domain.iter().map(|o| o.to_string()).collect();

            // Header format: "TypeId(0) [0..14]: "
            write!(f, "{:?} {}: ", type_id, range)?;

            if domain.is_empty() {
                writeln!(f, "{{}}")?;
            } else if domain.len() <= 15 {
                // COMPACT VIEW: Everything on one line
                writeln!(f, "{{ {} }}", values.join(", "))?;
            } else {
                // EXPANDED VIEW: Chunked for large domains
                writeln!(f, "{{")?;
                for chunk in values.chunks(15) {
                    writeln!(f, "    {}", chunk.join(", "))?;
                }
                writeln!(f, "}}")?;
            }
        }

        writeln!(
            f,
            "================================================================================"
        )
    }
}

#[cfg(test)]
impl ValueRegistry {
    /// Helper pour les tests unitaires : construit un registre à partir d'une liste d'objets.
    /// Note : Dans cette version de test, on considère que les types sont indépendants
    /// (pas de calcul de hiérarchie récursive, juste le mapping direct).
    pub fn from_objects<I>(objects: I) -> Self
    where
        I: IntoIterator<Item = TypedSymbol<ObjectId, TypeId>>,
    {
        use std::collections::{HashMap, HashSet};

        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();
        let mut unique_objs = HashSet::new(); // Pour compter les objets uniques
        let mut max_id = 0;

        // 1. Groupement initial
        for ts in objects {
            let obj_id = ts.symbol();
            unique_objs.insert(obj_id); // On enregistre l'objet

            for &tid in ts.ty().members() {
                let id_idx = tid.as_usize();
                if id_idx > max_id {
                    max_id = id_idx;
                }
                grouped.entry(tid).or_default().push(obj_id);
            }
        }

        let unique_object_count = unique_objs.len();

        // 2. Construction du stockage contigu
        let mut all_values = Vec::new();
        let mut ranges = vec![TypeRange::default(); max_id + 1];

        let mut sorted_keys: Vec<_> = grouped.keys().cloned().collect();
        sorted_keys.sort();

        for tid in sorted_keys {
            let mut objs = grouped.remove(&tid).unwrap();
            objs.sort_unstable();
            objs.dedup();

            let start = all_values.len();
            all_values.extend(&objs);
            let end = all_values.len();

            ranges[tid.as_usize()] = TypeRange::new(start, end);
        }

        Self {
            all_values,
            ranges,
            unique_objects_count: unique_object_count, // On ajoute le champ ici
        }
    }
}
