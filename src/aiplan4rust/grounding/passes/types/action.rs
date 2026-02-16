use std::collections::HashMap;
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::error::LirError;
use crate::aiplan4rust::grounding::passes::types::{expr, typed_list};

/// Flattens all union types (`Type::Either`) within an `Action` in place.
///
/// This function performs a complete transformation of the action by:
/// 1. Flattening the **parameters** in the header via `typed_list`.
/// 2. Flattening the **precondition** expression tree via `expr`.
/// 3. Flattening the **effect** expression tree via `expr`.
///
/// # Parameters
/// - `action`: The `Action` structure to modify.
/// - `map`: A mapping from union types to their unique flattened primitive `TypeID` (pivots).
///
/// # Returns
/// - `Ok(())` if the header, precondition, and effect were successfully flattened.
/// - `Err(LirError)` if any part of the action refers to a union type missing from the mapping.
///
/// # Implementation Note
/// It is vital to types both preconditions and effects because they often
/// contain quantified variables (forall/exists) or refer to object types that
/// must match the flattened domain.
/// Aplatit tous les types d'union (`Type::Either`) au sein d'une `Action` en place.
///
/// Cette fonction effectue une transformation complète de l'action en :
/// 1. Aplatissant les **paramètres** de la signature.
/// 2. Aplatissant l'expression de la **durée** (si présente).
/// 3. Aplatissant l'arbre d'expression des **préconditions** (ou conditions).
/// 4. Aplatissant l'arbre d'expression des **effets**.
///
/// # Paramètres
/// - `action`: La structure `Action` à modifier.
/// - `map`: Une table de hachage associant les types d'union à leur `TypeID` primitif unique.
///
/// # Returns
/// - `Ok(())` si tous les composants ont été aplatis avec succès.
/// - `Err(LirError)` si une partie de l'action fait référence à un type d'union absent de la table.
pub fn flatten(
    action: &mut LiftedAction,
    map: &HashMap<Type<TypeID>, TypeID>,
) -> Result<(), LirError> {
    // 1. Aplatissement des paramètres dans l'en-tête de l'action
    typed_list::flatten_typed_variable_list(action.parameters_mut(), map)?;

    // 2. Aplatissement de la durée (spécifique aux actions temporelles)
    if let Some(duration_mut) = action.duration_mut() {
        expr::flatten(duration_mut, map)?;
    }

    // 3. Aplatissement des préconditions (ou conditions temporelles)
    expr::flatten(action.precondition_mut(), map)?;

    // 4. Aplatissement des effets
    expr::flatten(action.effect_mut(), map)?;

    Ok(())
}
