use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::atomic_skeleton::task::Task;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::passes::types::typed_list;

/// Flattens all types within a `Task` parameters list in place according to the provided mapping.
///
/// This function updates the task's parameter definitions by replacing any union types
/// (`Type::Either`) with their corresponding flattened primitive type identifiers
/// found in the `map`.
///
/// # Parameters
/// - `task`: The `Task` (HTN) whose parameters need to be flattened.
/// - `map`: A mapping from complex union types to their unique flattened `TypeID` (pivots).
///
/// # Returns
/// - `Ok(())` if the parameters were successfully updated or were already primitive.
/// - `Err(LirError)` if a parameter's type is a union not found in the provided mapping.
///
/// # Implementation Note
/// This relies on `typed_list::flatten_typed_variable_list` to handle the actual
/// substitution within the `NamedTypedList` of the task.
pub fn flatten(
    task: &mut Task,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    typed_list::flatten_typed_variable_list(task.parameters_mut(), map)
}
