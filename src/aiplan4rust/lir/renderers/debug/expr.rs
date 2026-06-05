use crate::aiplan4rust::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::lir::renderers::RenderContext;
use std::fmt;

pub fn render(f: &mut fmt::Formatter<'_>, root_id: ExprId, ctx: &RenderContext) -> fmt::Result {
    if root_id == ExprId::default() {
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
    kind: &ExprKind,
    ctx: &RenderContext,
) -> fmt::Result {
    match kind {
        // --- Terminaux Variables & Labels ---
        ExprKind::Variable(id) => write!(f, "{} -> ?x{}", id, id.as_usize()),
        ExprKind::TaskLabel(id) => write!(f, "{} -> t{}", id, id.as_usize()),

        // --- Terminaux Symboles (Directs) ---
        ExprKind::PredicateSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_predicate(*id)),
        ExprKind::FunctionSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_functor(*id)),
        ExprKind::TaskSymbol(id) => write!(f, "{} -> {}", id, ctx.resolve_task_symbol(*id)),
        ExprKind::Object(id) => write!(f, "{} -> {}", id, ctx.resolve_object(*id)),
        ExprKind::PrefName(id) => write!(f, "{}", id), // À étendre si resolve_preference existe

        // --- Squelettes (Résolution via le Store + Interner) ---
        ExprKind::AtomicFormula(id) => {
            write!(f, "{}", id)
        }
        ExprKind::Function(id) => {
            write!(f, "{}", id)
        }
        ExprKind::Task(id) => write!(f, "{}", id),

        // --- Valeurs et Opérateurs (On utilise leur Display/Debug) ---
        ExprKind::Number(n) => write!(f, "{}", n),
        ExprKind::Comparison(op) => write!(f, "{}", op),
        ExprKind::Assignment(op) => write!(f, "{}", op),
        ExprKind::Arithmetic(op) => write!(f, "{}", op),
        ExprKind::Metric(op) => write!(f, "{}", op),

        // --- Quantificateurs ---
        ExprKind::Forall(vars) | ExprKind::Exists(vars) => {
            write!(f, "len: {}", vars.len())
        }

        // --- Tout le reste (And, Or, Not, AtStart, etc.) ---
        _ => write!(f, "Keyword: '{}'", kind.to_pddl_keyword()),
    }
}
