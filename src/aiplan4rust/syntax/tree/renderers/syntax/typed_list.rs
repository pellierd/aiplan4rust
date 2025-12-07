//! This module provides a function to renderers an Abstract Syntax Tree (AST) node
//! into a formatted string output, suitable for displaying syntax domain
//! definitions or problem descriptions in a pretty-printed manner.
//!
//! The `renderers` function recursively processes an AST node and its children,
//! formatting them either inline or with indentation and line breaks according
//! to the `multiline` flag. It leverages an arena allocator to access nodes
//! efficiently and uses an interner to resolve symbol names.
//!
//! # Overview
//! - `node`: the AST node to renderers
//! - `f`: the formatter to write output to (usually a string buffer)
//! - `arena`: the arena storing all AST nodes
//! - `interner`: a string interner for resolving symbols to names
//! - `multiline`: if true, output will include indentation and newlines for readability
//! - `indent`: current indentation level (number of indent steps)

use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::syntax;
use crate::aiplan4rust::syntax::tree::renderers;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxTree};

/// Recursively renders the given AST node and its children into the formatter.
///
/// Each child node is rendered either inline (with spaces) or multiline (with
/// indentation and line breaks) depending on the `multiline` flag.
///
/// If a child node cannot be found in the arena, `<invalid_node>` is printed instead.
///
/// # Parameters
///
/// - `node`: The AST node to renderers.
/// - `f`: The formatter where the output is written.
/// - `arena`: The arena allocator storing the AST nodes.
/// - `interner`: String interner used for symbol resolution.
/// - `multiline`: When true, format output with indentation and new lines.
/// - `indent`: Current indentation level (used only if `multiline` is true).
///
/// # Errors
///
/// Returns a formatting error if writing to the formatter fails.
pub fn render<T: SyntaxNode>(
    node: &T,
    f: &mut Formatter<'_>,
    arena: &SyntaxTree<T>,
    interner: &StringInterner,
    multiline: bool,
    indent: usize,
) -> fmt::Result {
    // Iterate over all children of the current node
    for (i, child_id) in node.children().iter().enumerate() {
        // If this is not the first child and we're not in multiline mode,
        // write a space to separate elements on the same line
        if i > 0 && !multiline {
            write!(f, " ")?;
        }

        // Attempt to get the child node from the arena
        if let Some(child_node) = arena.get_node(*child_id) {
            // If multiline is enabled, write indentation before the child
            if multiline {
                syntax::display::write_indent(f, indent)?;
            }
            // Recursively renderers the child node
            renderers::syntax::render(child_node, f, arena, interner)?;
            // Add a newline after the child if multiline is enabled
            if multiline {
                writeln!(f)?;
            }
        } else {
            // If the child node is invalid, write indentation if multiline is enabled
            if multiline {
                syntax::display::write_indent(f, indent)?;
            }
            // Print a placeholder for invalid node
            write!(f, "<invalid_node>")?;
            // Add a newline if multiline is enabled
            if multiline {
                writeln!(f)?;
            }
        }
    }

    // Return Ok if everything went well
    Ok(())
}
