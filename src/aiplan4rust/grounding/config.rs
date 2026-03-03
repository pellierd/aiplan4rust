/// The default initial capacity allocated for each type's object list within the evaluator.
///
/// # Technical Impact
/// This value determines the `with_capacity` size for the internal `Vec<ObjectId>`
/// during the collection phase.
///
/// * **Low value (e.g., 4-8)**: Saves memory in domains with many empty or
///   highly specialized types but may cause multiple reallocations.
/// * **High value (e.g., 64-128)**: Faster for object-heavy domains (Logistics,
///   Blocks-World) by avoiding heap reallocations, but increases the memory
///   footprint for simple problems.
///
/// # Default
/// Set to `16` as a balanced heuristic for standard IPC benchmarks.
pub const DEFAULT_VALUE_REGISTRY_SIZE: usize = 16;


pub const DEFAULT_MAX_ARITY: usize = 15;

/// Default maximum number of constants to project for index-based lookups.
pub const DEFAULT_MAX_PROJ: usize = 3;
