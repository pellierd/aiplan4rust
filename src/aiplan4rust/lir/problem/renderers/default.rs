use crate::aiplan4rust::lir::problem::{
    InitialTaskNetwork, LiftedAction, LiftedMethod, LiftedProblem, LiftedTaskNetwork,
};

/// Renders a `LiftedProblem` as a human-readable string.
///
/// This function formats the problem for display, showing:
/// - Domain and problem names
/// - Requirements, types, constants, predicates, functions
/// - Domain constraints
/// - Actions and methods (using their `Display` implementations)
/// - Initial state and goal
/// - Problem constraints, metric and length specifications
/// - Initial task network
///
/// # Example
///
/// ```rust
/// use crate::renderers::default::render_problem;
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
///
/// let problem: LiftedProblem = /* create or get a problem */;
/// println!("{}", render_problem(&problem));
/// ```
pub fn render_problem(problem: &LiftedProblem) -> String {
    let mut s = String::new();

    // Titre principal
    s.push_str("############################ PROBLEM #############################\n");
    s.push_str(&format!("  DOMAIN NAME  : {}\n", problem.domain_name()));
    s.push_str(&format!("  PROBLEM NAME : {}\n\n", problem.problem_name()));

    // Requirements
    s.push_str("=========================== REQUIREMENTS =========================\n");
    let mut reqs: Vec<_> = problem.requirements().iter().collect();
    if reqs.is_empty() {
        s.push_str("  - no requirements\n");
    } else {
        reqs.sort();
        for r in reqs {
            s.push_str(&format!("  - {}\n", r));
        }
    }
    s.push_str("\n");

    // Types
    s.push_str("============================== TYPES =============================\n");
    if problem.types().is_empty() {
        s.push_str("  - no types\n");
    } else {
        for t in problem.types() {
            s.push_str(&format!("  - {}\n", t));
        }
    }
    s.push_str("\n");

    // Constants
    s.push_str("============================ CONSTANTS ===========================\n");
    if problem.constants().is_empty() {
        s.push_str("  - no constants\n");
    } else {
        for c in problem.constants() {
            s.push_str(&format!("  - {}\n", c));
        }
    }
    s.push_str("\n");

    // Predicates
    s.push_str("=========================== PREDICATES ===========================\n");
    if problem.predicates().is_empty() {
        s.push_str("  - no predicates\n");
    } else {
        for p in problem.predicates() {
            s.push_str(&format!("  - {}\n", p));
        }
    }
    s.push_str("\n");

    // Functions
    s.push_str("=========================== FUNCTIONS ============================\n");
    if problem.functions().is_empty() {
        s.push_str("  - no functions\n");
    } else {
        for fct in problem.functions() {
            s.push_str(&format!("  - {}\n", fct));
        }
    }
    s.push_str("\n");

    // Domain constraints
    s.push_str("======================= DOMAIN CONSTRAINTS =======================\n");
    let dc = problem.domain_constraints();
    if dc.is_empty() {
        s.push_str("  - no domain constraints\n\n");
    } else {
        s.push_str(&format!("{}\n\n", dc));
    }

    // Actions
    if problem.actions().is_empty() {
        s.push_str("  - no actions\n\n");
    } else {
        for action in problem.actions() {
            s.push_str(&render_action(action));
            s.push_str("\n");
        }
    }

    // Methods
    if problem.methods().is_empty() {
        s.push_str("  - no methods\n\n");
    } else {
        for method in problem.methods() {
            s.push_str(&render_method(method));
            s.push_str("\n");
        }
    }

    // Init
    s.push_str("============================== INIT ==============================\n");
    let init = problem.init();
    if init.is_empty() {
        s.push_str("  - no init\n\n");
    } else {
        s.push_str(&format!("{}\n\n", init));
    }

    // Goal
    s.push_str("============================== GOAL ==============================\n");
    let goal = problem.goal();
    if goal.is_empty() {
        s.push_str("  - no goal\n\n");
    } else {
        s.push_str(&format!("{}\n\n", goal));
    }

    // Problem constraints
    s.push_str("======================= PROBLEM CONSTRAINTS ======================\n");
    let pc = problem.problem_constraints();
    if pc.is_empty() {
        s.push_str("  - no problem constraints\n\n");
    } else {
        s.push_str(&format!("{}\n\n", pc));
    }

    // Metric spec
    s.push_str("=========================== METRIC SPEC ==========================\n");
    let metric = problem.metric_spec();
    if metric.is_empty() {
        s.push_str("  - no metric spec\n\n");
    } else {
        s.push_str(&format!("{}\n\n", metric));
    }

    // Length spec
    s.push_str("=========================== LENGTH SPEC ==========================\n");
    let length = problem.length_spec();
    if length.is_empty() {
        s.push_str("  - no length spec\n\n");
    } else {
        s.push_str(&format!("{}\n\n", length));
    }

    // Initial Task Network
    s.push_str("===================== INITIAL TASK NETWORK ======================\n");
    s.push_str(&format!("{}\n", problem.initial_task_network()));

    s
}

/// Renders an `Action` as a human-readable string.
///
/// This function formats the action by displaying:
/// - The name of the action
/// - Its parameters
/// - Its precondition
/// - Its effect
///
/// # Example
///
/// ```rust
/// use crate::renderers::default::render_action;
/// use crate::aiplan4rust::lir::problem::action::Action;
///
/// let action: Action = /* create or get an action */;
/// println!("{}", render_action(&action));
/// ```
pub fn render_action(action: &LiftedAction) -> String {
    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut s = String::new();
    s.push_str("============================= ACTION =============================\n");
    s.push_str(&format!("  NAME: {}\n", action.name()));
    s.push_str(&format!("  PARAMETERS: {}\n", params));
    s.push_str("  PRECONDITION:\n");
    for line in format!("{}", action.precondition()).lines() {
        s.push_str(&format!("    {}\n", line));
    }
    s.push_str("  EFFECT:\n");
    for line in format!("{}", action.effect()).lines() {
        s.push_str(&format!("    {}\n", line));
    }

    s
}

/// Renders a `Method` as a human-readable string.
///
/// This function formats the method by displaying:
/// - Its name
/// - Its parameters
/// - The associated task
/// - Its precondition
/// - Its task network
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::lir::problem::renderers::default::render_method;
/// use crate::aiplan4rust::lir::problem::method::Method;
///
/// let method: Method = /* create or get a method */;
/// println!("{}", render_method(&method));
/// ```
pub fn render_method(method: &LiftedMethod) -> String {
    let params = method
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut s = String::new();
    s.push_str("============================= METHOD =============================\n");
    s.push_str(&format!("  NAME: {}\n", method.name()));
    s.push_str(&format!("  PARAMETERS: {}\n", params));
    s.push_str(&format!("  TASK: {}\n", method.task()));

    // PRECONDITION
    s.push_str("  PRECONDITION:\n");
    for line in format!("{}", method.precondition()).lines() {
        s.push_str(&format!("    {}\n", line));
    }

    // TASK NETWORK
    s.push_str("  TASK NETWORK:\n");
    for line in format!("{}", method.task_network()).lines() {
        s.push_str(&format!("    {}\n", line));
    }

    s
}


/// Renders a `TaskNetwork` as a human-readable string.
///
/// This function formats the task network by displaying:
/// - The list of tasks
/// - Ordering constraints
/// - Logical constraints
///
/// # Example
///
/// ```rust
/// use crate::renderers::default::render_task_network;
/// use crate::aiplan4rust::lir::problem::task_network::TaskNetwork;
///
/// let tn: TaskNetwork = /* create or get a TaskNetwork */;
/// println!("{}", render_task_network(&tn));
/// ```
pub fn render_task_network(network: &LiftedTaskNetwork) -> String {
    format!(
        "  TASKS: {}\n  ORDERING: {}\n  CONSTRAINTS: {}",
        network.tasks(),
        network.ordering_constraints(),
        network.logical_constraints()
    )
}

/// Renders an `InitialTaskNetwork` as a human-readable string.
///
/// This function formats the initial task network by displaying:
/// - Its parameters
/// - Its task network
///
/// # Example
///
/// ```rust
/// use crate::aiplan4rust::lir::problem::renderers::default::render_initial_task_network;
/// use crate::aiplan4rust::lir::problem::task_network::InitialTaskNetwork;
///
/// let init_network: InitialTaskNetwork = /* create or get the initial task network */;
/// println!("{}", render_initial_task_network(&init_network));
/// ```
pub fn render_initial_task_network(network: &InitialTaskNetwork) -> String {
    format!(
        "  PARAMETERS: {}\n{}",
        network.parameters(),
        network.task_network()
    )
}
