use std::fmt;
use crate::aiplan4rust::syntax::tree::SyntaxNode;

pub fn render<T>(node: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: SyntaxNode,
    T::Content: fmt::Display,
{
    // Collect the children NodeIds of the current node and convert each to string,
    // then join them with commas to form a single string representation.
    let children = node
        .children()
        .iter()
        .map(|idx| idx.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    // Get the parent NodeId as a string if it exists, otherwise use "none".
    let parent = node
        .parent()
        .map_or("none".to_string(), |idx| idx.to_string());

    // Write the formatted output showing:
    // - kind of the node
    // - content of the node
    // - span string showing source code location
    // - parent node id or "none"
    // - list of children node ids
    write!(
        f,
        "[kind={}, content={}, parent={}, children=[{}]]",
        node.kind(),
        node.content(),
        parent,
        children,
    )
}
