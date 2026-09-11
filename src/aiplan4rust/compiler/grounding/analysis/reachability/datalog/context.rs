//! Datalog Translation Context Module.
//!
//! This module provides the `DatalogContext` structure, which carries necessary
//! metadata, parameter references, type mappings, and inertia tables across
//! the various compilation and encoding phases of the Datalog pipeline.

use crate::aiplan4rust::support::lang::{AtomSkeletonId, TypedListId};
use crate::analysis::inertia::table::InertiaTable;

/// Encapsulates contextual configuration and reference mappings for Datalog translation.
///
/// `DatalogContext` is a lightweight, copyable container passed across encoder functions
/// to maintain structural awareness of current parameters, type skeletons, negation offsets,
/// and predicate inertia properties.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DatalogContext<'a> {
    pub(crate) param_list_id: TypedListId,
    pub(crate) type_to_skeleton: &'a [AtomSkeletonId],
    pub(crate) negation_offset: usize,
    pub(crate) inertia_table: &'a InertiaTable,
}

impl<'a> DatalogContext<'a> {
    /// Creates a new instance of `DatalogContext` for the encoding pipeline.
    ///
    /// # Arguments
    ///
    /// * `param_list_id` - The identifier of the current parameter list.
    /// * `type_to_skeleton` - A reference slice mapping domain types to their respective atom skeleton IDs.
    /// * `negation_offset` - The offset value used for handling negated predicates.
    /// * `inertia_table` - A reference to the inertia table tracking static vs. fluent predicates.
    ///
    /// # Returns
    ///
    /// Returns a new, initialized `DatalogContext` instance.
    pub(crate) fn new(
        param_list_id: TypedListId,
        type_to_skeleton: &'a [AtomSkeletonId],
        negation_offset: usize,
        inertia_table: &'a InertiaTable,
    ) -> Self {
        Self {
            param_list_id,
            type_to_skeleton,
            negation_offset,
            inertia_table,
        }
    }
}

/// Tests the creation and field initialization of the Datalog translation context.
///
/// # Objective
/// Verifies that `DatalogContext::new` correctly stores and exposes the provided parameter list ID,
/// type skeleton mappings, negation offset, and inertia table reference.
///
/// # Inputs
/// - A default parameter list ID (`TypedListId`).
/// - A vector of type skeletons mapping domain types.
/// - An empty inertia table.
/// - A negation offset integer (`5`).
///
/// # Expected Output
/// Returns a properly initialized `DatalogContext` whose fields match the input parameters exactly.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::inertia::table::InertiaTable;

    #[test]
    fn test_datalog_context_creation() {
        let param_list_id = TypedListId::default();
        let type_skeletons = vec![AtomSkeletonId::from(1), AtomSkeletonId::from(2)];
        let inertia_table = InertiaTable::empty();
        let negation_offset = 5;

        let ctx = DatalogContext::new(
            param_list_id,
            &type_skeletons,
            negation_offset,
            &inertia_table,
        );

        assert_eq!(ctx.param_list_id, param_list_id);
        assert_eq!(ctx.type_to_skeleton.len(), 2);
        assert_eq!(ctx.type_to_skeleton[0], AtomSkeletonId::from(1));
        assert_eq!(ctx.negation_offset, 5);
    }
}
