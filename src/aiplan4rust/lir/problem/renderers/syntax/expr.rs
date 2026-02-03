use std::fmt;
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::renderers::render_context::RenderContext;
use crate::aiplan4rust::lir::problem::renderers::syntax::expr_content;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    expr: &Expr,
    ctx: &RenderContext,
) -> std::fmt::Result  {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return write!(f, "()"),
    };

    // On suit la profondeur pour savoir combien de parenthèses fermer
    let mut last_depth = 0;

    // On utilise le preorder iterator
    for (_id, depth, _is_last, node) in expr.preorder_from(root_id) {

        // 1. Si on remonte dans l'arbre, on ferme les scopes précédents
        if depth < last_depth {
            for _ in 0..(last_depth - depth) {
                write!(f, ")")?;
            }
        }

        // 2. Gestion de l'espacement (entre frères ou après une parenthèse ouvrante)
        if depth > 0 {
            write!(f, " ")?;
        }

        // 3. On ouvre le scope du nœud actuel
        write!(f, "(")?;

        // 4. Rendu du contenu via le contexte (sans les étiquettes de debug)
        let content = node.content();
        if !matches!(content, Content::None) {
            expr_content::render(f, content, ctx)?;
        } else {
            // Si pas de contenu, on affiche le Kind (ex: and, or, not)
            write!(f, "{:?}", node.kind())?;
        }

        last_depth = depth;
    }

    // 5. On ferme toutes les parenthèses restantes à la fin
    for _ in 0..=last_depth {
        write!(f, ")")?;
    }

    Ok(())
}
