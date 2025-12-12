use std::fmt::Write;
use crate::aiplan4rust::lir::problem::{
    InitialTaskNetwork, LiftedAction, LiftedMethod, LiftedProblem, LiftedTaskNetwork,
};
use crate::aiplan4rust::lir::problem::renderers::common::writeln_centered;

pub fn render_problem(f: &mut impl Write, problem: &LiftedProblem) -> std::fmt::Result {
    // Titre principal
    writeln_centered(f, "PROBLEM", 80, '=')?;
    writeln!(f, "DOMAIN NAME  : {}", problem.domain_name())?;
    writeln!(f, "PROBLEM NAME : {}\n", problem.problem_name())?;

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
            writeln!(f, "  - {}", t)?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if problem.constants().is_empty() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in problem.constants() {
            writeln!(f, "  - {}", c)?;
        }
    }
    writeln!(f)?;

    // Predicates
    writeln_centered(f, "PREDICATES", 80, '=')?;
    if problem.predicates().is_empty() {
        writeln!(f, "  - no predicates")?;
    } else {
        for p in problem.predicates() {
            writeln!(f, "  - {}", p)?;
        }
    }
    writeln!(f)?;

    // Functions
    writeln_centered(f, "FUNCTIONS", 80, '=')?;
    if problem.functions().is_empty() {
        writeln!(f, "  - no functions")?;
    } else {
        for fct in problem.functions() {
            writeln!(f, "  - {}", fct)?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = problem.domain_constraints();
    if dc.is_empty() {
        writeln!(f, "  - no domain constraints\n")?;
    } else {
        writeln!(f, "{}\n", dc)?;
    }

    // Actions
    if problem.actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in problem.actions() {
            render_action(f, action)?;
            writeln!(f)?;
        }
    }

    // Methods
    if problem.methods().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in problem.methods() {
            render_method(f, method)?;
            writeln!(f)?;
        }
    }

    // Init
    writeln_centered(f, "INIT", 80, '=')?;
    let init = problem.init();
    if init.is_empty() {
        writeln!(f, "  - no init\n")?;
    } else {
        writeln!(f, "{}\n", init)?;
    }

    // Goal
    writeln_centered(f, "GOAL", 80, '=')?;
    let goal = problem.goal();
    if goal.is_empty() {
        writeln!(f, "  - no goal\n")?;
    } else {
        writeln!(f, "{}\n", goal)?;
    }

    // Problem constraints
    writeln_centered(f, "PROBLEM CONSTRAINTS", 80, '=')?;
    let pc = problem.problem_constraints();
    if pc.is_empty() {
        writeln!(f, "  - no problem constraints\n")?;
    } else {
        writeln!(f, "{}\n", pc)?;
    }

    // Metric spec
    writeln_centered(f, "METRIC SPEC", 80, '=')?;
    let metric = problem.metric_spec();
    if metric.is_empty() {
        writeln!(f, "  - no metric spec\n")?;
    } else {
        writeln!(f, "{}\n", metric)?;
    }

    // Length spec
    writeln_centered(f, "LENGTH SPEC", 80, '=')?;
    let length = problem.length_spec();
    if length.is_empty() {
        writeln!(f, "  - no length spec\n")?;
    } else {
        writeln!(f, "{}\n", length)?;
    }

    // Initial Task Network
    writeln_centered(f, "INITIAL TASK NETWORK", 80, '=')?;
    render_initial_task_network(f, problem.initial_task_network())?;

    Ok(())
}

pub fn render_action(f: &mut impl Write, action: &LiftedAction) -> std::fmt::Result {
    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "ACTION", 80, '-')?;
    writeln!(f, "NAME: {}", action.name())?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "PRECONDITION:")?;
    for line in format!("{}", action.precondition()).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "EFFECT:")?;
    for line in format!("{}", action.effect()).lines() {
        writeln!(f, "  {}", line)?;
    }
    Ok(())
}

pub fn render_method(f: &mut impl Write, method: &LiftedMethod) -> std::fmt::Result {
    let params = method
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "METHOD", 80, '-')?;
    writeln!(f, "NAME: {}", method.name())?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "TASK: {}", method.task())?;
    writeln!(f, "PRECONDITION:")?;
    for line in format!("{}", method.precondition()).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "TASK NETWORK:")?;
    for line in format!("{}", method.task_network()).lines() {
        writeln!(f, "  {}", line)?;
    }
    Ok(())
}


pub fn render_task_network(f: &mut impl Write, network: &LiftedTaskNetwork) -> std::fmt::Result {
    writeln!(f, "TASKS: {}", network.tasks())?;
    writeln!(f, "ORDERING: {}", network.ordering_constraints())?;
    writeln!(f, "CONSTRAINTS: {}", network.logical_constraints())?;
    Ok(())
}

pub fn render_initial_task_network(f: &mut impl Write, network: &InitialTaskNetwork) -> std::fmt::Result {
    writeln!(f, "PARAMETERS: {}", network.parameters())?;
    let tn = network.task_network();
    writeln!(f, "TASKS: {}", tn.tasks())?;
    writeln!(f, "ORDERING: {}", tn.ordering_constraints())?;
    writeln!(f, "CONSTRAINTS: {}", tn.logical_constraints())?;
    Ok(())
}
