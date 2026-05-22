use crate::aiplan4rust::lir::problem::ProblemDef;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, initial_task_network, typed_list};
use std::fmt::{self, Formatter};

/// Rendu complet d'une définition de problème (PDDL/HDDL Syntax version).
pub fn render(f: &mut Formatter<'_>, problem: &ProblemDef<'_>, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête : (define (problem ...) (:domain ...))
    writeln!(
        f,
        "(define (problem {})",
        ctx.resolve_symbol(problem.problem_name())
    )?;
    writeln!(
        f,
        "  (:domain {})",
        ctx.resolve_symbol(problem.domain_name())
    )?;

    // 2. Requirements (souvent hérités, mais parfois spécifiques au problème)
    let reqs = problem.requirements();
    if !reqs.is_empty() {
        write!(f, "  (:requirements")?;
        for req in reqs {
            let req_str = req.to_string().to_lowercase();
            let clean_req = if req_str.starts_with(':') {
                req_str
            } else {
                format!(":{}", req_str)
            };
            write!(f, " {}", clean_req)?;
        }
        writeln!(f, ")")?;
    }

    // 3. Objects
    if problem.has_object_defs() {
        write!(f, "  (:objects ")?;
        typed_list::render_typed_object_list(f, problem.object_defs(), ctx)?;
        writeln!(f, ")")?;
    }

    // 4. Initial State (:init)
    // En PDDL, :init attend une suite de faits. Si ton 'init' est un (and (f1) (f2)),
    // on l'affiche souvent sans le 'and' racine pour plus de compatibilité.
    if problem.init().is_some() {
        write!(f, "  (:init ")?;
        expr::render(f, problem.init(), ctx)?;
        writeln!(f, ")")?;
    }

    // 5. Goal State (:goal)
    if problem.goal().is_some() {
        write!(f, "  (:goal ")?;
        expr::render(f, problem.goal(), ctx)?;
        writeln!(f, ")")?;
    }

    // 6. Initial Task Network (:htn - Spécifique HDDL)
    // On ne l'affiche que s'il y a effectivement des tâches ou un réseau défini
    if problem.initial_task_network().is_empty() {
        write!(f, "  ")?;
        initial_task_network::render(f, problem.initial_task_network(), ctx)?;
        writeln!(f)?;
    }

    // 7. Constraints
    if problem.constraints().is_some() {
        write!(f, "  (:constraints ")?;
        expr::render(f, problem.constraints(), ctx)?;
        writeln!(f, ")")?;
    }

    // 8. Metric
    if problem.metric_spec().is_some() {
        write!(f, "  (:metric ")?;
        expr::render(f, problem.metric_spec(), ctx)?;
        writeln!(f, ")")?;
    }

    // 9. Length Spec
    if problem.length_spec().is_some() {
        write!(f, "  (:length ")?;
        expr::render(f, problem.length_spec(), ctx)?;
        writeln!(f, ")")?;
    }

    // Fermeture finale
    write!(f, ")")
}
