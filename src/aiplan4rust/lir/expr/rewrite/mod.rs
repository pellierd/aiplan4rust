/// Module `rewrite`
///
/// Contains low-level expression transformations used in the normalization pipeline.
/// Each module here represents a single atomic transformation step. Dependencies
/// between steps are explicit and must be respected by the orchestrating `normalize` module.
///
/// # Modules and dependencies
///
/// ```text
/// eliminate_imply            (independent)
///       ↓
/// push_negation              (depends on eliminate_imply)
///       ↓
/// push_time_specifier        (depends on push_negation)
///       ↓
/// factorize_time_specifier   (depends on push_time_specifier)
/// ```
///
/// Users should **not call these functions directly**; instead, use the `normalize`
/// module which handles the correct order of execution and ensures that all
/// preconditions are satisfied.
pub mod eliminate_imply;
pub mod push_negation;
pub mod push_time_specifier;
pub mod factorize_time_specifier;
