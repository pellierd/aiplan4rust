use std::fmt;
// type
use crate::aiplan4rust::lang::{Type, TypeID};
use crate::aiplan4rust::lir::renderers::RenderContext; // Importe ton contexte



pub fn render(
    f: &mut fmt::Formatter<'_>,
    ty: &Type<TypeID>,
    ctx: &RenderContext,
) -> fmt::Result {
    let members = ty.members();

    match members.len() {
        0 => Ok(()), // Aucun type spécifié (ex: constantes sans type)
        1 => {
            // Cas standard : " - type_name"
            write!(f, " - ")?;
            let ty_id = members[0];

            // Utilisation de la méthode de résolution simplifiée du contexte
            let type_name = ctx.resolve_type(ty_id);
            write!(f, "{}", type_name)
        }
        _ => {
            // Cas des types composites (PDDL/HDDL) : " - (either type1 type2 ...)"
            write!(f, " - (either")?;
            for m_id in members {
                let m_name = ctx.resolve_type(*m_id);
                write!(f, " {}", m_name)?;
            }
            write!(f, ")")
        }
    }
}
