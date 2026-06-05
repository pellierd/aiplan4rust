use crate::aiplan4rust::arena::ArenaError;
use crate::aiplan4rust::grounding::analysis::inertia::new_evaluator::InertiaRegistryError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::binding::iter::BindingsIteratorError;
use crate::aiplan4rust::grounding::binding::BindingError;
use crate::aiplan4rust::grounding::problem::registry::value::error::ValueRegistryError;
use crate::aiplan4rust::interner::InternerError;
use crate::aiplan4rust::lang::{SymbolId, Type};
use crate::aiplan4rust::lir::expr::builder::ExprBuilderError;
use crate::aiplan4rust::lir::expr::error::StorerError;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::old::expr::ExprError;
use crate::aiplan4rust::lir::old::problem::symbol_registry::IndexTableError;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::tree::error::SyntaxTreeError;
use crate::analysis::inertia::error::InertiaError;
use crate::analysis::inertia::new_table::InertiaTableError;
use std::backtrace::Backtrace;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GroundingError {
    #[error(transparent)]
    StorerError(#[from] StorerError),

    #[error(transparent)]
    ExprBuilder(#[from] ExprBuilderError),

    #[error(transparent)]
    Registry(#[from] ValueRegistryError),

    #[error(transparent)]
    ExprOp(#[from] ExprOpError),

    #[error(transparent)]
    InertiaTable(#[from] InertiaTableError),

    #[error(transparent)]
    InertiaRegistry(#[from] InertiaRegistryError),

    #[error(transparent)]
    Datalog(#[from] DatalogError),

    #[error(transparent)]
    BindingEngine(#[from] BindingError),

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

    /// A typing is not flattened: has more than one super-typing
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
            ty, bt
        );

        GroundingError::NonFlattenedType(ty.clone())
    }
}
