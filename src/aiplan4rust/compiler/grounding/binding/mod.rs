/// ### Grounding Binding Sub-System
///
/// This module exposes the core primitives, data structures, and engine loops
/// required to perform variable substitution (`binding`) and static structural
/// pruning (Köhler pruning) across expression trees.
///
/// The design relies on a decoupled architecture where memory management is
/// offloaded to recycled scratchpads to achieve zero-allocation profiles on the
/// hot path of PDDL domain grounding.
pub mod bindings;
pub mod evaluator;
pub mod iter;

mod error;
mod expr;
mod scratchpad;

// --- Public API Re-exports ---

#[doc(inline)]
pub use bindings::Bindings;

#[doc(inline)]
pub use error::BindingError;

#[doc(inline)]
pub use scratchpad::BindingScratchpad;

#[doc(inline)]
pub use expr::{bind, bind_with};
