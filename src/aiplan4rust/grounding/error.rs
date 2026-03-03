use std::backtrace::Backtrace;
use thiserror::Error;
use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::lir::problem::symbol_registry::IndexTableError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::grounding::analysis::inertia::registry::InertiaRegistryError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::binding::BindingError;
use crate::aiplan4rust::grounding::binding::iter::BindingsIteratorError;
use crate::aiplan4rust::lir::expr::ExprError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::analysis::inertia::table::InertiaTableError;

#[derive(Debug, Error)]
pub enum GroundingError {

    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    #[error(transparent)]
    InertiaRegistry(#[from] InertiaRegistryError),

    #[error(transparent)]
    Datalog(#[from] DatalogError),

    #[error(transparent)]
    BindingEngine(#[from] BindingError),

    #[error(transparent)]
    Logic(#[from] ExprOpError),

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
    DomainIteratorError(#[from] BindingsIteratorError),


    #[error(transparent)]
    Lir(#[from] LirError),

    /// A type is not flattened: has more than one super-type
    #[error("Type {0}' is not flattened")]
    NonFlattenedType(Type<SymbolId>),

    #[error(transparent)]
    Inertia(#[from] InertiaError),
}

impl GroundingError {

    pub fn non_flattened_type_error(ty: &Type<SymbolId>) -> GroundingError {
        let bt = Backtrace::capture();
        eprintln!(
            "[DEBUG] NonFlattenedType encountered: {:?}\nBacktrace:\n{}",
            ty,
            bt
        );

        GroundingError::NonFlattenedType(ty.clone())
    }
}
