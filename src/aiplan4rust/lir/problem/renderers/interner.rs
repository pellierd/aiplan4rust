use std::fmt::Write;
use crate::aiplan4rust::lir::problem::{
    InitialTaskNetwork, LiftedAction, LiftedMethod, LiftedProblem, LiftedTaskNetwork,
};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::problem::renderers::common::writeln_centered;

pub fn render_problem(
    f: &mut impl Write,
    problem: &LiftedProblem,
    interner: &StringInterner,
) -> std::fmt::Result {
    // Titre principal
    writeln_centered(f, "PROBLEM", 80, '=')?;
    writeln!(
        f,
        "DOMAIN NAME  : {}",
        interner.resolve_ident(problem.domain_name()).unwrap_or("<unknown>")
    )?;
    writeln!(
        f,
        "PROBLEM NAME : {}\n",
        interner.resolve_ident(problem.problem_name()).unwrap_or("<unknown>")
    )?;

    // Requirements
    writeln_centered(f, "REQUIREMENTS", 80, '=')?;
    let mut reqs: Vec<_> = problem.requirements().iter().collect();
    if reqs.is_empty() {
        writeln!(f, "  - no requirements")?;
    } else {
        reqs.sort();
        for r in reqs {
            writeln!(f, "  - {}", r)?;
        }
    }
    writeln!(f)?;

    // Types
    writeln_centered(f, "TYPES", 80, '=')?;
    if problem.types().is_empty() {
        writeln!(f, "  - no types")?;
    } else {
        for t in problem.types() {
            writeln!(f, "  - {}", t.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if problem.constants().is_empty() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in problem.constants() {
            writeln!(f, "  - {}", c.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Predicates
    writeln_centered(f, "PREDICATES", 80, '=')?;
    if problem.predicates().is_empty() {
        writeln!(f, "  - no predicates")?;
    } else {
        for p in problem.predicates() {
            writeln!(f, "  - {}", p.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Functions
    writeln_centered(f, "FUNCTIONS", 80, '=')?;
    if problem.functions().is_empty() {
        writeln!(f, "  - no functions")?;
    } else {
        for func in problem.functions() {
            writeln!(f, "  - {}", func.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = problem.domain_constraints();
    writeln!(f, "{}\n", dc.to_string_with_interner(interner))?;

    // Actions
    if problem.actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in problem.actions() {
            render_action(f, action, interner)?;
            writeln!(f)?;
        }
    }

    // Methods
    if problem.methods().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in problem.methods() {
            render_method(f, method, interner)?;
            writeln!(f)?;
        }
    }

    // Init
    writeln_centered(f, "INIT", 80, '=')?;
    writeln!(f, "{}\n", problem.init().to_string_with_interner(interner))?;

    // Goal
    writeln_centered(f, "GOAL", 80, '=')?;
    writeln!(f, "{}\n", problem.goal().to_string_with_interner(interner))?;

    // Problem constraints
    writeln_centered(f, "PROBLEM CONSTRAINTS", 80, '=')?;
    writeln!(f, "{}\n", problem.problem_constraints().to_string_with_interner(interner))?;

    // Metric spec
    writeln_centered(f, "METRIC SPEC", 80, '=')?;
    writeln!(f, "{}\n", problem.metric_spec().to_string_with_interner(interner))?;

    // Length spec
    writeln_centered(f, "LENGTH SPEC", 80, '=')?;
    writeln!(f, "{}\n", problem.length_spec().to_string_with_interner(interner))?;

    // Initial Task Network
    writeln_centered(f, "INITIAL TASK NETWORK", 80, '=')?;
    render_initial_task_network(f, problem.initial_task_network(), interner)?;
    writeln!(f)?;

    Ok(())
}

pub fn render_action(
    f: &mut impl Write,
    action: &LiftedAction,
    interner: &StringInterner,
) -> std::fmt::Result {
    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "ACTION", 80, '-')?;
    writeln!(
        f,
        "NAME: {}",
        interner.resolve_ident(action.name()).unwrap_or("<unknown>")
    )?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "PRECONDITION:")?;
    for line in action.precondition().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "EFFECT:")?;
    for line in action.effect().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }

    Ok(())
}

pub fn render_method(
    f: &mut impl Write,
    method: &LiftedMethod,
    interner: &StringInterner,
) -> std::fmt::Result {
    let params = method
        .parameters()
        .iter()
        .map(|p| p.to_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "METHOD", 80, '-')?;
    writeln!(
        f,
        "NAME: {}",
        interner.resolve_ident(method.name()).unwrap_or("<unknown>")
    )?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(
        f,
        "  TASK: {}",
        method.task().to_string_with_interner(interner)
    )?;

    writeln!(f, "PRECONDITION:")?;
    let precond_str = method.precondition().to_string_with_interner(interner);
    if precond_str.is_empty() {
        writeln!(f, "  <empty>")?;
    } else {
        for line in precond_str.lines() {
            writeln!(f, "  {}", line)?;
        }
    }

    writeln!(f, "TASK NETWORK:")?;
    render_task_network(f, &method.task_network(), interner)?;

    Ok(())
}

/// Renders a `TaskNetwork` to a formatter using an interner.
pub fn render_task_network(
    f: &mut impl Write,
    network: &LiftedTaskNetwork,
    interner: &StringInterner,
) -> std::fmt::Result {
    writeln!(f, "  TASKS: {}", network.tasks().to_string_with_interner(interner))?;
    writeln!(f, "  ORDERING: {}", network.ordering_constraints().to_string_with_interner(interner))?;
    writeln!(f, "  CONSTRAINTS: {}", network.logical_constraints().to_string_with_interner(interner))?;
    Ok(())
}

/// Renders an `InitialTaskNetwork` to a formatter using an interner.
pub fn render_initial_task_network(
    f: &mut impl Write,
    network: &InitialTaskNetwork,
    interner: &StringInterner,
) -> std::fmt::Result {
    writeln!(f, "PARAMETERS: {}", network.parameters().to_string_with_interner(interner))?;
    let tw = network.task_network();
    writeln!(f, "TASKS: {}", tw.tasks().to_string_with_interner(interner))?;
    writeln!(f, "ORDERING: {}", tw.ordering_constraints().to_string_with_interner(interner))?;
    writeln!(f, "CONSTRAINTS: {}", tw.logical_constraints().to_string_with_interner(interner))?;
Ok(())
}
