use crate::aiplan4rust::support::lang::{AtomSkeletonId, TypedListId};
use crate::analysis::inertia::table::InertiaTable;

#[derive(Clone, Copy, Debug)]
pub(crate) struct DatalogContext<'a> {
    pub(crate) param_list_id: TypedListId,
    pub(crate) type_to_skeleton: &'a [AtomSkeletonId],
    pub(crate) negation_offset: usize,
    pub(crate) inertia_table: &'a InertiaTable,
}

impl<'a> DatalogContext<'a> {
    /// Crée une nouvelle instance de `DatalogConfig` pour l'encodage.
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
