use crate::aiplan4rust::lir::store::problem::skeleton::AtomicTaskSkeleton;
use crate::aiplan4rust::lir::store::renderers::syntax::typed_list;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;
use std::fmt::Formatter;

/// Rendu syntaxique HDDL d'une déclaration de tâche.
/// Format :
/// (:task task_name
///     :parameters (?arg1 - type1 ...)
/// )
pub fn render(
    f: &mut Formatter<'_>,
    task: &AtomicTaskSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début du bloc de tâche et résolution du nom du symbole de tâche
    let task_name = ctx.resolve_task_symbol(task.task_symbol());
    write!(f, "(:task {}", task_name)?;

    // 2. Bloc des paramètres (indenté pour la lisibilité HDDL)
    write!(f, "\n    :parameters (")?;
    let parameters = task.parameters();
    if !parameters.is_empty() {
        // On réutilise notre renderer de liste de variables (?x - type)
        typed_list::render_typed_variable_list(f, parameters.as_slice(), ctx)?;
    }
    write!(f, ")")?;

    // 3. Fermeture du bloc (:task ...)
    write!(f, "\n  )")
}
