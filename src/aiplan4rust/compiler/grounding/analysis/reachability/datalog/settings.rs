//! Auxiliary Predicate Allocation and Rule Repair Phase.
//!
//! This module is responsible for transforming complex logical formulas (such as nested
//! AND/OR blocks) into flat Horn clauses by introducing uniquely identified auxiliary predicates.
//!
//! # Core Optimizations & Architecture
//!
//! To maximize performance and throughput during the grounding phase, this module bypasses
//! traditional heap-allocated structures (`Vec`, `HashSet`) in favor of hardware-level operations:
//!
//! - **Local Variable Scoping:** Variables are indexed locally, resetting to `0` at the beginning of
//!   each action/scope. This guarantees that local variable IDs remain tightly packed and highly bounded.
//! - **CPU Register Bitmasks:** Unique variable tracking and coverage checks are performed using
//!   bitsets fitting entirely inside standard CPU registers ([`settings::VariableMask`]). Operations like
//!   deduplication or presence checks compile down to ultra-fast bitwise AND/OR instructions.
//! - **Bit-Scanning Instructions:** Decoding variable masks leverages native processor instructions
//!   (`count_ones` for immediate allocation and `trailing_zeros` to fast-forward to set bits),
//!   achieving $O(1)$ control flow loops.
//! - **Stack Allocation Preservation:** By utilizing customized [`SmallVec`] boundaries
//!   ([`AuxPredicateBody`] and [`AtomArgs`]), all predicates, signatures, and terms reside
//!   entirely on the stack for standard arities, completely avoiding heap allocation fragmentation.
//!
//! # Safety & Constraints
//!
//! Due to the register-backed nature of the bitmasks, a scope cannot exceed the maximum bit-width
//! defined by [`settings::MAX_VARIABLES_PER_SCOPE`]. Explicit overflow guards are enforced on
//! every mask computation to ensure compile-time and runtime safety.

use std::mem::size_of;

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

/// The underlying primitive integer type used for tracking variable existence via a bitmask.
///
/// Switching this type (e.g., from `u64` to `u128`) automatically scales the entire
/// compilation and encoding pipeline capacity without breaking downstream bitwise operations.
pub type VariableMask = u64;

/// The maximum number of unique variables permitted within a single local scope (e.g., an action or rule).
///
/// This limit is automatically computed based on the bit-width of [`VariableMask`] to ensure
/// compile-time safety and prevent undefined behavior or bitwise overflow during encoding.
pub const MAX_VARIABLES_PER_SCOPE: usize = size_of::<VariableMask>() * 8;
