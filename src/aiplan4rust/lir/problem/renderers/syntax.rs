use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lir::problem::{renderers, InitialTaskNetwork, LiftedAction, LiftedMethod, LiftedTaskNetwork};
use crate::aiplan4rust::syntax::lexer::Token;
use crate::aiplan4rust::syntax::{display, SyntaxInternerDisplay};
use std::fmt;
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};
use crate::aiplan4rust::syntax::tree::SyntaxNode;

/// Renders a `DomainDef` as a PDDL-like syntax string.
///
/// This function formats a domain definition into the provided formatter, using the
/// given `StringInterner` to resolve interned identifiers. It includes the domain name,
/// requirements, types, constants, predicates, functions, actions, methods, and
/// domain-level constraints. The output is structured in a readable, indented format
/// suitable for syntax display or serialization.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the formatted domain into.
/// * `domain` - The [`DomainDef`] representing the domain to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for types,
///   constants, predicates, functions, and other interned names.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::DomainDef;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_domain_def;
///
/// # let domain: DomainDef = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_domain_def(&mut s, &domain, &interner).unwrap();
/// println!("{}", s);
/// ```
///
/// # Errors
///
/// Returns any [`fmt::Error`] encountered while writing to the formatter.
pub fn render_domain_def(
    f: &mut fmt::Formatter<'_>,
    domain: &DomainDef,
    interner: &StringInterner,
) -> fmt::Result {
    write!(f, "{}{} ", Token::LParen, Token::Define)?;
    write!(f, "{}{} ", Token::LParen, Token::Domain)?;
    writeln!(
        f,
        "{}{} ",
        domain.domain_name().to_syntax_string_with_interner(interner),
        Token::RParen
    )?;
    writeln!(f)?;

    // Requirements
    let mut reqs: Vec<_> = domain.requirements().iter().collect();
    if !reqs.is_empty() {
        reqs.sort();
        writeln!(f, "  {}{}", Token::LParen, Token::Requirements)?;
        for r in reqs {
            writeln!(f, "    {}", r)?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Types
    if !domain.constants().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Types)?;
    for t in domain.types() {
            writeln!(f, "    {}", t.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Constants
    if !domain.constants().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Constants)?;
        for c in domain.constants() {
            writeln!(f, "    {}", c.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Predicates
    if !domain.predicates().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Predicates)?;
        for p in domain.predicates() {
            writeln!(f, "    {}", p.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Functions
    if !domain.functions().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Functions)?;
        for func in domain.functions() {
            writeln!(f, "    {}", func.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Domain constraints
    let dc = domain.domain_constraints();
    if !dc.is_empty() {
        write!(f, "  {}{} ", Token::LParen, Token::Constraints)?;
        writeln!(f, "{}", dc.to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}\n", Token::RParen)?;
    }
    // Actions
    for action in domain.actions() {
        renderers::syntax::render_action(f, action, interner, 1)?;
        writeln!(f)?;
    }
    // Methods
    for method in domain.methods() {
        renderers::syntax::render_method(f, method, interner, 1)?;
        writeln!(f)?;
    }

    writeln!(f, "{}", Token::RParen)?;

    Ok(())
}

/// Renders a `ProblemDef` as a PDDL-like syntax string.
///
/// This function formats a problem definition into the provided formatter, using the
/// given `StringInterner` to resolve interned identifiers. It includes the problem name,
/// associated domain, requirements, objects, initial task network, initial state,
/// goal, problem-specific constraints, metric and length specifications.
///
/// The output is structured in a readable, indented format suitable for syntax display
/// or serialization.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the formatted problem into.
/// * `problem` - The [`ProblemDef`] representing the problem to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for objects,
///   facts, tasks, and other interned names.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.

/// Each section is indented appropriately to enhance readability.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::ProblemDef;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_problem_def;
///
/// # let problem: ProblemDef = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_problem_def(&mut s, &problem, &interner).unwrap();
/// println!("{}", s);
/// ```
///
/// # Errors
///
/// Returns any [`fmt::Error`] encountered while writing to the formatter.
pub fn render_problem_def(
    f: &mut fmt::Formatter<'_>,
    problem: &ProblemDef,
    interner: &StringInterner,
) -> fmt::Result {

    write!(f, "{}{} ", Token::LParen, Token::Define)?;
    write!(f, "{}{} ", Token::LParen, Token::Problem)?;
    writeln!(
        f,
        "{}{} ",
        problem.problem_name().to_syntax_string_with_interner(interner),
        Token::RParen
    )?;
    writeln!(f)?;

    write!(f, "  {}{} ", Token::LParen, Token::Domain)?;
    writeln!(f, "{}{}\n", problem.domain_name().to_syntax_string_with_interner(interner), Token::RParen)?;

    // Requirements
    let mut reqs: Vec<_> = problem.requirements().iter().collect();
    if !reqs.is_empty() {
        reqs.sort();
        writeln!(f, "  {}{}", Token::LParen, Token::Requirements)?;
        for r in reqs {
            writeln!(f, "    {}", r)?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Objects
    if !problem.objects().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Objects)?;
        for o in problem.objects() {
            writeln!(f, "    {}", o.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Initial Task Network
    writeln!(f, "  {}{}", Token::LParen, Token::Htn)?;
    render_initial_task_network(f, problem.initial_task_network(), interner, 2)?;
    writeln!(f, "  {}\n", Token::RParen)?;

    // Init
    writeln!(f, "  {}{}", Token::LParen, Token::Init)?;
    let init_expr = problem.init();
    if let Some(init) = init_expr.root_node() {
        for fact_id in init.children() {
            if let Some(fact) = init_expr.get_node(*fact_id) {
                writeln!(f, "    {}", fact.to_syntax_string(init_expr, interner))?;
            } else {
                writeln!(f, "    <unknown>")?;
            }
        }
    }
    writeln!(f, "  {}\n", Token::RParen)?;

    // Goal
    if !problem.goal().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Goal)?;
        writeln!(f, "    {}", problem.goal().to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Problem constraints
    if !problem.problem_constraints().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Constraints)?;
        writeln!(f, "    {}", problem.problem_constraints().to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Metric spec
    if !problem.metric_spec().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Metric)?;
        writeln!(f, "    {}\n", problem.metric_spec().to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    // Length spec
    if !problem.length_spec().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Length)?;
        writeln!(f, "    {}\n", problem.length_spec().to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}\n", Token::RParen)?;
    }

    writeln!(f, "{}", Token::RParen)?;

    Ok(())
}

/// Renders a lifted action (`LiftedAction`) into a PDDL-like syntax representation.
///
/// This function formats an action with its name, parameters, precondition, and effect
/// into the provided formatter. The `StringInterner` is used to resolve interned identifiers,
/// ensuring that the output uses readable names instead of numeric or internal IDs. The
/// `indent` parameter allows controlling the indentation level for pretty-printing.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the rendered action into.
/// * `action` - The [`LiftedAction`] to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for the action's name and parameters.
/// * `indent` - The indentation level (number of indent units) to apply to the rendered action.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedAction;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_action;
///
/// # let action: LiftedAction = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_action(&mut s, &action, &interner, 2).unwrap();
/// println!("{}", s);
/// ```
///
/// # Errors
///
/// Returns any [`fmt::Error`] encountered while writing to the formatter.
pub fn render_action(
    f: &mut fmt::Formatter<'_>,
    action: &LiftedAction,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {
    display::write_indent(f, indent)?;
    write!(f, "{} {} ", Token::LParen, Token::Action)?;
    writeln!(f, "{}", interner.resolve_ident(action.name()).unwrap_or("<unknown>"))?;

    let params = action
        .parameters()
        .iter()
        .map(|p| p.to_syntax_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(" ");

    display::write_indent(f, indent)?;
    writeln!(f, "  {} {}{}{}", Token::Parameters, Token::LParen, params, Token::RParen)?;

    display::write_indent(f, indent)?;
    writeln!(
        f,
        "  {} {}",
        Token::Precondition,
        action.precondition().to_syntax_string_with_interner(interner)
    )?;
    display::write_indent(f, indent)?;
    writeln!(
        f,
        "  {} {}",
        Token::Effect,
        action.effect().to_syntax_string_with_interner(interner)
    )?;
    display::write_indent(f, indent)?;
    writeln!(f, "{}", Token::RParen)?;

    Ok(())
}

/// Renders a lifted method (`LiftedMethod`) into a PDDL-like syntax representation.
///
/// This function formats a method with its name, parameters, task, precondition, and
/// task network into the provided formatter. The `StringInterner` is used to resolve
/// interned identifiers, so the output uses readable names instead of numeric or
/// internal IDs. The `indent` parameter allows controlling the indentation level for
/// pretty-printing.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the rendered method into.
/// * `method` - The [`LiftedMethod`] to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for the method's name, parameters, and task.
/// * `indent` - The indentation level (number of indent units) to apply to the rendered method.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedMethod;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_method;
///
/// # let method: LiftedMethod = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_method(&mut s, &method, &interner, 2).unwrap();
/// println!("{}", s);
/// ```
///
/// # Errors
///
/// Returns any [`fmt::Error`] encountered while writing to the formatter.
pub fn render_method(
    f: &mut fmt::Formatter<'_>,
    method: &LiftedMethod,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {

    display::write_indent(f, indent)?;
    write!(f, "{}{} ", Token::LParen, Token::Method)?;
    writeln!(f, "{}", interner.resolve_ident(method.name()).unwrap_or("<unknown>"))?;

    let params = method
        .parameters()
        .iter()
        .map(|p| p.to_syntax_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(" ");

    display::write_indent(f, indent)?;
    writeln!(f, "  {} {}{}{}", Token::Parameters, Token::LParen, params, Token::RParen)?;

    display::write_indent(f, indent)?;
    writeln!(f, "  {} {}", Token::Task, method.task().to_syntax_string_with_interner(interner))?;

    display::write_indent(f, indent)?;
    writeln!(
        f,
        "  {} {}",
        Token::Precondition,
        method.precondition().to_syntax_string_with_interner(interner)
    )?;

    display::write_indent(f, indent)?;
    render_task_network(f, &method.task_network(), interner, indent)?;

    display::write_indent(f, indent)?;
    writeln!(f, ")")?;

    Ok(())
}

/// Renders a lifted task network (`LiftedTaskNetwork`) into a PDDL-like syntax representation.
///
/// This function formats the tasks, ordering constraints, and logical constraints of a task network
/// into the provided formatter. The `StringInterner` is used to resolve interned identifiers,
/// so the output uses human-readable names rather than internal IDs. Indentation is applied
/// according to the `indent` parameter for readability.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the rendered task network into.
/// * `network` - The [`LiftedTaskNetwork`] to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for tasks and constraints.
/// * `indent` - The indentation level (number of indent units) to apply to the rendered task network.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::LiftedTaskNetwork;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_task_network;
///
/// # let network: LiftedTaskNetwork = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_task_network(&mut s, &network, &interner, 2).unwrap();
/// println!("{}", s);
/// ```
///
/// # Notes
///
/// - If the task network is empty, no `(tasks ...)` or `(ordered-tasks ...)` section is written.
/// - Ordering and logical constraints are only rendered if they are non-empty.
/// - This function is intended to be used by higher-level renderers such as `render_method` or `render_problem_def`.
pub fn render_task_network(
    f: &mut fmt::Formatter<'_>,
    network: &LiftedTaskNetwork,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {
    if !network.tasks().is_empty() {
        display::write_indent(f, indent)?;
        if network.is_declared_total_ordered() {
            write!(f, "{} ", Token::OrderedTasks)?;
        } else {
            write!(f, "{} ", Token::Tasks)?;
        }
        writeln!(f, "{}", network.tasks().to_syntax_string_with_interner(interner))?;
    }
    if !network.ordering_constraints().is_empty() {
        display::write_indent(f, indent)?;
        writeln!(f, "{} {}", Token::Ordering, network.ordering_constraints().to_syntax_string_with_interner(interner))?;
    }
    if !network.ordering_constraints().is_empty() {
        display::write_indent(f, indent)?;
        writeln!(f, "{} {}", Token::Constraints, network.logical_constraints().to_syntax_string_with_interner(interner))?;
    }
    Ok(())
}

/// Renders an initial task network (`InitialTaskNetwork`) into a PDDL-like syntax representation.
///
/// This function formats the parameters and task network of the initial task network
/// into the provided formatter. The `StringInterner` is used to resolve interned identifiers,
/// producing human-readable names. Indentation is applied according to the `indent` parameter
/// for readability.
///
/// # Arguments
///
/// * `f` - The [`fmt::Formatter`] to write the rendered output into.
/// * `init_network` - The [`InitialTaskNetwork`] to render.
/// * `interner` - The [`StringInterner`] used to resolve identifiers for parameters and tasks.
/// * `indent` - The indentation level (number of indent units) to apply to the rendered output.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating whether the formatting was successful.
///
/// Sections are omitted if the corresponding component is empty.
///
/// # Example
///
/// ```rust
/// use std::fmt::Write;
/// use crate::aiplan4rust::lir::problem::InitialTaskNetwork;
/// use crate::aiplan4rust::interner::StringInterner;
/// use crate::aiplan4rust::lir::problem::renderers::syntax::render_initial_task_network;
///
/// # let init_network: InitialTaskNetwork = todo!();
/// # let interner: StringInterner = todo!();
/// let mut s = String::new();
/// render_initial_task_network(&mut s, &init_network, &interner, 2).unwrap();
/// println!("{}", s);
/// ```
///
/// # Notes
///
/// - If the task network contains no tasks, the `(tasks ...)` section is omitted.
/// - Ordering and logical constraints are only rendered if they exist.
/// - This function is mainly used internally by `render_problem_def` when rendering the initial tasks of a problem.
pub fn render_initial_task_network(
    f: &mut fmt::Formatter<'_>,
    init_network: &InitialTaskNetwork,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {

    let params = init_network
        .parameters()
        .iter()
        .map(|p| p.to_syntax_string_with_interner(interner))
        .collect::<Vec<_>>()
        .join(" ");

    display::write_indent(f, indent)?;
    writeln!(f, "{} {}{}{}", Token::Parameters, Token::LParen, params, Token::RParen)?;

    let network = init_network.task_network();
    if !network.tasks().is_empty() {
        display::write_indent(f, indent)?;
        if network.is_declared_total_ordered() {
            write!(f, "{} ", Token::OrderedTasks)?;
        } else {
            write!(f, "{} ", Token::Tasks)?;
        }
        writeln!(f, "{}", network.tasks().to_syntax_string_with_interner(interner))?;
    }
    if !network.ordering_constraints().is_empty() {
        display::write_indent(f, indent)?;
        writeln!(f, "{} {}", Token::Ordering, network.ordering_constraints().to_syntax_string_with_interner(interner))?;
    }
    if !network.logical_constraints().is_empty() {
        display::write_indent(f, indent)?;
        writeln!(f, "{} {}", Token::Constraints, network.logical_constraints().to_syntax_string_with_interner(interner))?;
    }

    Ok(())
}
