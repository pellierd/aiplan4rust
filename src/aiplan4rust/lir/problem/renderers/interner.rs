use std::fmt;
use crate::aiplan4rust::lir::problem::{InitialTaskNetwork, LiftedAction, LiftedDerivedPredicate, LiftedDurativeAction, LiftedMethod, LiftedProblem, LiftedTaskNetwork};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::problem::renderers::common::writeln_centered;
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};

/// Renders a `LiftedProblem` in a human-readable, nicely formatted way using a `Formatter`.
///
/// This function prints all parts of the problem with centered section titles and optional
/// content lists. The formatting uses a fixed line width (e.g., 80 characters) and a custom
/// fill character for section separators.
///
/// # Sections Rendered
/// - **Problem Header**: The domain and problem names.
/// - **Requirements**: Lists any domain requirements.
/// - **Types**: All declared types in the problem.
/// - **Constants**: Problem constants.
/// - **Predicates**: Predicates defined in the domain.
/// - **Functions**: Functions defined in the domain.
/// - **Domain Constraints**: Global constraints of the domain.
/// - **Actions**: Each action with name, parameters, precondition, and effect.
/// - **Methods**: Each method with name, parameters, precondition, and task network.
/// - **Init**: Initial state.
/// - **Goal**: Goal specification.
/// - **Problem Constraints**: Constraints specific to this problem.
/// - **Metric Spec**: Metric for plan evaluation.
/// - **Length Spec**: Optional length limit or plan length information.
/// - **Initial Task Network**: The initial hierarchical task network.
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `problem`: The `LiftedProblem` instance to render.
///
/// # Returns
/// Returns `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedProblem;
///
/// let problem: LiftedProblem = /* obtain or create problem */;
/// let mut output = String::new();
/// render_problem(&mut output, &problem)?;
/// println!("{}", output);
/// ```
pub fn render_problem(
    f: &mut fmt::Formatter<'_>,
    problem: &LiftedProblem,
    interner: &StringInterner,
) -> std::fmt::Result {
    // Titre principal
    writeln_centered(f, "PROBLEM", 80, '=')?;
    writeln!(
        f,
        "DOMAIN NAME  : {}",
        interner.resolve_ident(problem.domain_id()).unwrap_or("<unknown>")
    )?;
    writeln!(
        f,
        "PROBLEM NAME : {}\n",
        interner.resolve_ident(problem.problem_id()).unwrap_or("<unknown>")
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
    if !problem.has_types() {
        writeln!(f, "  - no types")?;
    } else {
        for t in problem.types() {
            writeln!(f, "  - {}", t.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if !problem.has_constants() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in problem.constants() {
            writeln!(f, "  - {}", c.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Objects
    writeln_centered(f, "OBJECTS", 80, '=')?;
    if !problem.has_objects() {
        writeln!(f, "  - no objects")?;
    } else {
        for o in problem.objects() {
            writeln!(f, "  - {}", o.to_string_with_interner(interner))?;
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

    writeln_centered(f, "PREDICATE BINDINGS (NodeId -> Index)", 80, '-')?;
    if problem.predicate_bindings().is_empty() {
        writeln!(f, "  - no predicate bindings")?;
    } else {
        // On trie par NodeId pour la lisibilité
        let mut bindings: Vec<_> = problem.predicate_bindings().iter().collect();
        bindings.sort_by_key(|(&id, _)| id);
        for (node_id, &pred_idx) in bindings {
            let name = problem.predicates().get(pred_idx)
                .map(|p| p.to_string_with_interner(interner))
                .unwrap_or_else(|| "UNKNOWN".to_string());
            writeln!(f, "  Node #{} => [{}] {}", node_id, pred_idx, name)?;
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

    writeln_centered(f, "FUNCTION BINDINGS (NodeId -> Index)", 80, '-')?;
    if problem.function_bindings().is_empty() {
        writeln!(f, "  - no function bindings")?;
    } else {
        let mut bindings: Vec<_> = problem.function_bindings().iter().collect();
        bindings.sort_by_key(|(&id, _)| id);
        for (node_id, &func_idx) in bindings {
            let name = problem.functions().get(func_idx)
                .map(|f| f.to_string_with_interner(interner))
                .unwrap_or_else(|| "UNKNOWN".to_string());
            writeln!(f, "  Node #{} => [{}] {}", node_id, func_idx, name)?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = problem.domain_constraints();
    writeln!(f, "{}\n", dc.to_string_with_interner(interner))?;

    // Derived predicates
    if problem.derived_predicates().is_empty() {
        writeln!(f, "  - no derived predicates\n")?;
    } else {
        for derived_predicate in problem.derived_predicates() {
            render_derived_predicate(f, derived_predicate, interner)?;
            writeln!(f)?;
        }
    }

    // Actions
    if problem.actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in problem.actions() {
            render_action(f, action, interner)?;
            writeln!(f)?;
        }
    }

    // Durative Actions
    if problem.durative_actions().is_empty() {
        writeln!(f, "  - no durative actions\n")?;
    } else {
        for action in problem.durative_actions() {
            render_durative_action(f, action, interner)?;
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

/// Renders a human-readable representation of a `DomainDef` into a formatter.
///
/// This function formats and writes the full contents of a planning domain,
/// including its metadata, requirements, types, constants, predicates,
/// functions, domain constraints, actions, and methods. The output is intended
/// for inspection, debugging, or pretty-printing rather than for direct
/// consumption by a planner.
///
/// # Parameters
///
/// - `f`: A mutable reference to a [`fmt::Formatter`] used as the output target.
/// - `problem`: The [`DomainDef`] to be rendered.
/// - `interner`: A [`StringInterner`] used to resolve identifiers into readable strings.
///
/// # Returns
///
/// Returns [`fmt::Result`]. An error is propagated if any write operation to
/// the formatter fails.
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
/// - Identifiers that cannot be resolved by the interner are rendered as
///   `"<unknown>"`.
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
pub fn render_domain_def(
    f: &mut fmt::Formatter<'_>,
    domain: &DomainDef,
    interner: &StringInterner,
) -> fmt::Result {
    writeln_centered(f, "DOMAIN DEF", 80, '=')?;
    writeln!(
        f,
        "DOMAIN NAME  : {}",
        interner.resolve_ident(domain.domain_name()).unwrap_or("<unknown>")
    )?;

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
            writeln!(f, "  - {}", t.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if !domain.has_constants() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in domain.constants() {
            writeln!(f, "  - {}", c.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Predicates
    writeln_centered(f, "PREDICATES", 80, '=')?;
    if domain.predicates().is_empty() {
        writeln!(f, "  - no predicates")?;
    } else {
        for p in domain.predicates() {
            writeln!(f, "  - {}", p.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Functions
    writeln_centered(f, "FUNCTIONS", 80, '=')?;
    if domain.functions().is_empty() {
        writeln!(f, "  - no functions")?;
    } else {
        for func in domain.functions() {
            writeln!(f, "  - {}", func.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = domain.domain_constraints();
    writeln!(f, "{}\n", dc.to_string_with_interner(interner))?;

    // Derived predicates
    if domain.derived_predicates().is_empty() {
        writeln!(f, "  - no derived predicates\n")?;
    } else {
        for derived_predicate in domain.derived_predicates() {
            render_derived_predicate(f, derived_predicate, interner)?;
            writeln!(f)?;
        }
    }

    // Actions
    if domain.actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in domain.actions() {
            render_action(f, action, interner)?;
            writeln!(f)?;
        }
    }

    // Durative Actions
    if domain.durative_actions().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in domain.durative_actions() {
            render_durative_action(f, action, interner)?;
            writeln!(f)?;
        }
    }

    // Methods
    if domain.methods().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in domain.methods() {
            render_method(f, method, interner)?;
            writeln!(f)?;
        }
    }

    Ok(())
}

/// Renders a human-readable representation of a `ProblemDef` into a formatter.
///
/// This function formats and writes the full contents of a planning problem,
/// including its metadata, requirements, constants, objects, initial state,
/// goal, constraints, metrics, length specification, and initial task network.
/// The output is intended for inspection, debugging, or pretty-printing
/// rather than for direct consumption by a planner.
///
/// # Parameters
///
/// - `f`: A mutable reference to a [`fmt::Formatter`] used as the output target.
/// - `problem`: The [`ProblemDef`] to be rendered.
/// - `interner`: A [`StringInterner`] used to resolve identifiers into readable strings.
///
/// # Returns
///
/// Returns [`fmt::Result`]. An error is propagated if any write operation to
/// the formatter fails.
///
/// # Output Structure
///
/// The rendered output is organized into clearly delimited sections:
/// - **Problem header** (domain name and problem name)
/// - **Requirements**
/// - **Constants**
/// - **Objects**
/// - **Init**
/// - **Goal**
/// - **Problem constraints**
/// - **Metric specification**
/// - **Length specification**
/// - **Initial task network**
///
/// Each section is preceded by a centered title for improved readability.
///
/// # Notes
///
/// - Identifiers that cannot be resolved by the interner are rendered as
///   `"<unknown>"`.
/// - Empty sections (e.g., no requirements, constants, or objects) are
///   explicitly indicated in the output.
/// - This function does not allocate intermediate strings; it writes directly
///   to the provided formatter.
///
/// # See Also
///
/// - [`render_initial_task_network`]
/// - [`ProblemDef`]
pub fn render_problem_def(
    f: &mut fmt::Formatter<'_>,
    problem: &ProblemDef,
    interner: &StringInterner,
) -> fmt::Result {
    writeln_centered(f, "PROBLEM DEF", 80, '=')?;
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

    // Objects
    writeln_centered(f, "OBJECTS", 80, '=')?;
    if !problem.has_objects() {
        writeln!(f, "  - no objects")?;
    } else {
        for o in problem.objects() {
            writeln!(f, "  - {}", o.to_string_with_interner(interner))?;
        }
    }
    writeln!(f)?;

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

/// Renders a `LiftedAction` in a human-readable format to any type implementing `Write`.
///
/// This function formats the action with a centered section title, then prints:
/// - **Name**: The action's name, resolved through the given `StringInterner`.
/// - **Parameters**: All parameters of the action, formatted using the interner.
/// - **Precondition**: The action's precondition, line by line.
/// - **Effect**: The action's effect, line by line.
///
/// The section title is centered using `writeln_centered` with a fixed width (e.g., 80)
/// and a custom fill character (`'-'` in this case).
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `action`: The `LiftedAction` to render.
/// - `interner`: A `StringInterner` used to resolve identifiers and format parameters.
///
/// # Returns
/// Returns `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{LiftedAction, StringInterner};
///
/// let action: LiftedAction = /* obtain or create action */;
/// let interner: StringInterner = /* obtain interner */;
/// let mut output = String::new();
/// render_action(&mut output, &action, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_action(
    f: &mut fmt::Formatter<'_>,
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

/// Renders a `LiftedDurativeAction` in a human-readable format to any type implementing `Write`.
///
/// This function formats the durative action with a centered section title, then prints:
/// - **Name**: The action's name, resolved through the given `StringInterner`.
/// - **Parameters**: All parameters of the action, formatted using the interner.
/// - **Duration**: The duration expression of the action, printed line by line.
/// - **Conditions**: The timed conditions of the action, printed line by line.
/// - **Effect**: The action's effect, printed line by line.
///
/// The section title is centered using `writeln_centered` with a fixed width (e.g., 80)
/// and a custom fill character (`'-'` in this case).
///
/// # Parameters
///
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `action`: The `LiftedDurativeAction` to render.
/// - `interner`: A `StringInterner` used to resolve identifiers and format parameters.
///
/// # Returns
///
/// Returns `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{LiftedDurativeAction, StringInterner};
///
/// let action: LiftedDurativeAction = /* obtain or create durative action */;
/// let interner: StringInterner = /* obtain interner */;
/// let mut output = String::new();
/// render_durative_action(&mut output, &action, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_durative_action(
    f: &mut fmt::Formatter<'_>,
    action: &LiftedDurativeAction,
    interner: &StringInterner,
) -> std::fmt::Result {
    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(", ");

    writeln_centered(f, "DURATIVE ACTION", 80, '-')?;
    writeln!(
        f,
        "NAME: {}",
        interner.resolve_ident(action.name()).unwrap_or("<unknown>")
    )?;
    writeln!(f, "PARAMETERS: {}", params)?;
    writeln!(f, "DURATION:")?;
    for line in action.duration().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "CONDITIONS:")?;
    for line in action.condition().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }
    writeln!(f, "EFFECT:")?;
    for line in action.effect().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }

    Ok(())
}

/// Renders a `LiftedMethod` in a human-readable format to any writer implementing `Write`.
///
/// This function prints the method in a structured way with a centered section title. It includes:
/// - **Name**: The method's name, resolved via the given `StringInterner`.
/// - **Parameters**: All parameters of the method, formatted using the interner.
/// - **Task**: The associated task, line by line, using the interner for identifiers.
/// - **Precondition**: The method's precondition, line by line. If empty, `<empty>` is displayed.
/// - **Task Network**: Renders the method's task network using `render_task_network`.
///
/// The section title is centered using `writeln_centered` with a fixed width (80 characters)
/// and a custom fill character (`'-'`).
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `method`: The `LiftedMethod` to render.
/// - `interner`: A `StringInterner` for resolving identifiers and formatting parameters.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{LiftedMethod, StringInterner};
///
/// let method: LiftedMethod = /* obtain or create method */;
/// let interner: StringInterner = /* obtain interner */;
/// let mut output = String::new();
/// render_method(&mut output, &method, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_method(
    f: &mut fmt::Formatter<'_>,
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
        "TASK:\n  {}",
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

    render_task_network(f, &method.task_network(), interner)?;

    Ok(())
}

/// Renders a `LiftedTaskNetwork` in a human-readable format to any writer implementing `Write`.
///
/// This function prints the task network in a structured way, including:
/// - **Tasks**: Lists all tasks, using the provided `StringInterner` to resolve identifiers.
/// - **Ordering constraints**: Shows the ordering constraints between tasks.
/// - **Logical constraints**: Displays any logical constraints associated with the task network.
///
/// Each section is printed on its own line, with the content indented for readability.
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `network`: The `LiftedTaskNetwork` to render.
/// - `interner`: A `StringInterner` for resolving identifiers.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{LiftedTaskNetwork, StringInterner};
///
/// let network: LiftedTaskNetwork = /* obtain or create task network */;
/// let interner: StringInterner = /* obtain interner */;
/// let mut output = String::new();
/// render_task_network(&mut output, &network, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_task_network(
    f: &mut fmt::Formatter<'_>,
    network: &LiftedTaskNetwork,
    interner: &StringInterner,
) -> std::fmt::Result {
    writeln!(f, "TASKS:\n  {}", network.tasks().to_string_with_interner(interner))?;
    writeln!(f, "ORDERING:\n  {}", network.ordering_constraints().to_string_with_interner(interner))?;
    writeln!(f, "CONSTRAINTS:\n  {}", network.logical_constraints().to_string_with_interner(interner))?;
    Ok(())
}

/// Renders an `InitialTaskNetwork` in a human-readable format to any writer implementing `Write`.
///
/// This function prints the initial task network in a structured format, including:
/// - **Parameters**: Lists the parameters of the initial task network.
/// - **Tasks**: Lists all tasks, using the provided `StringInterner` to resolve identifiers.
/// - **Ordering constraints**: Shows the ordering constraints between tasks.
/// - **Logical constraints**: Displays any logical constraints associated with the task network.
///
/// Each section is printed on its own line, with the content indented for readability.
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output is written.
/// - `network`: The `InitialTaskNetwork` to render.
/// - `interner`: A `StringInterner` for resolving identifiers.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{InitialTaskNetwork, StringInterner};
///
/// let initial_network: InitialTaskNetwork = /* obtain or create initial task network */;
/// let interner: StringInterner = /* obtain interner */;
/// let mut output = String::new();
/// render_initial_task_network(&mut output, &initial_network, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_initial_task_network(
    f: &mut fmt::Formatter<'_>,
    network: &InitialTaskNetwork,
    interner: &StringInterner,
) -> std::fmt::Result {
    writeln!(f, "PARAMETERS: {}", network.parameters().to_string_with_interner(interner))?;
    let tw = network.task_network();
    writeln!(f, "TASKS:\n  {}", tw.tasks().to_string_with_interner(interner))?;
    writeln!(f, "ORDERING:\n  {}", tw.ordering_constraints().to_string_with_interner(interner))?;
    writeln!(f, "CONSTRAINTS:\n  {}", tw.logical_constraints().to_string_with_interner(interner))?;
    Ok(())
}

/// Renders a `LiftedDerivedPredicate` in a human-readable format to a `Formatter`,
/// resolving interned identifiers using a `StringInterner`.
///
/// This function prints the derived predicate in two main sections:
/// - **HEAD**: The atomic formula skeleton representing the predicate's name and parameters,
///   resolved via the provided `StringInterner`.
/// - **BODY**: The logical expression defining the derived predicate, printed line by line,
///   with identifiers resolved via the interner.
///
/// Each section is indented for readability, and a centered section title is printed at the top.
///
/// # Parameters
/// - `f`: A mutable reference to a `Formatter` where the formatted output will be written.
/// - `derived_predicate`: The `LiftedDerivedPredicate` to render.
/// - `interner`: A `StringInterner` used to resolve interned identifiers for the predicate's head and body.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::{LiftedDerivedPredicate, StringInterner};
///
/// let derived: LiftedDerivedPredicate = /* create or obtain a derived predicate */;
/// let interner: StringInterner = /* create or obtain an interner */;
/// let mut output = String::new();
/// render_derived_predicate(&mut std::fmt::Formatter::new(&mut output), &derived, &interner)?;
/// println!("{}", output);
/// ```
pub fn render_derived_predicate(
    f: &mut fmt::Formatter<'_>,
    derived_predicate: &LiftedDerivedPredicate,
    interner: &StringInterner,
) -> std::fmt::Result {
    writeln_centered(f, "DERIVED PREDICATE", 80, '-')?;
    writeln!(
        f,
        "HEAD:\n  {}",
        derived_predicate.head().to_string_with_interner(interner)
    )?;

    writeln!(f, "BODY:")?;
    for line in derived_predicate.body().to_string_with_interner(interner).lines() {
        writeln!(f, "  {}", line)?;
    }

    Ok(())
}
