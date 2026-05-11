use crate::aiplan4rust::lir::store::expr::ExprId;
use crate::aiplan4rust::lir::store::problem::NewLiftedProblem;
use crate::aiplan4rust::lir::store::renderers::debug::common::writeln_centered;
use crate::aiplan4rust::lir::store::renderers::debug::{
    action, atom, expr, function, method, typed_list,
};
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

/// Structure privée pour faire le pont avec le système de formatage de Rust.
/// Elle transporte maintenant une référence au problème ET au contexte de rendu.
struct ProblemWrapper<'a> {
    problem: &'a NewLiftedProblem,
    ctx: &'a RenderContext<'a>,
}

impl<'a> fmt::Display for ProblemWrapper<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // On délègue à ta fonction de rendu en passant le contexte
        render(f, self.problem, self.ctx)
    }
}

/// Retourne la représentation textuelle complète du problème.
/// Nécessite le contexte pour résoudre les symboles (IDs -> Noms).
pub fn to_string(problem: &NewLiftedProblem, ctx: &RenderContext) -> String {
    // La méthode .to_string() est automatiquement fournie par le trait Display
    ProblemWrapper { problem, ctx }.to_string()
}

/// Renders a `LiftedProblem` in a structured, human-readable format.
pub fn render(
    f: &mut fmt::Formatter<'_>,
    problem: &NewLiftedProblem,
    ctx: &RenderContext,
) -> fmt::Result {
    // === HEADER ===
    writeln_centered(f, " [ LIFTED PROBLEM ] ", 80, '=')?;
    writeln!(
        f,
        "  {:<15} : {}",
        "DOMAIN NAME",
        ctx.resolve_symbol(problem.domain_name())
    )?;
    writeln!(
        f,
        "  {:<15} : {}",
        "PROBLEM NAME",
        ctx.resolve_symbol(problem.problem_name())
    )?;
    writeln!(f, "{:-<80}\n", "")?;

    // === REQUIREMENTS ===
    if !problem.requirements().is_empty() {
        writeln_centered(f, " [ REQUIREMENTS ] ", 80, '=')?;
        let mut reqs: Vec<_> = problem.requirements().iter().collect();
        reqs.sort();
        for r in reqs {
            writeln!(f, "  - {:?}", r)?;
        }
        writeln!(f, "{:-<80}\n", "")?;
    }

    // === TYPES (Hiérarchie) ===
    if !problem.type_defs().is_empty() {
        writeln_centered(f, " [ TYPES HIERARCHY ] ", 80, '=')?;
        // Utilise notre fonction slice alignée
        typed_list::render_type_typed_list(f, problem.type_defs().as_slice(), ctx)?;
        writeln!(f, "\n{:-<80}\n", "")?;
    }

    // === CONSTANTS (Domaine) ===
    let constants = problem.domain_constant_def(); // C'est une slice &[T]
    if !constants.is_empty() {
        writeln_centered(f, " [ DOMAIN CONSTANTS ] ", 80, '=')?;
        typed_list::render_object_typed_list(f, constants, ctx)?;
        writeln!(f, "\n{:-<80}\n", "")?;
    }

    // === OBJECTS (Problème) ===
    let objects = problem.problem_object_def(); // C'est une slice &[T]
    if !objects.is_empty() {
        writeln_centered(f, " [ PROBLEM OBJECTS ] ", 80, '=')?;
        typed_list::render_object_typed_list(f, objects, ctx)?;
        writeln!(f, "\n{:-<80}\n", "")?;
    }

    // === PREDICATES ===
    writeln_centered(f, " PREDICATES ", 80, '-')?;
    if problem.predicate_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for p in problem.predicate_defs() {
            write!(f, "  ")?;
            // Signature : (nom ?arg1 - type)
            atom::render(f, p, ctx)?;
            writeln!(f)?;
        }
    }
    writeln!(f)?;

    // === FUNCTIONS ===
    writeln_centered(f, " FUNCTIONS (FLUENTS) ", 80, '-')?;
    if problem.function_defs().is_empty() {
        writeln!(f, "  <None>")?;
    } else {
        for func in problem.function_defs() {
            write!(f, "  ")?;
            function::render(f, func, ctx)?;
            writeln!(f)?;
        }
    }
    writeln!(f)?;
    // === CONSTRAINTS (Domain & Problem) ===
    writeln_centered(f, " [ CONSTRAINTS ] ", 80, '=')?;
    let dc = problem.domain_constraints();
    if dc != ExprId::EMPTY_AND {
        write!(f, "  DOMAIN: ")?;
        expr::render(f, dc, ctx)?;
        writeln!(f)?;
    }
    let pc = problem.problem_constraints();
    if pc != ExprId::EMPTY_AND {
        write!(f, "  PROBLEM: ")?;
        expr::render(f, pc, ctx)?;
        writeln!(f)?;
    }
    writeln!(f)?;

    // === INITIAL STATE (INIT) ===
    writeln_centered(f, " [ INITIAL STATE ] ", 80, '=')?;
    let init = problem.init();
    if init == ExprId::EMPTY_AND {
        writeln!(f, "  <Empty Init>")?;
    } else {
        expr::render(f, init, ctx)?;
    }
    writeln!(f, "\n")?;

    // === GOAL ===
    writeln_centered(f, " [ GOAL ] ", 80, '=')?;
    let goal = problem.goal();
    if goal == ExprId::EMPTY_AND {
        writeln!(f, "  <No Goal Specified>")?;
    } else {
        expr::render(f, goal, ctx)?;
    }
    writeln!(f, "\n")?;

    // === ACTIONS & METHODS ===
    // (Délégué aux modules spécialisés comme dans DomainDef)
    if !problem.action_defs().is_empty() {
        writeln_centered(f, " [ ACTIONS ] ", 80, '=')?;
        for action in problem.action_defs() {
            action::render(f, action, ctx)?;
            writeln!(f)?;
        }
    }

    if !problem.method_defs().is_empty() {
        writeln_centered(f, " [ METHODS ] ", 80, '=')?;
        for method in problem.method_defs() {
            method::render(f, method, ctx)?;
            writeln!(f)?;
        }
    }

    // === HDDL: INITIAL TASK NETWORK ===
    writeln_centered(f, " [ INITIAL TASK NETWORK ] ", 80, '=')?;
    // Supposant un module task_network::render
    // task_network::render(f, problem.initial_task_network(), ctx)?;

    // === METRICS & SPECS ===
    writeln_centered(f, " [ SPECS & METRICS ] ", 80, '=')?;
    writeln!(f, "  METRIC: {:?}", problem.metric_spec())?;
    writeln!(f, "  LENGTH: {:?}", problem.length_spec())?;

    writeln!(f, "\n{}", "=".repeat(80))
}
