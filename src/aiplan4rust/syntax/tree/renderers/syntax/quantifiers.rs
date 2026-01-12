use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::SyntaxInternerDisplay;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxTree};

pub fn render<T: SyntaxNode>(
    node: &T,
    f: &mut Formatter<'_>,
    arena: &SyntaxTree<T>,
    interner: &StringInterner,
    indent: usize, // pas de multiline ici
) -> fmt::Result {
    let indent_str = "    ".repeat(indent);
    let children = node.children();

    // 1. (forall / exists + début ligne)
    write!(f, "{}(", indent_str)?;
    node.render_kind().fmt_syntax_with_interner(f, interner)?;
    write!(f, " ")?;

    let body_index;
    if children.len() == 1 {
        let content = node.content();
        write!(f, "({})", content.to_syntax_string_with_interner(interner))?;
        body_index = 0;
    } else {
        // 2. Variables quantifiées (sur la même ligne)
        if let Some(&vars_id) = children.get(0) {
            if let Some(vars_node) = arena.get_node(vars_id) {
                write!(f, "(")?;
                vars_node.fmt_syntax(f, arena, interner)?;
                write!(f, ")")?;
            } else {
                write!(f, "<invalid-variables>")?;
            }
        } else {
            write!(f, "<missing-variables>")?;
        }
        body_index = 1;
    }

    // 3. Expression (indentée d’un cran)
    if let Some(&expr_id) = children.get(body_index) {
        if let Some(expr_node) = arena.get_node(expr_id) {
            write!(f, " ")?;
            expr_node.fmt_syntax_with_indent(f, arena, interner, indent + 1)?;
        } else {
            writeln!(f, "{}<invalid-expression>", indent_str)?;
        }
    } else {
        writeln!(f, "{}<missing-expression>", indent_str)?;
    }

    write!(f, "{})", indent_str)?; // fermeture
    Ok(())
}
