use std::backtrace::Backtrace;
use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::lir::problem::symbol_table::IndexTableError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{StringID, Type};
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::grounding::iterator::DomainIteratorError;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;

#[derive(Debug, Error)]
pub enum GroundingError {

    #[error(transparent)]
    Arena(#[from] ArenaError),

    #[error(transparent)]
    SyntaxTree(#[from] SyntaxTreeError),

    #[error(transparent)]
    Expr(#[from] ExprError),

    #[error(transparent)]
    Interner(#[from] InternerError),

    #[error(transparent)]
    IndexTable(#[from] IndexTableError),

    #[error(transparent)]
    DomainIteratorError(#[from] DomainIteratorError),


    #[error(transparent)]
    Lir(#[from] LirError),

    /// A type is not flattened: has more than one super-type
    #[error("Type {0}' is not flattened")]
    NonFlattenedType(Type<StringID>),

    #[error(transparent)]
    Inertia(#[from] InertiaError),
}

impl GroundingError {

    pub fn non_flattened_type_error(ty: &Type<StringID>) -> GroundingError {
        let bt = Backtrace::capture();
        eprintln!(
            "[DEBUG] NonFlattenedType encountered: {:?}\nBacktrace:\n{}",
            ty,
            bt
        );

        GroundingError::NonFlattenedType(ty.clone())
    }
}
