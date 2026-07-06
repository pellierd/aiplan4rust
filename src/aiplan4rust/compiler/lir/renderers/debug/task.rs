use crate::aiplan4rust::compiler::lir::problem::skeleton::task::Task;
use crate::aiplan4rust::compiler::lir::renderers::debug::typed_list;
use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use std::fmt::{self, Formatter};

/// Rendu d'une signature de tâche en mode Debug (Hybride).
/// Format : (nom_tache [t#ID] ?x0 [v#0] - type1 [t#ID])
pub fn render(f: &mut Formatter<'_>, task: &Task, ctx: &LirRenderContext) -> fmt::Result {
    // 1. Récupération de l'ID et du nom de la tâche
    let symbol_id = task.task_symbol();
    let name = ctx.resolve_task_symbol(symbol_id);

    // 2. Début du bloc avec le nom et l'ID technique [t#ID] (t pour task)
    write!(f, "({} [t#{}]", name, symbol_id.as_usize())?;

    // 3. Paramètres : Récupération sécurisée via le store
    if let Some(params_list) = ctx.store().get_typed_list(task.parameters()) {
        if !params_list.is_empty() {
            write!(f, " ")?;
            // Ce renderer affiche "?nom [v#ID] - type [t#ID]"
            typed_list::render_variable_typed_list(f, params_list.as_slice(), ctx)?;
        }
    } else {
        write!(f, " <error: params not found>")?;
    }

    // 4. Fermeture de la signature
    write!(f, ")")
}
