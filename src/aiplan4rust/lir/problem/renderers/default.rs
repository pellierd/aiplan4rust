use std::fmt;
use crate::aiplan4rust::lir::problem::{InitialTaskNetwork, LiftedAction, LiftedDurativeAction, LiftedMethod, LiftedProblem, LiftedTaskNetwork};
use crate::aiplan4rust::lir::problem::renderers::common::writeln_centered;
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};

/// Renders a `LiftedProblem` in a structured, human-readable format to a `Formatter`.
///
/// This function prints all parts of the problem in sections with centered titles
/// using `writeln_centered`. Each section may contain lists of items or multi-line
/// content, depending on the part of the problem being rendered.
///
/// # Sections Rendered
/// The output includes the following sections, in order:
/// 1. **Problem Header**: Displays the domain and problem names.
/// 2. **Requirements**: Lists any domain requirements (or indicates none).
/// 3. **Types**: Declared types in the problem.
/// 4. **Constants**: Problem constants.
/// 5. **Predicates**: Predicates defined in the domain.
/// 6. **Functions**: Functions defined in the domain.
/// 7. **Domain Constraints**: Global constraints of the domain.
/// 8. **Actions**: Each action with name, parameters, precondition, and effect.
/// 9. **Methods**: Each method with name, parameters, precondition, and task network.
/// 10. **Init**: The initial state of the problem.
/// 11. **Goal**: The goal specification.
/// 12. **Problem Constraints**: Constraints specific to the problem instance.
/// 13. **Metric Spec**: Metrics for plan evaluation.
/// 14. **Length Spec**: Optional length limit or plan length information.
/// 15. **Initial Task Network**: The initial hierarchical task network.
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the formatted output will be written.
/// - `problem`: The `LiftedProblem` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
///
/// let problem: LiftedProblem = /* create or obtain a problem instance */;
/// let mut output = String::new();
/// // Use a Formatter adapter if necessary
/// let _ = render_problem(&mut std::fmt::Formatter::new(&mut output), &problem);
/// println!("{}", output);
/// ```
pub fn render_problem(f: &mut fmt::Formatter<'_>, problem: &LiftedProblem) -> std::fmt::Result {
    // Titre principal
    writeln_centered(f, "PROBLEM", 80, '=')?;
    writeln!(f, "DOMAIN NAME  : {}", problem.domain_id())?;
    writeln!(f, "PROBLEM NAME : {}\n", problem.problem_id())?;

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
    if !problem.has_types() {
        writeln!(f, "  - no types")?;
    } else {
        for t in problem.types() {
            writeln!(f, "  - {}", t)?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if !problem.has_constants() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in problem.constants() {
            writeln!(f, "  - {}", c)?;
        }
    }
    writeln!(f)?;

    // Objects
    writeln_centered(f, "OBJECTS", 80, '=')?;
    if !problem.has_objects() {
        writeln!(f, "  - no objects")?;
    } else {
        for o in problem.objects() {
            writeln!(f, "  - {}", o)?;
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

/// Renders a human-readable representation of a `DomainDef` into a formatter.
///
/// This function pretty-prints the contents of a planning domain, including
/// its name, requirements, types, constants, predicates, functions, domain
/// constraints, actions, and methods. The output is intended for inspection,
/// debugging, or presentation purposes rather than for strict PDDL/HDDL export.
///
/// # Parameters
///
/// - `f`: A mutable reference to a [`fmt::Formatter`] used as the output target.
/// - `domain`: The [`DomainDef`] to be rendered.
///
/// # Returns
///
/// Returns [`fmt::Result`]. Any formatting or I/O error encountered while
/// writing to the formatter is propagated to the caller.
///
/// # Output Structure
///
/// The rendered output is organized into clearly delimited sections:
/// - **Domain header** (domain name)
/// - **Requirements**
/// - **Types**
/// - **Constants**
/// - **Predicates**
/// - **Functions**
/// - **Domain constraints**
/// - **Actions**
/// - **Methods**
///
/// Each section is preceded by a centered title for improved readability.
///
/// # Notes
///
/// - Empty sections (e.g., no types, predicates, actions, or methods) are
///   explicitly indicated in the output.
/// - Actions and methods are rendered using their dedicated helper functions
///   ([`render_action`] and [`render_method`]).
/// - This function writes directly to the provided formatter and does not
///   allocate intermediate strings.
///
/// # See Also
///
/// - [`render_action`]
/// - [`render_method`]
/// - [`DomainDef`]
pub fn render_domain_def(f: &mut fmt::Formatter<'_>, domain: &DomainDef) -> std::fmt::Result {
    writeln_centered(f, "DOMAIN DEF", 80, '=')?;
    writeln!(f, "DOMAIN NAME  : {}", domain.domain_name())?;

    // Requirements
    writeln_centered(f, "REQUIREMENTS", 80, '=')?;
    let mut reqs: Vec<_> = domain.requirements().iter().collect();
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
    if !domain.has_types() {
        writeln!(f, "  - no types")?;
    } else {
        for t in domain.types() {
            writeln!(f, "  - {}", t)?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if !domain.has_constants() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in domain.constants() {
            writeln!(f, "  - {}", c)?;
        }
    }
    writeln!(f)?;

    // Predicates
    writeln_centered(f, "PREDICATES", 80, '=')?;
    if domain.predicates().is_empty() {
        writeln!(f, "  - no predicates")?;
    } else {
        for p in domain.predicates() {
            writeln!(f, "  - {}", p)?;
        }
    }
    writeln!(f)?;

    // Functions
    writeln_centered(f, "FUNCTIONS", 80, '=')?;
    if domain.functions().is_empty() {
        writeln!(f, "  - no functions")?;
    } else {
        for fct in domain.functions() {
            writeln!(f, "  - {}", fct)?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = domain.domain_constraints();
    if dc.is_empty() {
        writeln!(f, "  - no domain constraints\n")?;
    } else {
        writeln!(f, "{}\n", dc)?;
    }

    // Actions
    if domain.actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in domain.actions() {
            render_action(f, action)?;
            writeln!(f)?;
        }
    }

    // Methods
    if domain.methods().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in domain.methods() {
            render_method(f, method)?;
            writeln!(f)?;
        }
    }

    Ok(())
}

/// Renders a human-readable representation of a `ProblemDef` into a formatter.
///
/// This function pretty-prints the contents of a planning problem, including
/// its domain name, problem name, requirements, objects, initial state (init),
/// goal, problem constraints, metric specification, length specification, and
/// the initial task network. The output is intended for inspection,
/// debugging, or presentation purposes rather than for direct PDDL/HDDL export.
///
/// # Parameters
///
/// - `f`: A mutable reference to a [`fmt::Formatter`] used as the output target.
/// - `problem`: The [`ProblemDef`] to be rendered.
///
/// # Returns
///
/// Returns [`std::fmt::Result`]. Any formatting or I/O error encountered while
/// writing to the formatter is propagated to the caller.
///
/// # Output Structure
///
/// The rendered output is organized into clearly delimited sections:
/// - **Problem header** (domain name and problem name)
/// - **Requirements**
/// - **Objects**
/// - **Init**
/// - **Goal**
/// - **Problem constraints**
/// - **Metric specification**
/// - **Length specification**
/// - **Initial Task Network**
///
/// Each section is preceded by a centered title for improved readability.
///
/// # Notes
///
/// - Empty sections (e.g., no requirements, objects, init, goal) are explicitly
///   indicated in the output.
/// - The initial task network is rendered using the helper function
///   [`render_initial_task_network`].
/// - This function writes directly to the provided formatter and does not
///   allocate intermediate strings.
///
/// # See Also
///
/// - [`render_initial_task_network`]
/// - [`ProblemDef`]
pub fn render_problem_def(f: &mut fmt::Formatter<'_>, problem: &ProblemDef) -> std::fmt::Result {
    writeln_centered(f, "PROBLEM DEF", 80, '=')?;
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

    // Objects
    writeln_centered(f, "OBJECTS", 80, '=')?;
    if !problem.has_objects() {
        writeln!(f, "  - no objects")?;
    } else {
        for o in problem.objects() {
            writeln!(f, "  - {}", o)?;
        }
    }
    writeln!(f)?;

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

/// Renders a `LiftedAction` in a structured, human-readable format to a `Formatter`.
///
/// This function prints the action in a clear, indented style with a centered
/// section title. The output includes:
/// - **Name**: The action's name.
/// - **Parameters**: A comma-separated list of parameters.
/// - **Precondition**: The precondition expression, printed line by line.
/// - **Effect**: The effect expression, printed line by line.
///
/// The section title is centered using `writeln_centered` with a fixed width
/// (80 characters) and a custom fill character (`'-'`).
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the output will be written.
/// - `action`: The `LiftedAction` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedAction;
///
/// let action: LiftedAction = /* create or obtain an action */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_action(&mut std::fmt::Formatter::new(&mut output), &action);
/// println!("{}", output);
/// ```
pub fn render_action(f: &mut fmt::Formatter<'_>, action: &LiftedAction) -> std::fmt::Result {
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

/// Renders a `LiftedDurativeAction` in a structured, human-readable format to a `Formatter`.
///
/// This function prints the durative action in a clear, indented style with a
/// centered section title. The output includes:
/// - **Name**: The action's name.
/// - **Parameters**: A comma-separated list of parameters.
/// - **Duration**: The duration expression, printed line by line.
/// - **Conditions**: The timed conditions of the action, printed line by line.
/// - **Effect**: The effect expression, printed line by line.
///
/// The section title is centered using `writeln_centered` with a fixed width
/// (80 characters) and a custom fill character (`'-'`).
///
/// # Parameters
///
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the output will be written.
/// - `action`: The `LiftedDurativeAction` instance to render.
///
/// # Returns
///
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedDurativeAction;
///
/// let action: LiftedDurativeAction = /* create or obtain a durative action */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_durative_action(&mut std::fmt::Formatter::new(&mut output), &action);
/// println!("{}", output);
/// ```
pub fn render_durative_action(f: &mut fmt::Formatter<'_>, action: &LiftedDurativeAction) -> std::fmt::Result {
    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "DURATIVE ACTION", 80, '-')?;
    writeln!(f, "NAME: {}", action.name())?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "DURATION:")?;
    for line in format!("{}", action.duration()).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "CONDITIONS:")?;
    for line in format!("{}", action.condition()).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "EFFECT:")?;
    for line in format!("{}", action.effect()).lines() {
        writeln!(f, "  {}", line)?;
    }
    Ok(())
}

/// Renders a `LiftedMethod` in a structured, human-readable format to a `Formatter`.
///
/// This function prints the method with a centered section title and a clear, indented layout.
/// The output includes:
/// - **Name**: The method's name.
/// - **Parameters**: A comma-separated list of parameters.
/// - **Task**: The associated task, printed on a separate line.
/// - **Precondition**: The method's precondition, printed line by line.
/// - **Task Network**: The hierarchical task network of the method, rendered via `render_task_network`.
///
/// The section title is centered using `writeln_centered` with a fixed width (80 characters)
/// and a custom fill character (`'-'`).
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the formatted output will be written.
/// - `method`: The `LiftedMethod` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedMethod;
///
/// let method: LiftedMethod = /* obtain or create a method */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_method(&mut std::fmt::Formatter::new(&mut output), &method);
/// println!("{}", output);
/// ```
pub fn render_method(f: &mut fmt::Formatter<'_>, method: &LiftedMethod) -> std::fmt::Result {
    let params = method
        .parameters()
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "METHOD", 80, '-')?;
    writeln!(f, "NAME: {}", method.name())?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "TASK:\n  {}", method.task())?;
    writeln!(f, "PRECONDITION:")?;
    for line in format!("{}", method.precondition()).lines() {
        writeln!(f, "  {}", line)?;
    }
    render_task_network(f, &method.task_network())?;
    Ok(())
}


/// Renders a `LiftedTaskNetwork` in a human-readable, structured format to a `Formatter`.
///
/// This function prints the task network in three sections:
/// - **Tasks**: Lists all tasks in the network.
/// - **Ordering**: Shows the ordering constraints between tasks.
/// - **Constraints**: Displays logical constraints associated with the task network.
///
/// Each section is printed on its own line, with content indented for readability.
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the output will be written.
/// - `network`: The `LiftedTaskNetwork` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedTaskNetwork;
///
/// let network: LiftedTaskNetwork = /* obtain or create a task network */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_task_network(&mut std::fmt::Formatter::new(&mut output), &network);
/// println!("{}", output);
/// ```
pub fn render_task_network(f: &mut fmt::Formatter<'_>, network: &LiftedTaskNetwork) -> std::fmt::Result {
    writeln!(f, "TASKS:\n  {}", network.tasks())?;
    writeln!(f, "ORDERING:\n  {}", network.ordering_constraints())?;
    writeln!(f, "CONSTRAINTS:\n  {}", network.logical_constraints())?;
    Ok(())
}

/// Renders an `InitialTaskNetwork` in a human-readable, structured format to a `Formatter`.
///
/// This function prints the initial task network in four sections:
/// - **Parameters**: Lists all parameters of the initial task network.
/// - **Tasks**: Lists all tasks in the network.
/// - **Ordering**: Shows the ordering constraints between tasks.
/// - **Constraints**: Displays logical constraints associated with the task network.
///
/// Each section is printed on its own line, with content indented for readability.
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the output will be written.
/// - `network`: The `InitialTaskNetwork` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::InitialTaskNetwork;
///
/// let initial_network: InitialTaskNetwork = /* obtain or create initial task network */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_initial_task_network(&mut std::fmt::Formatter::new(&mut output), &initial_network);
/// println!("{}", output);
/// ```
pub fn render_initial_task_network(f: &mut fmt::Formatter<'_>, network: &InitialTaskNetwork) -> std::fmt::Result {
    writeln!(f, "PARAMETERS: {}", network.parameters())?;
    let tn = network.task_network();
    writeln!(f, "TASKS:\n  {}", tn.tasks())?;
    writeln!(f, "ORDERING:\n  {}", tn.ordering_constraints())?;
    writeln!(f, "CONSTRAINTS:\n  {}", tn.logical_constraints())?;
    Ok(())
}
