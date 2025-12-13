use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lir::problem::{renderers, InitialTaskNetwork, LiftedAction, LiftedMethod, LiftedTaskNetwork};
use crate::aiplan4rust::syntax::lexer::Token;
use crate::aiplan4rust::syntax::{display, SyntaxInternerDisplay};
use std::fmt;
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};
use crate::aiplan4rust::syntax::tree::SyntaxNode;

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
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
    }

    // Types
    if !domain.constants().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Types)?;
    for t in domain.types() {
            writeln!(f, "    {}", t.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
    }


    // Constants
    if !domain.constants().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Constants)?;
        for c in domain.constants() {
            writeln!(f, "    {}", c.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
    }

    // Predicates
    if !domain.predicates().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Predicates)?;
        for p in domain.predicates() {
            writeln!(f, "    {}", p.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
    }

    // Functions
    if !domain.functions().is_empty() {
        writeln!(f, "  {}{}", Token::LParen, Token::Functions)?;
        for func in domain.functions() {
            writeln!(f, "    {}", func.to_syntax_string_with_interner(interner))?;
        }
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
    }

    // Domain constraints
    let dc = domain.domain_constraints();
    if !dc.is_empty() {
        write!(f, "  {}{} ", Token::LParen, Token::Constraints)?;
        writeln!(f, "{}", dc.to_syntax_string_with_interner(interner))?;
        writeln!(f, "  {}", Token::RParen)?;
        writeln!(f)?;
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
