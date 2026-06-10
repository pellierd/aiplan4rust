use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::compiler::lir::renderers::debug::typed_list;
use crate::aiplan4rust::compiler::lir::renderers::{debug, RenderContext};
use crate::aiplan4rust::support::lang::{TypeId, TypedList, VariableId};
use std::fmt;

// Taille standard d'un palier d'indentation.
pub const INDENT_SIZE: usize = 2;

/// Écrit l'indentation correspondant au niveau demandé.
pub fn write_indent(f: &mut fmt::Formatter<'_>, level: usize) -> fmt::Result {
    write!(f, "{}", " ".repeat(level * INDENT_SIZE))
}

/// Écrit un titre centré avec des caractères de remplissage.
/// Note : On utilise `fmt::Write` pour être compatible avec les formatters.
pub fn writeln_centered(
    f: &mut dyn fmt::Write,
    title: &str,
    width: usize,
    fill: char,
) -> fmt::Result {
    if title.len() >= width {
        writeln!(f, "{}", title)
    } else {
        let total_fill = width - title.len();
        let left_fill = total_fill / 2;
        let right_fill = total_fill - left_fill;
        writeln!(
            f,
            "{}{}{}",
            fill.to_string().repeat(left_fill),
            title,
            fill.to_string().repeat(right_fill)
        )
    }
}

/// La version labellisée qui réutilise le moteur
pub fn render_labeled_variable_typed_list(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    items: &TypedList<VariableId, TypeId>,
    ctx: &RenderContext,
) -> fmt::Result {
    write!(f, "  {:<12} : ", label)?;

    if items.is_empty() {
        writeln!(f, "<None>")?;
    } else {
        // On appelle le moteur avec une virgule pour la lisibilité en liste
        typed_list::render_variable_typed_list(f, items.as_slice(), ctx)?;
        writeln!(f)?;
    }
    Ok(())
}

/// Rendu d'une expression étiquetée.
/// Au lieu de formater l'expression en string (lent), on appelle le renderer d'arbre.
pub fn render_labeled_expr(
    f: &mut fmt::Formatter<'_>,
    label: &str,
    expr_id: ExprId,
    ctx: &RenderContext,
) -> fmt::Result {
    writeln!(f, "  {:<12} :", label)?;

    // On vérifie si l'ID est celui par défaut (vide)
    if expr_id == ExprId::default() {
        writeln!(f, "    <None>")
    } else {
        // On délègue au moteur de rendu d'arbre (dans expression.rs)
        // avec une indentation de base.
        debug::expr::render(f, expr_id, ctx)
    }
}
