use std::fmt;
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::{Expr, ExprNode};
use crate::aiplan4rust::lir::{InitialTaskNetwork, LiftedAction, LiftedDerivedPredicate, LiftedMethod, LiftedTaskNetwork};
use crate::aiplan4rust::lir::renderers::common::{render_labeled_expr, render_labeled_typed_list, writeln_centered};
use crate::aiplan4rust::lir::problem::{DomainDef, LiftedProblem, ProblemDef};

/// Structure privée pour faire le pont avec le système de formatage de Rust
struct ProblemWrapper<'a>(&'a LiftedProblem);

impl<'a> fmt::Display for ProblemWrapper<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // C'est ici que le formatter 'f' nous est donné par Rust !
        // On appelle ta fonction existante en lui passant 'f'
        render_problem(f, self.0)
    }
}

/// Retourne la représentation textuelle complète du problème
pub fn to_string(problem: &LiftedProblem) -> String {
    // L'appel à .to_string() utilise l'implémentation fmt::Display ci-dessus
    ProblemWrapper(problem).to_string()
}

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
    writeln_centered(f, " [ PROBLEM ] ", 80, '=')?;
    writeln!(f, "  {:<12} : {}", "DOMAIN NAME", problem.domain_name())?;
    writeln!(f, "  {:<12} : {}", "PROBLEM NAME", problem.problem_name())?;
    writeln!(f, "{:-<80}\n", "")?;

    // Requirements
    if !problem.requirements().is_empty() {
        writeln_centered(f, " [ REQUIREMENTS ] ", 80, '=')?;
        writeln!(f, "  ID    : FEATURE")?;
        writeln!(f, "{:-<80}", "")?;

        let mut reqs: Vec<_> = problem.requirements().iter().collect();
        reqs.sort();

        for (i, r) in reqs.iter().enumerate() {
            let id_str = format!("#{}", i);
            // On garde le {:<5} pour l'ID et le ":" pour l'alignement vertical global
            writeln!(f, "  {:<5} : {}", id_str, r)?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Types
    if !problem.type_defs().is_empty() {
        writeln_centered(f, " [ TYPES ] ", 80, '=')?;
        writeln!(f, "  ID    : NAME            - PARENT")?;
        writeln!(f, "{:-<80}", "")?;
        for (i, t) in problem.type_defs().iter().enumerate() {
            let id_str = format!("#{}", i);
            let name = format!("{}", t.symbol()).trim().to_string();
            let parent = format!("{}", t.ty()).trim().to_string();
            writeln!(f, "  {:<5} : {:<15} - {}", id_str, name, parent)?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Constants
    if !problem.domain_constant_def().is_empty() {
        writeln_centered(f, " [ CONSTANTS ] ", 80, '=')?;
        writeln!(f, "  ID    : NAME            - TYPE")?;
        writeln!(f, "{:-<80}", "")?;
        for (i, c) in problem.domain_constant_def().iter().enumerate() {
            let id_str = format!("#{}", i);
            let name = format!("{}", c.symbol()).trim().to_string();
            let c_type = format!("{}", c.ty()).trim().to_string();
            writeln!(f, "  {:<5} : {:<15} - {}", id_str, name, c_type)?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Objects
    if !problem.problem_object_def().is_empty() {
        writeln_centered(f, " [ OBJECTS ] ", 80, '=')?;
        // Alignement identique : ID (5), NAME (15) et TYPE
        writeln!(f, "  ID    : NAME            - TYPE")?;
        writeln!(f, "{:-<80}", "")?;
        for (i, o) in problem.problem_object_def().iter().enumerate() {
            let id_str = format!("#{}", i);
            let name = format!("{}", o.symbol()).trim().to_string();
            let type_name = format!("{}", o.ty()).trim().to_string();
            writeln!(f, "  {:<5} : {:<15} - {}", id_str, name, type_name)?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Predicates
    if !problem.predicate_defs().is_empty() {
        writeln_centered(f, " [ PREDICATES ] ", 80, '=')?;
        // Colonnes : ID, Nom du prédicat, et Paramètres
        writeln!(f, "  {:<5} : {:<15} {}", "ID", "NAME", "PARAMETERS")?;
        writeln!(f, "{:-<80}", "")?;

        for (i, p) in problem.predicate_defs().iter().enumerate() {
            let id_str = format!("#{}", i);
            let name = format!("{}", p.symbol()).trim().to_string();

            // On récupère les paramètres et on les formate (ex: "?x ?y")
            let params: Vec<String> = p.parameters()
                .iter()
                .map(|param| format!("{}", param))
                .collect();
            let params_str = format!("({})", params.join(" "));

            // Rendu final aligné
            writeln!(f, "  {:<5} : {:<15} {}",
                     id_str,
                     name,
                     params_str
            )?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Functions
    if !problem.function_defs().is_empty() {
        writeln_centered(f, " [ FUNCTIONS ] ", 80, '=')?;
        writeln!(f, "  {:<5} : {:<15} - {}", "ID", "NAME", "PARAMETERS")?;
        writeln!(f, "{:-<80}", "")?;
        for (i, fct) in problem.function_defs().iter().enumerate() {
            let id_str = format!("#{}", i);
            let name = format!("{}", fct.symbol()).trim().to_string();
            let params: Vec<String> = fct.parameters()
                .iter()
                .map(|param| format!("{}", param))
                .collect();
            let params_str = format!("({})", params.join(" "));
            writeln!(f, "  {:<5} : {:<15} - {}",
                     id_str,
                     name,
                     params_str
            )?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = problem.domain_constraints();
    if dc.is_empty() {
        writeln!(f, "  - no domain constraints\n")?;
    } else {
        writeln!(f, "{}\n", dc)?;
    }

    // Derived predicates
    if problem.derived_predicate_defs().is_empty() {
        writeln!(f, "  - no derived predicates\n")?;
    } else {
        for derived_predicate in problem.derived_predicate_defs() {
            render_derived_predicate(f, derived_predicate)?;
            writeln!(f)?;
        }
    }

    // Actions
    if problem.action_defs().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in problem.action_defs() {
            render_action(f, action)?;
            writeln!(f)?;
        }
    }

    // Methods
    if problem.method_defs().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in problem.method_defs() {
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
    if !domain.has_type_defs() {
        writeln!(f, "  - no types")?;
    } else {
        for t in domain.type_defs() {
            writeln!(f, "  - {}", t)?;
        }
    }
    writeln!(f)?;

    // Constants
    writeln_centered(f, "CONSTANTS", 80, '=')?;
    if !domain.has_constant_defs() {
        writeln!(f, "  - no constants")?;
    } else {
        for c in domain.constant_defs() {
            writeln!(f, "  - {}", c)?;
        }
    }
    writeln!(f)?;

    // Predicates
    writeln_centered(f, "PREDICATES", 80, '=')?;
    if domain.predicate_defs().is_empty() {
        writeln!(f, "  - no predicates")?;
    } else {
        for p in domain.predicate_defs() {
            writeln!(f, "  - {}", p)?;
        }
    }
    writeln!(f)?;

    // Functions
    writeln_centered(f, "FUNCTIONS", 80, '=')?;
    if domain.functions_defs().is_empty() {
        writeln!(f, "  - no functions")?;
    } else {
        for fct in domain.functions_defs() {
            writeln!(f, "  - {}", fct)?;
        }
    }
    writeln!(f)?;

    // Domain constraints
    writeln_centered(f, "DOMAIN CONSTRAINTS", 80, '=')?;
    let dc = domain.constraints();
    if dc.is_empty() {
        writeln!(f, "  - no domain constraints\n")?;
    } else {
        writeln!(f, "{}\n", dc)?;
    }

    // Derived predicates
    if domain.derived_predicates().is_empty() {
        writeln!(f, "  - no derived predicates\n")?;
    } else {
        for derived_predicate in domain.derived_predicates() {
            render_derived_predicate(f, derived_predicate)?;
            writeln!(f)?;
        }
    }

    // Actions
    if domain.action_defs().is_empty() {
        writeln!(f, "  - no actions\n")?;
    } else {
        for action in domain.action_defs() {
            render_action(f, action)?;
            writeln!(f)?;
        }
    }

    // Methods
    if domain.method_defs().is_empty() {
        writeln!(f, "  - no methods\n")?;
    } else {
        for method in domain.method_defs() {
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
    if !problem.has_object_defs() {
        writeln!(f, "  - no objects")?;
    } else {
        for o in problem.object_defs() {
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
    let pc = problem.constraints();
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

/// Effectue le rendu d'une `Action` (simple ou durative) dans un format structuré et lisible.
///
/// Cette fonction adapte dynamiquement l'affichage selon que l'action est temporelle ou non :
/// - Pour les actions simples : Affiche PARAMETERS, PRECONDITION, et EFFECT.
/// - Pour les actions duratives : Affiche PARAMETERS, DURATION, CONDITION, et EFFECT.
///
/// # Paramètres
/// - `f`: Le formateur où écrire la sortie.
/// - `action`: L'instance d'Action à rendre.
pub fn render_action(f: &mut fmt::Formatter<'_>, action: &LiftedAction) -> std::fmt::Result {
    // Détermination du titre et du style de bordure
    let is_durative = action.is_durative();
    let title = if is_durative {
        format!(" DURATIVE ACTION: {} ", action.name())
    } else {
        format!(" ACTION: {} ", action.name())
    };
    let border_char = if is_durative { '-' } else { '=' };

    // En-tête centré
    writeln_centered(f, &title, 80, border_char)?;

    // Paramètres (Commun)
    render_labeled_typed_list(f, "PARAMETERS", action.parameters())?;

    // Durée (Uniquement pour les duratives)
    if let Some(duration) = action.duration() {
        render_labeled_expr(f, "DURATION", duration)?;
    }

    // Condition / Précondition
    // On adapte le label selon le type d'action pour rester proche de la sémantique PDDL
    let cond_label = if is_durative { "CONDITION" } else { "PRECONDITION" };
    render_labeled_expr(f, cond_label, action.precondition())?;

    // Effet (Commun)
    render_labeled_expr(f, "EFFECT", action.effect())?;

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

/// Renders a `LiftedDerivedPredicate` in a human-readable, structured format to a `Formatter`.
///
/// This function prints the derived predicate in two sections:
/// - **HEAD**: The atomic formula skeleton representing the predicate's name and parameters.
/// - **BODY**: The logical expression defining when the derived predicate is true, printed line by line.
///
/// Each section is printed with indentation for readability. A centered section title is added
/// at the top for clarity.
///
/// # Parameters
/// - `f`: A mutable reference to a `std::fmt::Formatter` where the output will be written.
/// - `derived_predicate`: The `LiftedDerivedPredicate` instance to render.
///
/// # Returns
/// Returns a `std::fmt::Result` indicating whether writing to the formatter succeeded.
///
/// # Example
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedDerivedPredicate;
///
/// let derived: LiftedDerivedPredicate = /* create or obtain a derived predicate */;
/// let mut output = String::new();
/// // If rendering via Formatter, use a wrapper:
/// let _ = render_derived_predicate(&mut std::fmt::Formatter::new(&mut output), &derived);
/// println!("{}", output);
/// ```
pub fn render_derived_predicate(
    f: &mut fmt::Formatter<'_>,
    derived_predicate: &LiftedDerivedPredicate,
) -> std::fmt::Result {
    writeln_centered(f, "DERIVED PREDICATE", 80, '-')?;
    writeln!(f, "HEAD: {}", derived_predicate.head())?;
    writeln!(f, "BODY:")?;
    for line in format!("{}", derived_predicate.body()).lines() {
        writeln!(f, "  {}", line)?;
    }
    Ok(())
}

pub fn render_expr(f: &mut fmt::Formatter<'_>, expr: &Expr) -> fmt::Result {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return write!(f, "<Empty Expression>"),
    };

    let mut ancestor_is_last = Vec::new();

    for (id, depth, is_last, node) in expr.preorder_from(root_id) {
        // 1. On aligne la pile sur la profondeur actuelle
        ancestor_is_last.truncate(depth);

        // 2. Rendu du préfixe des ancêtres (3 caractères par niveau)
        for &parent_was_last in ancestor_is_last.iter() {
            if parent_was_last {
                write!(f, "   ")?; // 3 espaces
            } else {
                write!(f, "│  ")?; // Barre + 2 espaces
            }
        }

        // 3. Rendu de la branche actuelle (3 caractères)
        if depth > 0 {
            // "├─ " ou "└─ "
            write!(f, "{}", if is_last { "└─ " } else { "├─ " })?;
        }

        // 4. On enregistre l'état pour les enfants
        ancestor_is_last.push(is_last);

        // 5. Contenu du nœud
        write!(f, "{}", node.kind())?;
        let content = node.content();
        if !matches!(content, Content::None) {
            write!(f, " [{}]", content)?;
        }

        // ID et nouvelle ligne
        writeln!(f, " ({})", id)?;
    }

    Ok(())
}

/// Rendu compact d'un ExprNode au format "Kind (Content)"
pub fn render_expr_node(f: &mut fmt::Formatter<'_>, node: &ExprNode) -> fmt::Result {
        write!(f, "{}", node.kind())?;
    write!(f, " (")?;
    render_node_expr_content(f, node.content())?;
    write!(f, ")")
}

pub fn render_node_expr_content(
    f: &mut fmt::Formatter<'_>,
    content: &Content,
) -> fmt::Result {
    match content {
        Content::None => write!(f, "None"),

        // --- Identifiants (Délégation à tes impl_display_prefix) ---
        Content::Variable(id)   => write!(f, "{}", id), // Sortie ex: v#1
        Content::Constant(id)   => write!(f, "{}", id), // Sortie ex: o#12
        Content::Predicate(id)  => write!(f, "{}", id), // Sortie ex: P#5
        Content::Functor(id)    => write!(f, "{}", id), // Sortie ex: f#2
        Content::TaskSymbol(id) => write!(f, "{}", id), // Sortie ex: tk#3
        Content::TaskID(id)     => write!(f, "{}", id), // Sortie ex: TK#1
        Content::Preference(id) => write!(f, "{}", id), // Sortie ex: pref#0

        // --- Skeletons ---
        Content::AtomSkeleton(id)     => write!(f, "{}", id), // Sortie ex: as#9
        Content::FunctionSkeleton(id) => write!(f, "{}", id), // Sortie ex: fs#2
        Content::TaskSkeleton(id)     => write!(f, "{}", id), // Sortie ex: ts#7

        // --- Valeurs et Opérateurs ---
        Content::Float(val)        => write!(f, "Float({})", val),
        Content::BinaryComp(op)    => write!(f, "BinaryComp({:?})", op),
        Content::AssignOp(op)      => write!(f, "AssignOp({:?})", op),
        Content::ArithmeticOp(op)  => write!(f, "ArithmeticOp({:?})", op),
        Content::Optimization(opt) => write!(f, "Optimization({:?})", opt),

        // --- Listes ---
        Content::QuantifierVariables(vars) => {
            // Ici, si TypedList n'implémente pas Display, on affiche juste la taille
            write!(f, "QuantifierVars(len:{})", vars.len())
        }
    }
}
