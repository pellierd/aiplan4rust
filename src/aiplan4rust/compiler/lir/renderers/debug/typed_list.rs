use crate::aiplan4rust::compiler::lir::renderers::debug::ty;
use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use crate::aiplan4rust::support::lang::{ObjectId, TypeId, TypedSymbol, VariableId};
use std::fmt;

/// Rendu pour n'importe quelle séquence de variables : ?x0 [v#0] - type (id)
pub fn render_variable_typed_list(
    f: &mut fmt::Formatter<'_>,
    parameters: &[TypedSymbol<VariableId, TypeId>], // Slice
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, param) in parameters.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        let var_id = param.symbol();
        // Format : ?xID [v#ID]
        write!(f, "?x{} [{}]", var_id.as_usize(), var_id)?;

        // Rendu du type (ajoute " - type (id)")
        ty::render(f, param.ty(), ctx)?;
    }
    Ok(())
}

/// Rendu pour n'importe quelle séquence d'objets : Nom [o#ID] - type (id)
pub fn render_object_typed_list(
    f: &mut fmt::Formatter<'_>,
    objects: &[TypedSymbol<ObjectId, TypeId>], // Slice
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, obj) in objects.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        let obj_id = obj.symbol();
        let name = ctx.resolve_object(obj_id);

        // Format : Nom [o#ID]
        write!(f, "{} [{}]", name, obj_id)?;

        // Rendu du type (ajoute " - type (id)")
        ty::render(f, obj.ty(), ctx)?;
    }
    Ok(())
}

/// Rendu pour n'importe quelle séquence de types : Nom [t#ID] - parent (id)
pub fn render_type_typed_list(
    f: &mut fmt::Formatter<'_>,
    types: &[TypedSymbol<TypeId, TypeId>], // Slice
    ctx: &LirRenderContext,
) -> fmt::Result {
    for (i, t_def) in types.iter().enumerate() {
        if i > 0 {
            write!(f, " ")?;
        }

        let type_id = t_def.symbol();
        let name = ctx.resolve_type(type_id);

        // Format : Nom [t#ID]
        write!(f, "{} [{}]", name, type_id)?;

        // Rendu du parent (ajoute " - parent (id)" ou " - (either ...)")
        ty::render(f, t_def.ty(), ctx)?;
    }
    Ok(())
}
