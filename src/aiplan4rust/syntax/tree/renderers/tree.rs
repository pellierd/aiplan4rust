
use std::fmt::{self, Formatter};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::syntax::tree::{SyntaxContent, SyntaxNode, SyntaxTree};


pub fn render<T: SyntaxNode>(
    node: &T,
    f: &mut Formatter<'_>,
    syntax_tree: &SyntaxTree<T>,
    interner: &StringInterner,
) -> fmt::Result {

    fn fmt_node<T: SyntaxNode>(
        node: &T,
        f: &mut Formatter<'_>,
        syntax_tree: &SyntaxTree<T>,
        interner: &StringInterner,
        prefix: &str,
        last: bool,
    ) -> fmt::Result {
        // Choose the branch character: "└─" if this node is the last child, otherwise "├─"
        let branch = if last { "└─" } else { "├─" };

        // Format the node content: if it's an Ident, display it using the interner; otherwise, show other content
        let content_str = if node.content().is_none() {
            String::new()
        } else {
            format!(" [{}]", node.content().to_string_with_interner(interner))
        };


        // Get the list of children nodes and count them
        let children = node.children();
        let len = children.len();

        // Write the current node line: prefix indentation + branch + kind + content + position info
        write!(
            f,
            "{}{}{}{}",
            prefix,       // Indentation prefix passed from recursive calls
            branch,       // Branch character (└─ or ├─)
            node.kind(),  // AST node kind
            content_str,  // Formatted content string
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
