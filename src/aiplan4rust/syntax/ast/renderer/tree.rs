use std::fmt::{self, Formatter};
use crate::aiplan4rust::arena::Arena;
use crate::aiplan4rust::syntax::ast::{AstNode, AstContent};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};

pub fn render(
    node: &AstNode,
    f: &mut Formatter<'_>,
    arena: &Arena<AstNode>,
    interner: &StringInterner,
) -> fmt::Result {
    fn fmt_node(
        node: &AstNode,
        f: &mut Formatter<'_>,
        arena: &Arena<AstNode>,
        interner: &StringInterner,
        prefix: &str,
        last: bool,
    ) -> fmt::Result {
        let branch = if last { "└─" } else { "├─" };

        let content_str = match node.content() {
            AstContent::None => String::new(),
            AstContent::Ident(id) => format!(" [{}]", id.to_string_with_interner(interner)),
            other => format!(" [{}]", other),
        };

        let (line, column) = node.span().start_position();
        let span_str = format!(" (l{}:c{})", line, column);

        let children = node.children();
        let len = children.len();

        write!(
            f,
            "{}{}{}{}{}",
            prefix,
            branch,
            node.kind(),
            content_str,
            span_str
        )?;

        if !children.is_empty() {
            writeln!(f)?;
        }

        let new_prefix = if last {
            format!("{}   ", prefix)
        } else {
            format!("{}│  ", prefix)
        };

        for (i, child_idx) in children.iter().enumerate() {
            let child = arena
                .get_node(*child_idx)
                .expect("Child not found in arena");
            fmt_node(child, f, arena, interner, &new_prefix, i == len - 1)?;

            if i < len - 1 {
                writeln!(f)?;
            }
        }

        Ok(())
    }

    fmt_node(node, f, arena, interner, "", true)
}
