/// The maximum inline (stack-allocated) capacity for a Datalog rule's body.
///
/// Preconditions fitting within this limit avoid heap allocation overhead entirely,
/// preventing heap fragmentation during rule transformation phases.
pub const INLINE_RULE_CAPACITY: usize = 8;

/// The maximum inline capacity utilized during the allocation and repair of auxiliary predicates.
///
/// This threshold is strictly aligned with [`INLINE_RULE_CAPACITY`] to enable immediate,
/// zero-cost ownership transfers ($O(1)$ stack moves) into final rule structures.
pub const INLINE_AUX_PREDICATE_CAPACITY: usize = INLINE_RULE_CAPACITY;

/// The maximum inline (stack-allocated) capacity for a Datalog atom's arguments.
///
/// Atoms with an arity lower than or equal to this limit will reside entirely
/// on the stack within the struct's memory block, bypassing the heap allocator.
pub const INLINE_ATOM_ARGS_CAPACITY: usize = 4;

/// The maximum inline (stack-allocated) capacity for a concrete ground tuple's arguments.
///
/// This threshold is strictly aligned with [`INLINE_ATOM_ARGS_CAPACITY`] to reflect that
/// ground tuples directly mirror the arity of their logical atom counterparts.
pub const INLINE_TUPLE_ARGS_CAPACITY: usize = INLINE_ATOM_ARGS_CAPACITY;

/// Le type brut utilisé pour le bitmask des variables (u64 ou u128)
pub type VariableMask = u64; // Idéalement u64 pour le moment, ou u128 plus tard

/// La limite de variables, calquée automatiquement sur la taille du type choisi
pub const MAX_VARIABLES_PER_SCOPE: usize = size_of::<VariableMask>() * 8;
