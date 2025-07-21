//! Module for rendering `task` AST nodes.
//!
//! This function provides a formatted display of the syntax tree for a `task` node,
//! recursively traversing its children and respecting configurable indentation.
//!
//! # Main function
//!
//! [`render`] prints a `task` node in a readable representation, optionally showing
//! a prefix indicating the node type and indentation reflecting the hierarchy.
//!
//! # Arguments
//!
//! - `node`: The AST `task` node to render.
//! - `f`: The formatter to write the output to (typically a `fmt::Formatter`).
//! - `arena`: The arena containing all AST nodes, used to access children nodes.
//! - `interner`: A string interner used to resolve node identifiers.
//! - `with_prefix`: Boolean flag to indicate if a prefix (`":task "`) should be printed.
//! - `indent`: Current indentation level (each level corresponds to 2 spaces).
//!
//! # Output format
//!
//! - If `with_prefix` is `true`, the output begins with the indentation followed by `":task "`.
//! - Children are printed inside parentheses, separated by spaces.
//! - If the node has no children, `()` is printed.
//! - Each child is rendered with the method `fmt_planning_syntax_with_indent` at the same indentation level.
//! - If a child is invalid (missing in the arena), `"<invalid>"` is printed instead.
//! - If `with_prefix` is true, a newline is appended at the end.
//!
//! # Example usage
//!
//! ```rust,no_run
//! use std::fmt::Formatter;
//! use crate::aiplan4rust::arena::Arena;
//! use crate::aiplan4rust::interner::StringInterner;
//! use crate::aiplan4rust::syntax::ast::AstNode;
//! use your_crate::renderer::syntax::task::render;
//!
//! // Example display function using the render function
//! fn display_task(node: &AstNode, f: &mut Formatter<'_>, arena: &Arena<AstNode>, interner: &StringInterner) -> std::fmt::Result {
//!     render(node, f, arena, interner, true, 0)
//! }
//! ```

use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax::ast::{renderer, AstNode};
use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::syntax;
use crate::aiplan4rust::syntax::tree::SyntaxTree;

/// Renders a `task` AST node in an indented, syntax-like format with optional prefix.
///
/// # Parameters
/// - `node`: Reference to the `task` AST node to render.
/// - `f`: Target formatter to write the output.
/// - `arena`: Arena holding all AST nodes, used to access children.
/// - `interner`: String interner for resolving identifiers.
/// - `with_prefix`: Whether to print a prefix `":task "` at the start of the line.
/// - `indent`: Current indentation level (each level equals 2 spaces).
///
/// # Returns
/// Formatting result (`fmt::Result`).
pub fn render(
    node: &AstNode,
    f: &mut Formatter<'_>,
    arena: &SyntaxTree<AstNode>,
    interner: &StringInterner,
    with_prefix: bool,
    indent: usize,
) -> fmt::Result {

    // Write prefix if requested
    if with_prefix {
        syntax::display::write_indent(f, indent)?;
        write!(f, ":task ")?;
    }

    let children = node.children();

    if children.is_empty() {
        // No children → print empty parentheses
        write!(f, "()")?;
    } else {
        // Otherwise, print opening parenthesis
        write!(f, "(")?;

        // Iterate over children separated by spaces
        for (i, child_id) in children.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }

            // Render each child recursively with current indentation
            if let Some(child_node) = arena.get_node(*child_id) {
                renderer::syntax::render_with_indent(child_node, f, arena, interner, indent)?;
            } else {
                // Invalid child → print placeholder
                write!(f, "<invalid>")?;
            }
        }

        // Close the parenthesis
        write!(f, ")")?;
    }

    // Add newline if prefix was printed, else return Ok
    if with_prefix {
        writeln!(f)
    } else {
        Ok(())
    }
}
