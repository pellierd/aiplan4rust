use crate::aiplan4rust::compiler::lir::renderers::syntax::ty;
use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use crate::aiplan4rust::support::lang::{ObjectId, TypeId, TypedSymbol, VariableId};
use std::fmt;

/// Rendu syntaxique PDDL pour une liste de variables : ?x0 - type ?x1 - type
pub fn render_typed_variable_list(
    f: &mut fmt::Formatter<'_>,
    parameters: &[TypedSymbol<VariableId, TypeId>],
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, param) in parameters.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        // En PDDL syntaxique, on utilise le nom résolu ou un format standard sans ID technique
        // Si tu as le nom original dans le ctx, utilise-le, sinon ?x{id}
        write!(f, "?x{}", param.symbol().as_usize())?;

        // Rendu du typing : " - type"
        ty::render(f, param.ty(), ctx)?;
    }
    Ok(())
}

/// Rendu syntaxique PDDL pour une liste d'objets ou constantes : obj1 - type obj2 - type
pub fn render_typed_object_list(
    f: &mut fmt::Formatter<'_>,
    objects: &[TypedSymbol<ObjectId, TypeId>],
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, obj) in objects.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        let name = ctx.resolve_object(obj.symbol());
        write!(f, "{}", name)?;

        // Rendu du typing : " - type"
        ty::render(f, obj.ty(), ctx)?;
    }
    Ok(())
}

/// Rendu syntaxique PDDL pour la hiérarchie des types : type - parent
pub fn render_typed_type_list(
    f: &mut fmt::Formatter<'_>,
    types: &[TypedSymbol<TypeId, TypeId>],
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, t_def) in types.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        let name = ctx.resolve_type(t_def.symbol());
        write!(f, "{}", name)?;

        // Rendu du parent : " - parent"
        // ty::render gère déjà le fait de ne rien écrire si le type est 'object'
        ty::render(f, t_def.ty(), ctx)?;
    }
    Ok(())
}
