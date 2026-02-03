use std::fmt;
use std::io::Lines;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::problem::renderers::render_context::RenderContext;
use crate::aiplan4rust::lir::problem::renderers::syntax::action;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    problem: &LiftedProblem,
) -> std::fmt::Result  {
    let ctx = RenderContext::new(problem);
    if !problem.actions().is_empty() {
        for action in problem.actions() {
            action::render(f, action, &ctx)?;
            writeln!(f)?;
        }
    }
    Ok(())
}
