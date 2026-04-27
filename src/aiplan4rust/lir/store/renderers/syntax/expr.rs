use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

pub fn render(f: &mut fmt::Formatter<'_>, root_id: ExprId, ctx: &RenderContext) -> fmt::Result {
    if root_id == ExprId::NONE {
        return write!(f, "    <Empty Expression>");
    }

    let mut ancestor_is_last = Vec::new();

    for (id, depth, is_last, entry) in ctx.store().tree_preorder(root_id) {
        ancestor_is_last.truncate(depth);

        // --- Dessin de l'arbre ---
        write!(f, "    ")?;
        for &parent_was_last in ancestor_is_last.iter() {
            if parent_was_last {
                write!(f, "   ")?;
            } else {
                write!(f, "│  ")?;
            }
        }
        if depth > 0 {
            write!(f, "{}", if is_last { "└─ " } else { "├─ " })?;
        }
        ancestor_is_last.push(is_last);

        // --- Affichage du Kind (via son Display implémenté) ---
        let kind = entry.kind();
        write!(f, "{}", kind)?;

        // --- Affichage de la Résolution Hybride ---
        write!(f, " [")?;
        render_hybrid_content(f, kind, ctx)?;
        write!(f, "]")?;

        // --- ID technique du Store ---
        writeln!(f, " ({})", id)?;
    }
    Ok(())
}

fn render_hybrid_content(
    f: &mut fmt::Formatter<'_>,
    kind: &ExprEntryKind,
    ctx: &RenderContext,
) -> fmt::Result {
    match kind {
        // --- Terminaux Variables & Labels ---
        ExprEntryKind::Variable(id) => write!(f, "{} -> ?x{}", id, id.as_usize()),
        ExprEntryKind::TaskLabel(id) => write!(f, "{} -> t{}", id, id.as_usize()),

        // --- Terminaux Symboles (Directs) ---
        ExprEntryKind::PredicateSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_predicate(*id)),
        ExprEntryKind::FunctionSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_functor(*id)),
        ExprEntryKind::TaskSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_task_symbol(*id)),
        ExprEntryKind::Object(id) => write!(f, "{} -> {}", id, ctx.resolve_object(*id)),
        ExprEntryKind::PrefName(id) => write!(f, "{}", id), // À étendre si resolve_preference existe

        // --- Squelettes (Résolution via le Store + Interner) ---
        ExprEntryKind::AtomicFormula(id) => {
            write!(f, "{}", id)
        }
        ExprEntryKind::Function(id) => {
            write!(f, "{}", id)
        }
        ExprEntryKind::Task(id) => write!(f, "{}", id),

        // --- Valeurs et Opérateurs (On utilise leur Display/Debug) ---
        ExprEntryKind::Number(n) => write!(f, "{}", n),
        ExprEntryKind::Comparison(op) => write!(f, "{}", op),
        ExprEntryKind::Assignment(op) => write!(f, "{}", op),
        ExprEntryKind::Arithmetic(op) => write!(f, "{}", op),
        ExprEntryKind::Metric(op) => write!(f, "{}", op),

        // --- Quantificateurs ---
        ExprEntryKind::Forall(vars) | ExprEntryKind::Exists(vars) => {
            write!(f, "len: {}", vars.len())
        }

        // --- Tout le reste (And, Or, Not, AtStart, etc.) ---
        _ => write!(f, "Keyword: '{}'", kind.to_pddl_keyword()),
    }
}
