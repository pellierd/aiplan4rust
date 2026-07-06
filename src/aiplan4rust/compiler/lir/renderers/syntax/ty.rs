use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use crate::aiplan4rust::support::lang::{Type, TypeId};
use std::fmt;

/// Rendu syntaxique PDDL pour un Type.
/// Produit " - type" ou " - (either type1 type2)"
pub fn render(
    f: &mut fmt::Formatter<'_>,
    ty: &Type<TypeId>,
    ctx: &LirRenderContext,
) -> fmt::Result {
    let members = ty.members();

    match members.len() {
        // Cas 0 : Pas de type (souvent implicitement 'object' en PDDL)
        0 => Ok(()),

        // Cas 1 : Type atomique standard
        1 => {
            let ty_id = members[0];
            let type_name = ctx.resolve_type(ty_id);
            write!(f, " - {}", type_name)
        }

        // Cas > 1 : Union de types (Syntaxe 'either')
        _ => {
            write!(f, " - (either")?;
            for m_id in members {
                let m_name = ctx.resolve_type(*m_id);
                write!(f, " {}", m_name)?;
            }
            write!(f, ")")
        }
    }
}
