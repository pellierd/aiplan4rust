use std::collections::HashMap;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// A central registry managing value domains for every type within a planning problem.
///
/// The `ValueRegistry` handles the collection, deduplication, and sorting of objects
/// from a [`LiftedProblem`]. It is optimized for the grounding phase, providing
/// $O(1)$ access to type domains via [`TypeId`].
///
/// # Internal Structure
/// Domains are stored contiguously in a vector to maximize CPU cache locality during
/// the intensive iterations required for quantifier expansion and action grounding.
#[derive(Debug, Clone)]
pub struct ValueRegistry {
    /// Main storage indexed by `TypeId`. Each [`ValueDomain`] contains a sorted 
    /// and unique list of [`ObjectId`]s.
    type_domains: Vec<ValueDomain>,
    /// The initial capacity allocated for each type bucket during object collection.
    init_size: usize,
}

impl ValueRegistry {

    /// Creates a valid but empty `ValueRegistry`.
    ///
    /// This is primarily intended for unit testing or scenarios where a registry
    /// is required but no type/object data is available yet. It bypasses the
    /// collection and optimization logic.
    pub fn empty() -> Self {
        Self {
            type_domains: Vec::new(),
            init_size: 0,
        }
    }

    /// Builds and finalizes the registry from decoupled type and object definitions.
    ///
    /// Unique point d'entrée pour construire un registre validé et optimisé.
    /// Cette fonction combine la collecte, le tri et le dédoublonnage.
    ///
    /// # Process
    /// 1. **Collection**: Extracting objects and mapping them to all applicable types
    ///    in the hierarchy using [`Self::collect_objects`].
    /// 2. **Optimization**: Sorting and deduplicating each domain to ensure $O(\log n)$
    ///    search speed and deterministic grounding results.
    /// 3. **Finalization**: Encapsulating data into immutable [`ValueDomain`]s.
    ///
    /// # Arguments
    /// * `type_defs` - The complete list of type declarations.
    /// * `object_defs` - The objects (constants and problem objects) to be registered.
    /// * `init_size` - The initial capacity for each type bucket to minimize reallocations.
    ///
    /// # Errors
    /// Returns a [`GroundingError`] if the type hierarchy is inconsistent.
    pub fn build(
        type_defs: &[TypedSymbol<TypeId, TypeId>],
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
        init_size: usize,
    ) -> Result<Self, GroundingError> {

        // 1. COLLECTION: Call the bucket distribution logic.
        // We pass init_size explicitly as we don't have an instance yet.
        let raw_objects = Self::collect_objects(type_defs, object_defs, init_size);

        // 2. OPTIMIZATION: Sort and Deduplicate each domain.
        // We use into_iter to move the raw vectors into the domains without deep-copying.
        let type_domains = raw_objects
            .into_iter()
            .map(|mut objs| {
                objs.sort_unstable();
                objs.dedup();
                ValueDomain::new(objs)
            })
            .collect();

        // 3. ASSEMBLY: Return the finalized object.
        Ok(Self {
            type_domains,
            init_size,
        })
    }

    /// Scans the provided definitions to extract and categorize objects by their types.
    ///
    /// This method performs a "decoupled" collection, meaning it does not require
    /// a full `LiftedProblem` but only the relevant slices of type and object definitions.
    ///
    /// # Memory Management
    /// To minimize reallocations, this method uses `init_size` to pre-allocate internal
    /// buckets for each type. This is particularly efficient for problems with a
    /// large number of objects (e.g., logistics or satellite domains).
    ///
    /// # Type Hierarchy
    /// PDDL/HDDL objects can belong to multiple types via inheritance. This method
    /// respects that hierarchy by iterating over all `members()` of an object's type
    /// and pushing the [`ObjectId`] into every corresponding type bucket.
    ///
    /// # Arguments
    /// * `type_defs` - The complete list of type declarations to determine the number of buckets.
    /// * `object_defs` - The objects (constants or problem-specific objects) to be registered.
    /// * `init_size` - The initial capacity allocated for each type bucket.
    fn collect_objects(
        type_defs: &[TypedSymbol<TypeId, TypeId>],
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
        init_size: usize,
    ) -> Vec<Vec<ObjectId>> {
        let num_types = type_defs.len();

        // repeat_with ensures each inner Vec is initialized with its own capacity.
        let mut tmp_objects: Vec<Vec<ObjectId>> = std::iter::repeat_with(|| Vec::with_capacity(init_size))
            .take(num_types)
            .collect();

        for typed_object in object_defs {
            let obj_id = typed_object.symbol();
            // An object can belong to multiple types in a hierarchy.
            for &ty_id in typed_object.ty().members() {
                if let Some(bucket) = tmp_objects.get_mut(ty_id.as_usize()) {
                    bucket.push(obj_id);
                }
            }
        }
        tmp_objects
    }

    /// Retrieves the domains corresponding to a list of typed variables.
    ///
    /// This is typically used to initialize iterators for quantifier expansion 
    /// or action instantiation. Returns a vector of references to the internal 
    /// [`ValueDomain`]s.
    pub fn get_variable_domains(&self, variables: &TypedList<VariableId, TypeId>) -> Vec<&ValueDomain> {
        variables
            .iter()
            .map(|var| self.get_type_domain(var.ty()))
            .collect()
    }

    /// Retrieves the value domain for a specific [`Type`].
    ///
    /// # Panics
    /// Panics if the provided type contains no primitive members or if the
    /// internal hierarchy is malformed.
    pub fn get_type_domain(&self, ty: &Type<TypeId>) -> &ValueDomain {
        self.get_primitive_type_domain(ty.members()[0])
    }

    /// Direct $O(1)$ access to a type domain via its [`TypeId`].
    ///
    /// # Panics
    /// Panics if the `type_id` is out of bounds for this registry.
    pub fn get_primitive_type_domain(&self, type_id: TypeId) -> &ValueDomain {
        &self.type_domains[type_id.as_usize()]
    }
}

#[cfg(test)]
impl  ValueRegistry {
    /*pub fn with_typed_list(mut self, objects: TypedList<ObjectId, TypeId>) -> Self {
        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();

        for ts in objects {
            for tid in ts.ty().members() {
                grouped.entry(*tid).or_default().push(ts.symbol());
            }
        }

        // --- MODIFICATION ICI ---
        // Au lieu de quitter si c'est vide, on regarde l'ID de type le plus élevé
        // que l'on veut supporter, ou on s'assure d'une taille minimale.
        let max_id = grouped.keys()
            .map(|&tid| usize::from(tid))
            .max()
            .unwrap_or(0); // Par défaut 0, donc le vecteur aura au moins une taille de 1

        // On initialise/agrandit le vecteur
        self.type_domains = vec![ValueDomain::new(Vec::new()); max_id + 1];

        for (tid, mut objs) in grouped {
            objs.sort_unstable();
            objs.dedup();
            self.type_domains[usize::from(tid)] = ValueDomain::new(objs);
        }

        self
    }*/

    /// Helper for unit tests to build a registry directly from a list of objects.
    ///
    /// This bypasses the standard `build` pipeline and is intended ONLY for
    /// testing isolated logic where a full `LiftedProblem` is not available.
    pub fn from_objects<I>(objects: I) -> Self
    where
        I: IntoIterator<Item = TypedSymbol<ObjectId, TypeId>>
    {
        use std::collections::HashMap;

        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();
        let mut max_id = 0;

        for ts in objects {
            for tid in ts.ty().members() {
                let id_idx = usize::from(*tid);
                if id_idx > max_id {
                    max_id = id_idx;
                }
                grouped.entry(*tid).or_default().push(ts.symbol());
            }
        }

        // Initialize domains up to the highest TypeId found.
        let mut type_domains = vec![ValueDomain::new(Vec::new()); max_id + 1];

        for (tid, mut objs) in grouped {
            objs.sort_unstable();
            objs.dedup();
            type_domains[usize::from(tid)] = ValueDomain::new(objs);
        }

        Self {
            type_domains,
            init_size: 0, // No pre-allocation needed for static test lists
        }
    }
}
