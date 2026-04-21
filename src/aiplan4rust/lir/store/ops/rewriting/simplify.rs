/*use crate::aiplan4rust::lir::expr::ops::{StaticEvaluator, StaticValue};
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::{ExprBuilder, ExprEntryKind, ExprId, ExprStore};
use fxhash::FxHashMap;

pub fn simplify(
    root: ExprId,
    store: &mut ExprStore,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<ExprId, ExprOpErrorHC> {
    let mut substitution_map: FxHashMap<ExprId, ExprId> = FxHashMap::default();

    // Correction ici : on déstructure le tuple renvoyé par l'itérateur
    let postorder_ids: Vec<ExprId> = store.postorder(root).map(|(id, _, _)| id).collect();

    // On utilise le store directement si le builder ne l'expose pas en pub
    for old_id in postorder_ids {
        let old_entry = &store[old_id];

        let new_children: Vec<ExprId> = old_entry
            .children()
            .iter()
            .map(|child_id| *substitution_map.get(child_id).unwrap_or(child_id))
            .collect();

        let kind = old_entry.kind().clone();

        // On crée le builder à l'intérieur ou on le réutilise
        let mut builder = ExprBuilder::new(store);

        // Note : Si 'reconstruct' n'existe pas, utilise le nom exact (ex: construct)
        let reconstructed_id = builder.reconstruct(kind, new_children);

        // On convertit NodeId en ExprId si nécessaire (ici via .into())
        let final_id = simplify_node(reconstructed_id.into(), &mut builder, evaluator)?;

        substitution_map.insert(old_id, final_id);
    }

    Ok(*substitution_map.get(&root).unwrap_or(&root))
}

fn simplify_node(
    id: ExprId,
    builder: &mut ExprBuilder,
    evaluator: Option<&dyn StaticEvaluator>,
) -> Result<ExprId, ExprOpErrorHC> {
    // Si fetch n'est pas pub, utilise get().ok_or(...)
    let entry = builder.fetch(id)?;
    let kind = entry.kind();

    if is_evaluable(kind, evaluator.is_some()) {
        if let Some(eval) = evaluator {
            // Attention : evaluate attend peut-être un &Expr et non un &ExprStore
            // Il faut passer l'objet que ton trait 'StaticEvaluator' attend.
            // On deref 'n' pour OrderedFloat
            if let Some(static_val) = eval.evaluate(id.into(), builder.as_expr()) {
                let new_id = match static_val {
                    StaticValue::Boolean(true) => builder.empty_and(),
                    StaticValue::Boolean(false) => builder.empty_or(),
                    StaticValue::Number(n) => builder.number(*n), // *n pour passer de OrderedFloat à f64
                    StaticValue::Object(o) => builder.constant(o),
                };
                return Ok(new_id.into());
            }
        }
    }

    Ok(id)
}

/// Filtre pour savoir si on doit appeler l'évaluateur statique.
fn is_evaluable(kind: &ExprEntryKind, has_evaluator: bool) -> bool {
    match kind {
        ExprEntryKind::AtomicFormula(_) | ExprEntryKind::Function(_) => has_evaluator,
        ExprEntryKind::Comparison(_) | ExprEntryKind::Arithmetic(_) => true,
        _ => false,
    }
}

// --- Wrappers de commodité ---

pub fn simplify_default(root: ExprId, store: &mut ExprStore) -> Result<ExprId, ExprOpErrorHC> {
    simplify(root, store, None)
}*/
