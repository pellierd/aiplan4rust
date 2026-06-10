use crate::aiplan4rust::error::Traceable;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, FunctionSkeletonId, ObjectId, PreferenceSymbolId, TaskSkeletonId, TypeId,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LiftedProblemError {
    // A definition was provided for a typing that was never registered in the symbol table.
    #[error("Type definition provided for an unregistered ID: {id:?}")]
    TypeDefinitionOrphan { id: TypeId },

    // In the LirError enum
    #[error("Object definition provided for an unregistered ID: {id:?}")]
    ObjectDefinitionOrphan { id: ObjectId },

    #[error("Predicate definition requested for an unregistered ID: {id:?}")]
    PredicateDefinitionOrphan { id: AtomSkeletonId },

    #[error("Function definition requested for an unregistered ID: {id:?}")]
    FunctionDefinitionOrphan { id: FunctionSkeletonId },

    // In your LirError enum
    #[error("Task definition requested for an unregistered ID: {id:?}")]
    TaskDefinitionOrphan { id: TaskSkeletonId },

    // Dans l'enum LirError
    #[error("Preference definition requested for an unregistered ID: {id:?}")]
    PreferenceDefinitionOrphan { id: PreferenceSymbolId },
}

impl LiftedProblemError {
    #[track_caller]
    pub fn type_definition_orphan(id: TypeId) -> Self {
        Self::TypeDefinitionOrphan { id }.trace()
    }

    // In the LirError impl block
    #[track_caller]
    pub fn object_definition_orphan(id: ObjectId) -> Self {
        Self::ObjectDefinitionOrphan { id }.trace()
    }

    #[track_caller]
    pub fn predicate_definition_orphan(id: AtomSkeletonId) -> Self {
        Self::PredicateDefinitionOrphan { id }
    }

    /// Constructeur pour l'erreur de fonction

    #[track_caller]
    pub fn function_definition_orphan(id: FunctionSkeletonId) -> Self {
        Self::FunctionDefinitionOrphan { id }.trace()
    }

    // In your impl LirError block
    #[track_caller]
    pub fn task_definition_orphan(id: TaskSkeletonId) -> Self {
        Self::TaskDefinitionOrphan { id }.trace()
    }

    #[track_caller]
    pub fn preference_definition_orphan(id: PreferenceSymbolId) -> Self {
        Self::PreferenceDefinitionOrphan { id }.trace()
    }
}
/// Allows [`ExprBuilderError`] to be augmented with stack trace information.
impl Traceable for LiftedProblemError {}
