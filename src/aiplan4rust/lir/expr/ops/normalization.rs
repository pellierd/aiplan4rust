use crate::aiplan4rust::lir::expr::{ops, Expr};
use crate::aiplan4rust::lir::expr::ops::ExprOpError;

pub fn normalize(expr: &mut Expr) -> Result<(), ExprOpError> {
    let Some(root_id) = expr.root_id() else { return Ok(()); };

    // 1. Mise en forme logique
    ops::rewriting::eliminate_imply(root_id, expr)?;
    ops::rewriting::push_negation(root_id, expr)?;

    // 2. Mise en forme temporelle
    if ops::rewriting::push_time_specifier(root_id, expr)? {
        ops::rewriting::factorize_time_specifier(root_id, expr)?;
    }

    // 3. Simplification Post-Order (utilise le trait)
    ops::simplify_with(expr, None)?;

    Ok(())
}
