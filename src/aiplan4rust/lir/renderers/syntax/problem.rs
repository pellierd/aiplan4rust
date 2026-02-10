use std::fmt::{self, Formatter};
use crate::aiplan4rust::lir::problem::ProblemDef;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, ty, task_network, typed_list};

/// Rendu complet d'une définition de problème (PDDL/HDDL).
pub fn render(f: &mut Formatter<'_>, problem: &ProblemDef<'_>, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête du problème
    write!(
        f,
        "(define (problem {})\n  (:domain {})",
        ctx.resolve_symbol(problem.problem_name()),
        ctx.resolve_symbol(problem.domain_name())
    )?;

    // 2. Requirements (si présents dans le problème)
    let reqs = problem.requirements();
    if !reqs.is_empty() {
        write!(f, "\n  (:requirements")?;
        for req in reqs {
            write!(f, "\n    {}", req.to_string())?;
        }
        write!(f, ")")?;
    }

    // 3. Objects
    if problem.has_object_defs() {
        write!(f, "\n  (:objects\n    ")?;
        typed_list::render_typed_object_list(f, problem.object_defs(), ctx)?;
        write!(f, "\n  )")?;
    }

    // 4. Initial State (:init)
    // Note: L'expression init contient généralement une liste d'atomes (AND)
    if !problem.init().is_empty() {
        write!(f, "\n  (:init ")?;
        expr::render(f, problem.init(), ctx)?;
        write!(f, ")")?;
    }

    // 5. Goal State (:goal)
    if !problem.goal().is_empty() {
        write!(f, "\n  (:goal ")?;
        expr::render(f, problem.goal(), ctx)?;
        write!(f, ")")?;
    }

    // 6. Initial Task Network (:htn - Spécifique HDDL)
    // On vérifie si le réseau contient des tâches avant de l'afficher
    write!(f, "\n  ")?;
    task_network::render_init_task_network(f, problem.initial_task_network(), ctx)?;

    // 7. Constraints (Problem level)
    if !problem.constraints().is_empty() {
        write!(f, "\n  (:constraints ")?;
        expr::render(f, problem.constraints(), ctx)?;
        write!(f, ")")?;
    }

    // 8. Metric
    if !problem.metric_spec().is_empty() {
        write!(f, "\n  (:metric ")?;
        expr::render(f, problem.metric_spec(), ctx)?;
        write!(f, ")")?;
    }

    // 9. Length Spec
    if !problem.length_spec().is_empty() {
        write!(f, "\n  (:length ")?;
        expr::render(f, problem.length_spec(), ctx)?;
        write!(f, ")")?;
    }

    // Fermeture finale
    write!(f, "\n)")?;

    Ok(())
}
