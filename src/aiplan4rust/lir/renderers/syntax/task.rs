use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicTaskSkeleton;
use crate::aiplan4rust::lir::renderers::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::typed_list;

pub fn render(
    f: &mut Formatter<'_>,
    task: &AtomicTaskSkeleton,
    ctx: &RenderContext
) -> fmt::Result {
    // 1. Résolution du nom (ex: 'at')
    write!(f, "(:task {}", ctx.resolve_task_symbol(task.task_symbol()))?;
    write!(f, "\n    (:parameters")?;
    let parameters = task.parameters();
    if !parameters.is_empty() {
        write!(f, " ")?;
        typed_list::render_typed_variable_list(f, parameters, ctx)?;
    }
    write!(f, ")")?;
    write!(f, "\n  )")

}
