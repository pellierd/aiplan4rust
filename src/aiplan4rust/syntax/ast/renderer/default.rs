use std::fmt;
use crate::aiplan4rust::syntax::ast::AstNode;

pub fn render(
    node: &AstNode,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let children = node
        .children()
        .iter()
        .map(|idx| idx.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let parent = node
        .parent()
        .map_or("none".to_string(), |idx| idx.to_string());

    let span = node.span();
    let span_str = format!(
        "[l{}:c{}-l{}:c{}]",
        span.begin_line(),
        span.begin_column(),
        span.end_line(),
        span.end_column()
    );

    write!(
        f,
        "[kind={}, content={}, span={} parent={}, children=[{}]]",
        node.kind(),
        node.content(),
        span_str,
        parent,
        children,
    )
}
