use std::fmt;
// typed_list.rs
use crate::aiplan4rust::lang::TypeID;
use crate::aiplan4rust::lang::TypedSymbol;
use crate::aiplan4rust::lir::problem::renderers::RenderContext; // Importe ton contexte

pub fn render(
    f: &mut fmt::Formatter<'_>,
    parameters: &[TypedSymbol<TypeID>],
    ctx: &RenderContext
) -> fmt::Result {
    let mut i = 0;

    while i < parameters.len() {
        let current_type = parameters[i].ty();
        let mut j = i;

        // 1. On groupe les variables qui ont le même TypeID
        while j < parameters.len() && parameters[j].ty() == current_type {
            // On résout le nom du symbole
            let name = ctx.resolve_ident(parameters[j].symbol());
            write!(f, "?{} ", name)?;
            j += 1;
        }

        // 2. On ajoute le séparateur
        write!(f, "- ")?;

        // 3. Résolution et affichage du nom du type
       // let type_name = ctx.resolve_type(current_type);
        //write!(f, "{}", type_name)?;

        // 4. Espace entre les groupes si nécessaire
        if j < parameters.len() {
            write!(f, " ")?;
        }

        i = j;
    }

    Ok(())
}
