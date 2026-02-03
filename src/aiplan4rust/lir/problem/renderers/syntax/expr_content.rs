use std::fmt;
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::problem::renderers::render_context::RenderContext;
use crate::aiplan4rust::lir::problem::renderers::syntax::typed_list;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    content: &Content,
    ctx: &RenderContext,
) -> std::fmt::Result {
    match content {
        Content::None => write!(f, "None"),

        Content::Variable(id) => {
            write!(f, "?x{}", id.as_usize())
        },
        Content::Constant(id) => {
            write!(f, "{}", ctx.resolve_object(*id))
        },
        Content::Parameter(id) => {
            // Les paramètres sont souvent indexés, mais si tu as leur StringID :
            write!(f, "?X{}", id.as_usize())
        },
        Content::Predicate(id) => {
            write!(f, "{}", ctx.resolve_predicate(*id))
        },
        Content::Functor(id) => {
            write!(f, "{}", ctx.resolve_functor(*id))
        },
        Content::TaskSymbol(id) => {
            write!(f, "TO DO")
            //write!(f, "{}", ctx.resolve_task_symbol(*id))
        },

        Content::TaskID(id) => {
            write!(f, "t{}",  id.as_usize())
        },

        Content::Preference(id) => {
            write!(f, "TO DO")
            //write!(f, "pref{}", ctx.resolve_preference(*id))
        }

        // --- Skeletons (On affiche le nom du symbole racine) ---
        Content::AtomSkeleton(id) => {
            // Ici, on va chercher le nom du prédicat associé au skeleton
            // Hypothèse : ton problem contient la liste des skeletons
            write!(f, "AtomSkeleton({})", id.as_usize())
        },

        // --- Valeurs et Opérateurs (Inchangés car techniques) ---
        Content::Float(val)        => write!(f, "{}", val),
        Content::BinaryComp(op)    => write!(f, "{}", op),
        Content::AssignOp(op)      => write!(f, "{}", op),
        Content::ArithmeticOp(op)  => write!(f, "{}", op),
        Content::Optimization(opt) => write!(f, "{}", opt),

        // --- Listes typées ---
        Content::QuantifierVariables(vars) =>
            typed_list::render(f, vars, ctx),

        // Pour les autres IDs techniques, on peut garder le Display par défaut ou enrichir
        Content::FunctionSkeleton(_) => Ok(()),
        Content::TaskSkeleton(_) => Ok(())
    }
}
