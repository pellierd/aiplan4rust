//! Pretty-printing utilities for AST nodes with tree structure visualization.
//!
//! This module provides functions to recursively renderers an [`AstNode`] and its
//! children as a visually indented tree, showing node kinds, content, and source
//! span information.
//!
//! The rendering uses ASCII tree branches (├─, └─, │) to denote hierarchy and
//! sibling relationships for easy human reading.
//!
//! # Example
//!
//! ```rust
//! use std::fmt::Write;
//! use crate::aiplan4rust::syntax::ast::AstNode;
//! use crate::aiplan4rust::syntax::tree::SyntaxTree;
//! use crate::aiplan4rust::interner::StringInterner;
//! use crate::aiplan4rust::syntax::renderer::renderers;
//!
//! // Assume `node`, `arena`, and `interner` are available
//! let mut output = String::new();
//! let mut formatter = std::fmt::Formatter::new(&mut output);
//! renderers(&node, &mut formatter, &arena, &interner).unwrap();
//! println!("{}", output);
//! ```

use crate::aiplan4rust::core::interner::{InternerDisplay, SymbolInterner};
use crate::aiplan4rust::syntax::ast::tree::Tree;
use crate::aiplan4rust::syntax::ast::{AstContent, AstNode};
use std::fmt::{self, Formatter};

/// Recursively renders an [`AstNode`] and its subtree as a visually indented tree.
///
/// This function prints the node kind, optionally its content (with interner
/// support for identifiers), and the node's starting position span.
///
/// Child nodes are displayed with appropriate ASCII branch characters indicating
/// hierarchy and sibling relationships.
///
/// # Arguments
///
/// * `node` - The root AST node to renderers.
/// * `f` - The formatter to write the output to.
/// * `syntax_tree` - The syntax tree containing the node and its children.
/// * `interner` - The string interner used to resolve interned identifiers.
///
/// # Returns
///
/// Returns a [`fmt::Result`] indicating success or failure in formatting.
///
/// # Internal Details
///
/// The rendering is implemented via the inner helper function `fmt_node`, which
/// handles recursive traversal and printing with proper indentation and branch
/// drawing.
///
/// # Example
///
/// ```rust
/// # use aiplan4rust::syntax::ast::AstNode;
/// # use aiplan4rust::syntax::tree::SyntaxTree;
/// # use aiplan4rust::interner::StringInterner;
/// # use std::fmt::Formatter;
/// # fn example(node: &AstNode, syntax_tree: &SyntaxTree<AstNode>, interner: &StringInterner, f: &mut Formatter<'_>) -> std::fmt::Result {
/// renderers(node, f, arena, interner)
/// # }
/// ```
pub fn render(
    node: &AstNode,
    f: &mut Formatter<'_>,
    syntax_tree: &Tree<AstNode>,
    interner: &SymbolInterner,
) -> fmt::Result {
    /// Internal helper to recursively format a node and its children.
    ///
    /// # Arguments
    ///
    /// * `node` - The AST node to format.
    /// * `f` - Formatter to write output.
    /// * `syntax_tree` - Syntax tree containing the node.
    /// * `interner` - String interner to resolve identifiers.
    /// * `prefix` - String prefix for indentation and branch drawing.
    /// * `last` - Whether this node is the last child of its parent, used for branch style.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating formatting success or failure.
    fn fmt_node(
        node: &AstNode,
        f: &mut Formatter<'_>,
        syntax_tree: &Tree<AstNode>,
        interner: &SymbolInterner,
        prefix: &str,
        last: bool,
    ) -> fmt::Result {
        // Choose the branch character: "└─" if this node is the last child, otherwise "├─"
        let branch = if last { "└─" } else { "├─" };

        // Format the node content: if it's an Ident, display it using the interner; otherwise, show other content
        let content_str = match node.content() {
            AstContent::None => String::new(),
            AstContent::Ident(id) => format!(" [{}]", id.to_string_with_interner(interner)),
            other => format!(" [{}]", other),
        };

        // Get the starting line and column from the node’s span for display
        let (line, column) = node.span().start_position();
        let span_str = format!(" (l{}:c{})", line, column);

        // Get the list of children nodes and count them
        let children = node.children();
        let len = children.len();

        // Write the current node line: prefix indentation + branch + kind + content + position info
        write!(
            f,
            "{}{}{}{}{}",
            prefix,      // Indentation prefix passed from recursive calls
            branch,      // Branch character (└─ or ├─)
            node.kind(), // AST node kind
            content_str, // Formatted content string
            span_str     // Source code position span
        )?;

        // If the node has children, write a newline before printing them
        if !children.is_empty() {
            writeln!(f)?;
        }

        // Update the prefix for children:
        // if this is the last child, add spaces; else add a vertical bar '│' for indentation
        let new_prefix = if last {
            format!("{}   ", prefix)
        } else {
            format!("{}│  ", prefix)
        };

        // Recursively format each child node
        for (i, child_idx) in children.iter().enumerate() {
            // Get the child node from the syntax tree by index
            let child = syntax_tree
                .get_node(*child_idx)
                .expect("Child not found in arena");

            // Recursive call to format the child, update prefix and indicate if last child
            fmt_node(child, f, syntax_tree, interner, &new_prefix, i == len - 1)?;

            // Add a blank line between children except after the last one
            if i < len - 1 {
                writeln!(f)?;
            }
        }

        Ok(())
    }
    fmt_node(node, f, syntax_tree, interner, "", true)
}
