/// The debug initial capacity allocated for each typing's object list within the evaluator.
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

/// The maximum number of arguments a predicate or function can possess by default.
///
/// This serves as a structural safety boundary used during `check_invariants` to prevent
/// the processing of excessively large signatures that could overflow static constraints
/// or downstream fixed bitmasks.
pub const DEFAULT_MAX_ARITY: usize = 15;

/// The fallback maximum projection size allowed for combinatorial indexing.
///
/// This configuration restricts the size of the sub-vectors evaluated during partial
/// instantiation analysis (Definition 7 & 8 of the IPP paper). It prevents exponential
/// explosion when calculating the power set of combinations for highly variable fluents.
pub const DEFAULT_MAX_PROJECTION: usize = 4;
