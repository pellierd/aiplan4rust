pub mod problem;
pub(crate) mod normalize;
pub mod domain_def;
pub mod problem_def;
pub mod flatten;
pub mod expand;
pub mod symbol_table;

pub use problem::Problem as LiftedProblem;
pub use problem_def::ProblemDef;
pub use domain_def::DomainDef;
pub use crate::aiplan4rust::lir::action::Action as LiftedAction;
pub use crate::aiplan4rust::lir::derived_predicate::DerivedPredicate as LiftedDerivedPredicate;
pub use crate::aiplan4rust::lir::durative_action::DurativeAction as LiftedDurativeAction;
pub use crate::aiplan4rust::lir::method::Method as LiftedMethod;
pub use crate::aiplan4rust::lir::task_network::TaskNetwork as LiftedTaskNetwork;
pub use crate::aiplan4rust::lir::initial_task_network::InitialTaskNetwork;
pub use symbol_table::SymbolTable;
