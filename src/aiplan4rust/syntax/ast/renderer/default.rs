//! Rendering utilities for AST nodes in the `aiplan4rust` crate.
//!
//! This module provides helper functions to format and renderers AST nodes,
//! mainly for debugging or pretty-printing purposes.
//!
//! # Provided Functions
//!
//! - [`render`]: Formats a single [`AstNode`] into a human-readable string,
//!   including its kind, content, span information, parent, and children.
//!
//! # Example
//!
//! ```rust
//! use std::fmt::Write;
//! use aiplan4rust::syntax::ast::AstNode;
//! use aiplan4rust::syntax::renderer::renderers;
//!
//! // Assuming `node` is an AstNode instance
//! let mut output = String::new();
//! let _ = renderers(&node, &mut output);
//! println!("{}", output);
//! ```

use std::fmt;
use crate::aiplan4rust::syntax::ast::AstNode;

/// Renders an [`AstNode`] into a human-readable format.
///
/// This function writes a detailed string representation of the node to the
/// provided formatter. The output includes:
/// - The node kind (syntax classification).
/// - The node content (payload).
/// - The source code span covering this node (line and column ranges).
/// - The parent node ID (or `"none"` if root).
/// - A list of child node IDs.
///
/// # Arguments
///
/// * `node` - Reference to the AST node to be rendered.
/// * `f` - Mutable formatter to write the output string.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating success or formatting error.
///
/// # Example
///
/// ```rust
/// # use std::fmt;
/// # use aiplan4rust::syntax::ast::AstNode;
/// # use aiplan4rust::syntax::renderer::render;
/// # fn example(node: &AstNode) -> fmt::Result {
/// let mut output = String::new();
/// let mut formatter = fmt::Formatter::new(&mut output);
/// render(node, &mut formatter)?;
/// println!("{}", output);
/// # Ok(())
/// # }
/// ```
pub fn render(
    node: &AstNode,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
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

    // Retrieve the span of the node, then format it as
    // "[l<begin_line>:c<begin_column>-l<end_line>:c<end_column>]"
    let span = node.span();
    let span_str = format!(
        "[l{}:c{}-l{}:c{}]",
        span.begin_line(),
        span.begin_column(),
        span.end_line(),
        span.end_column()
    );

    // Write the formatted output showing:
    // - kind of the node
    // - content of the node
    // - span string showing source code location
    // - parent node id or "none"
    // - list of children node ids
    write!(
        f,
        "[kind={}, content={}, span={}, parent={}, children=[{}]]",
        node.kind(),
        node.content(),
        span_str,
        parent,
        children,
    )
}
