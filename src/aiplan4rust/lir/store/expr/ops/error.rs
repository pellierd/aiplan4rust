use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::lir::store::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::store::expr::error::StorerError;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExprOpErrorHC {
    #[error(transparent)]
    Store(#[from] StorerError),

    #[error(transparent)]
    ExpBuilder(#[from] ExprBuilderError),

    /// Indicates an illegal nesting of temporal operators (e.g., 'at start' inside 'at end').
    /// This is typically caught during parsing or initial expression building.
    #[error("Illegal Temporal Nesting: Cannot nest temporal operator {nested_kind:?} inside {parent_kind:?} at node {id:?}")]
    IllegalTemporalNesting {
        /// The ID of the node where the violation occurred.
        id: ExprId,
        /// The kind of the parent temporal operator.
        parent_kind: ExprEntryKind,
        /// The kind of the nested temporal operator that is forbidden.
        nested_kind: ExprEntryKind,
    },
}

impl ExprOpErrorHC {
    /// Creates an `IllegalTemporalNesting` error variant and captures the call site.
    #[track_caller]
    pub fn illegal_temporal_nesting(
        id: ExprId,
        parent_kind: ExprEntryKind,
        nested_kind: ExprEntryKind,
    ) -> Self {
        ExprOpErrorHC::IllegalTemporalNesting {
            id,
            parent_kind,
            nested_kind,
        }
        .trace()
    }
}

impl Traceable for ExprOpErrorHC {}
