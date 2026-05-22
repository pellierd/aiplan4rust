use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lang::{ObjectId, TypeId, VariableId};
use crate::aiplan4rust::lir::old::renderers::syntax::ty;
use crate::aiplan4rust::lir::old::renderers::RenderContext;
use std::fmt;

pub fn render_typed_variable_list(
    f: &mut fmt::Formatter<'_>,
    parameters: &[TypedSymbol<VariableId, TypeId>],
    ctx: &RenderContext,
) -> fmt::Result {
    for (i, param) in parameters.iter().enumerate() {
        // Ajouter un espace avant chaque variable sauf la première
        if i > 0 {
            write!(f, " ")?;
        }

        // 1. Nom de la variable
        write!(f, "?x{}", param.symbol().as_usize())?;

        // 2. Utilisation de la fonction de rendu de typing
        ty::render(f, param.ty(), ctx)?;
    }
    Ok(())
}

pub fn render_typed_object_list(
    f: &mut fmt::Formatter<'_>,
    objects: &[TypedSymbol<ObjectId, TypeId>],
    ctx: &RenderContext,
) -> fmt::Result {
    for (i, obj) in objects.iter().enumerate() {
        // 1. Gestion de l'espacement entre les objets
        if i > 0 {
            write!(f, " ")?;
        }

        // 2. Résolution du nom de l'objet via le Renderer (via le Context)
        // resolve_object utilise en interne table.try_get_string(id, interner)
        let name = ctx.resolve_object(obj.symbol());
        write!(f, "{}", name)?;

        // 3. Rendu du typing associé
        // On réutilise la fonction render_type qui utilise aussi le Context
        ty::render(f, obj.ty(), ctx)?;
    }
    Ok(())
}
