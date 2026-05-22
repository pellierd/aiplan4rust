use crate::aiplan4rust::lang::{Type, TypeId};
use crate::aiplan4rust::lir::renderers::RenderContext;
use std::fmt;

pub fn render(f: &mut fmt::Formatter<'_>, ty: &Type<TypeId>, ctx: &RenderContext) -> fmt::Result {
    let members = ty.members();

    match members.len() {
        0 => Ok(()), // Aucun typing spécifié
        1 => {
            // Cas standard : " - type_name (id)"
            write!(f, " - ")?;
            let ty_id = members[0];
            let type_name = ctx.resolve_type(ty_id);
            write!(f, "{} ({})", type_name, ty_id)
        }
        _ => {
            // Cas composite : " - (either type1 (id1) type2 (id2) ...)"
            write!(f, " - (either")?;
            for m_id in members {
                let m_name = ctx.resolve_type(*m_id);
                write!(f, " {} ({})", m_name, m_id)?;
            }
            write!(f, ")")
        }
    }
}
