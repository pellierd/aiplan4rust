use crate::aiplan4rust::lang::{ObjectId, SymbolId};
use crate::aiplan4rust::lir::old::problem::LiftedProblem;

/// Resolves a `SymbolId` into its string representation using the interner.
///
/// # Arguments
/// * `symbol_id` - Optional reference to a symbol ID.
/// * `lifted_problem` - Reference to the problem containing the interner.
/// * `fallback` - Text to return if the symbol cannot be resolved.
pub fn resolve_name(
    symbol_id: Option<&SymbolId>,
    lifted_problem: &LiftedProblem,
    fallback: &str,
) -> String {
    symbol_id
        .and_then(|sid| lifted_problem.interner().resolve_symbol(*sid))
        .map(|s| s.to_string())
        .unwrap_or_else(|| fallback.to_string())
}

/// Formats a slice of `ObjectId`s into a readable string like "(obj1, obj2, ...)".
///
/// If an object cannot be resolved, it falls back to a placeholder containing its raw index.
///
/// # Arguments
/// * `args` - The list of object IDs to render.
/// * `lifted_problem` - Reference to the problem to look up object names.
pub fn render_args(args: &[ObjectId], lifted_problem: &LiftedProblem) -> String {
    if args.is_empty() {
        return "()".to_string();
    }

    let names: Vec<String> = args
        .iter()
        .map(|&obj_id| {
            // Get the symbol ID for this object
            let symbol_id = lifted_problem.object_symbols().try_get_ident(obj_id).ok();

            // Resolve the name, falling back to the raw ID index if resolution fails
            resolve_name(symbol_id, lifted_problem, &format!("{}", obj_id))
        })
        .collect();

    format!("({})", names.join(", "))
}
