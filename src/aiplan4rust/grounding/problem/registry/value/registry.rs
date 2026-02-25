use std::collections::HashMap;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::problem::value_domain::ValueDomain;
use crate::aiplan4rust::lang::{ObjectId, Type, TypeId, TypedList, VariableId};
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
pub struct ValueRegistry {
    /// Main storage indexed by `TypeId`. Each [`ValueDomain`] contains a sorted 
    /// and unique list of [`ObjectId`]s.
    type_domains: Vec<ValueDomain>,
    /// The initial capacity allocated for each type bucket during object collection.
    init_size: usize,
}

impl ValueRegistry {
    /// Default pre-allocation size (16) for each type's object list.
    const DEFAULT_INIT_SIZE: usize = 16;

    /// Creates a new, empty `ValueRegistry` with default configuration.
    ///
    /// The registry must be populated using [`Self::with_problem`] before use.
    pub fn new() -> Self {
        Self {
            type_domains: Vec::new(),
            init_size: Self::DEFAULT_INIT_SIZE,
        }
    }

    /// Sets the initial capacity for object collection (Builder Pattern).
    ///
    /// If the problem is known to have a high number of objects per type, 
    /// increasing this value can significantly reduce the number of memory 
    /// reallocations during the initialization phase.
    ///
    /// # Example
    /// ```rust
    /// let registry = ValueRegistry::new()
    ///     .with_init_size(128)
    ///     .from_problem(&problem)?;
    /// ```
    pub fn with_init_size(mut self, size: usize) -> Self {
        self.init_size = size;
        self
    }

    /// Builds and finalizes the registry from a [`LiftedProblem`].
    ///
    /// This process involves:
    /// 1. Collecting all objects defined in the problem.
    /// 2. Organizing them by type (handling type hierarchies).
    /// 3. **Sorting** and **deduplicating** each domain to ensure deterministic grounding.
    ///
    /// This method consumes `self` to take ownership of the configuration.
    ///
    /// # Errors
    /// Returns a [`GroundingError`] if the problem structure is inconsistent or 
    /// if type definitions are missing.
    pub fn with_problem(self, problem: &LiftedProblem) -> Result<Self, GroundingError> {
        // 1. Raw data collection
        let raw_objects = self.collect_objects(problem);

        // 2. Transformation into optimized domains (Sort + Dedup)
        // We use into_iter to move the raw vectors without deep-copying data.
        let type_domains = raw_objects
            .into_iter()
            .map(|mut objs| {
                objs.sort_unstable(); // Fast sorting for primitive IDs
                objs.dedup();         // Linear deduplication on sorted vector
                ValueDomain::new(objs)
            })
            .collect();

        Ok(Self {
            type_domains,
            init_size: self.init_size
        })
    }

    /// Initializes the registry directly from a `TypedList`.
    ///
    /// # Note
    /// This is a convenience method primarily intended for **unit testing**.
    /// In a standard workflow, the registry should be initialized via `with_problem`
    /// to ensure consistency with the global problem definition.
    ///
    /// This method handles cases where `ts.ty()` returns a union type (multiple `TypeId`).
    /// Each object is added to the domain of every type present in its type union.
    pub fn with_typed_list(mut self, objects: TypedList<ObjectId, TypeId>) -> Self {
        // 1. Group objects by individual TypeId
        let mut grouped: HashMap<TypeId, Vec<ObjectId>> = HashMap::new();

        for ts in objects {
            // ts.ty().members() provides an iter or slice of TypeId
            for tid in ts.ty().members() {
                grouped.entry(*tid)
                    .or_default()
                    .push(ts.symbol());
            }
        }

        if grouped.is_empty() {
            return self;
        }

        // 2. Determine the maximum ID for vector sizing
        let max_id = grouped.keys()
            .map(|&tid| usize::from(tid))
            .max()
            .unwrap_or(0);

        // 3. Initialize the domain vector with padding for non-contiguous IDs
        // We ensure the vector is large enough to be indexed by any valid TypeId
        self.type_domains = vec![ValueDomain::new(Vec::new()); max_id + 1];

        // 4. Fill the domains
        for (tid, mut objs) in grouped {
            // Sort and dedup is mandatory: an object might belong to multiple
            // types in the hierarchy or be declared redundantly.
            objs.sort_unstable();
            objs.dedup();

            self.type_domains[usize::from(tid)] = ValueDomain::new(objs);
        }

        self
    }
    
    /// Scans the problem to extract raw objects categorized by type.
    ///
    /// Uses `init_size` to pre-allocate internal buckets, minimizing the 
    /// memory management overhead.
    fn collect_objects(&self, problem: &LiftedProblem) -> Vec<Vec<ObjectId>> {
        let num_types = problem.type_defs().len();

        // repeat_with ensures each inner Vec is initialized with its own capacity.
        let mut tmp_objects: Vec<Vec<ObjectId>> = std::iter::repeat_with(|| Vec::with_capacity(self.init_size))
            .take(num_types)
            .collect();

        for typed_object in problem.object_defs() {
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
