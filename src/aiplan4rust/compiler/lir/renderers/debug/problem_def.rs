use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::compiler::lir::problem::ProblemDef;
use crate::aiplan4rust::compiler::lir::renderers::debug::common::writeln_centered;
use crate::aiplan4rust::compiler::lir::renderers::debug::{expr, initial_task_network, typed_list};
use crate::aiplan4rust::compiler::lir::renderers::RenderContext;
use std::fmt;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    problem: &ProblemDef,
    ctx: &RenderContext,
) -> fmt::Result {
    // === HEADER ===
    writeln_centered(f, " [ PROBLEM DEFINITION ] ", 80, '=')?;
    writeln!(
        f,
        "  DOMAIN NAME  : {}",
        ctx.resolve_symbol(problem.domain_name())
    )?;
    writeln!(
        f,
        "  PROBLEM NAME : {}\n",
        ctx.resolve_symbol(problem.problem_name())
    )?;

    // === REQUIREMENTS ===
    writeln_centered(f, " REQUIREMENTS ", 80, '-')?;
    if problem.requirements().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for req in problem.requirements().iter() {
            writeln!(f, "  - {:?}", req)?;
        }
    }
    writeln!(f)?;

    // === OBJECTS ===
    writeln_centered(f, " OBJECTS ", 80, '-')?;
    if problem.object_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        typed_list::render_object_typed_list(f, problem.object_defs(), ctx)?;
    }
    writeln!(f)?;

    // === INIT ===
    // On utilise EMPTY_AND (ID 0) comme valeur "vide" ou "neutre"
    writeln_centered(f, " INITIAL STATE (INIT) ", 80, '-')?;
    if problem.init() == ExprId::EMPTY_AND {
        writeln!(f, "  <Empty/Default>")?;
    } else {
        expr::render(f, problem.init(), ctx)?;
    }
    writeln!(f)?;

    // === GOAL ===
    writeln_centered(f, " GOAL ", 80, '-')?;
    if problem.goal() == ExprId::EMPTY_AND {
        writeln!(f, "  <None/Always Satisfied>")?;
    } else {
        expr::render(f, problem.goal(), ctx)?;
    }
    writeln!(f)?;

    // === INITIAL TASK NETWORK (HDDL) ===
    // Utilisation de is_empty() pour détecter l'ITN par défaut
    if !problem.initial_task_network().is_empty() {
        writeln_centered(f, " INITIAL TASK NETWORK ", 80, '-')?;
        initial_task_network::render(f, problem.initial_task_network(), ctx)?;
        writeln!(f)?;
    }

    // === CONSTRAINTS ===
    // Les contraintes sont souvent EMPTY_AND par défaut
    if problem.constraints() != ExprId::EMPTY_AND {
        writeln_centered(f, " PROBLEM CONSTRAINTS ", 80, '-')?;
        expr::render(f, problem.constraints(), ctx)?;
        writeln!(f)?;
    }

    // === METRIC SPEC ===
    // On utilise NONE (ID MAX) pour l'absence de métrique
    if problem.metric_spec() != ExprId::NONE {
        writeln_centered(f, " METRIC SPEC ", 80, '-')?;
        expr::render(f, problem.metric_spec(), ctx)?;
        writeln!(f)?;
    }

    // === LENGTH SPEC ===
    if problem.length_spec() != ExprId::NONE {
        writeln_centered(f, " LENGTH SPEC ", 80, '-')?;
        expr::render(f, problem.length_spec(), ctx)?;
        writeln!(f)?;
    }

    writeln!(f, "\n{}", "=".repeat(80))
}
