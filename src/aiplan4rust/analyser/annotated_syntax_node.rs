use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNode;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::parser::Span;

use crate::aiplan4rust::analyser::AnnotatedSyntaxTree;
use serde::Deserialize;
use serde::Serialize;
use std::fmt;

/// A node in the annotated syntax tree with additional metadata.
///
/// `AnnotatedSyntaxNode` represents a node in a parsed syntax tree, enriched
/// with its syntactic kind, source span, and references to its child nodes.
/// This structure is used to facilitate semantic analysis and requirement
/// checking after parsing.
///
/// # Fields
///
/// * `kind` - The specific syntactic category of the node (e.g., action, predicate, etc.).
/// * `span` - The position of the node in the original source input, used for error reporting.
/// * `children` - Indices of this node’s children in the annotated syntax tree.
///
/// This struct derives common traits such as `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`,
/// `Serialize`, and `Deserialize` for convenience in debugging, storage, and comparison.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnnotatedSyntaxNode {
    kind: SyntaxNodeKind,
    span: Span,
    children: Vec<usize>,
}

impl AnnotatedSyntaxNode {
    /// Creates a new `AnnotatedSyntaxNode`.
    ///
    /// This constructor initializes an `AnnotatedSyntaxNode` with the given kind,
    /// source span, and list of child indices.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of the syntax node (e.g., a specific grammar construct).
    /// * `span` - The span in the source input that this node covers.
    /// * `children` - A vector of indices referring to this node's children in the
    ///                annotated syntax tree.
    ///
    /// # Returns
    ///
    /// A new `AnnotatedSyntaxNode` instance with the provided data.
    ///
    /// # Example
    ///
    /// ```rust
    /// let node = AnnotatedSyntaxNode::new(kind, span, vec![1, 2, 3]);
    /// ```
    pub fn new(kind: SyntaxNodeKind, span: Span, children: Vec<usize>) -> Self {
        AnnotatedSyntaxNode {
            kind,
            span,
            children,
        }
    }

    /// Constructs an `AnnotatedSyntaxNode` from a `SyntaxNode` and an index table.
    ///
    /// This function traverses the children of the given `SyntaxNode`, retrieves
    /// their corresponding indices from the provided `index_table`, and uses them to
    /// build a new `AnnotatedSyntaxNode`.
    ///
    /// # Arguments
    ///
    /// * `node` - A reference to the `SyntaxNode` to be annotated.
    /// * `index_table` - A map that associates each child `SyntaxNode` with a unique index,
    ///                   typically corresponding to its position in a syntax tree structure.
    ///
    /// # Returns
    ///
    /// Returns `Ok(AnnotatedSyntaxNode)` if all child nodes are found in the `index_table`.
    /// Otherwise, returns a `ParserInternalError` if a child node is missing from the table.
    ///
    /// # Errors
    ///
    /// Returns an error if one of the child nodes of `node` is not found in the `index_table`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let annotated_node = AnnotatedSyntaxNode::from(&syntax_node, &index_table)?;
    /// ```
    pub fn from(node: &SyntaxNode) -> Result<AnnotatedSyntaxNode, ParserInternalError> {
        let children = node
            .children()
            .iter()
            .map(|child| child.id())
            .copied()
            .collect();

        Ok(AnnotatedSyntaxNode::new(
            node.kind().clone(),
            node.span().clone(),
            children,
        ))
    }

    /// Returns a reference to the kind of this syntax node.
    ///
    /// The kind indicates the syntactic role of the node, such as a predicate, type definition,
    /// function, or logical operator.
    pub fn kind(&self) -> &SyntaxNodeKind {
        &self.kind
    }

    /// Returns a reference to the list of indices of this node's children.
    ///
    /// The indices refer to positions in the `AnnotatedSyntaxTree` where the child nodes are stored.
    pub fn children(&self) -> &Vec<usize> {
        &self.children
    }

    /// Returns a reference to the span of this syntax node in the source input.
    ///
    /// The span captures the start and end positions of the node, useful for diagnostics and error
    /// messages.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns `true` if this syntax node has any children.
    ///
    /// Useful to distinguish between leaf and non-leaf nodes in the syntax tree.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Attempts to extract a symbol associated with this node.
    ///
    /// This method returns:
    /// - `Ok(Some(symbol))` if a symbol is found associated with the node.
    /// - `Ok(None)` if the node does not have an associated symbol (this is not an error).
    /// - `Err(ParserInternalError)` if the node is malformed (e.g., a `FunctionTerm` without children, or a non-existent child).
    ///
    /// This function checks if the node's kind directly contains a symbol. If not, it handles compound node types
    /// like `FunctionTerm` or `AtomicFormula`, which derive their symbol from their first child node.
    /// If the first child is not present or does not contain a valid symbol, an error is returned.
    ///
    /// # Parameters
    /// - `syntax_tree`: The syntax tree to look up the child node from, if needed.
    ///
    /// # Returns
    /// - `Result<Option<String>, ParserInternalError>`: The result is either an `Option<String>` containing
    ///   the symbol (if found), or an error indicating why no symbol could be retrieved.
    pub fn get_symbol<'a>(
        &'a self,
        syntax_tree: &'a AnnotatedSyntaxTree,
    ) -> Result<Option<&'a String>, ParserInternalError> {
        // Direct case: the kind of the node contains a recognizable symbol
        if let Some(sym) = self.kind.get_symbol() {
            return Ok(Some(sym));
        }

        // For compound nodes like FunctionTerm or AtomicFormula, the symbol is derived from the first child
        match &self.kind {
            SyntaxNodeKind::FunctionTerm | SyntaxNodeKind::AtomicFormula => {
                let child_index = self.children.first().ok_or_else(|| {
                    ParserInternalError::new(
                        "No children found for 'FunctionTerm' or 'AtomicFormula'.".to_string(),
                    )
                })?;

                let child = syntax_tree.get_entry(*child_index).ok_or_else(|| {
                    ParserInternalError::new(format!(
                        "Child index {} not found in syntax tree.",
                        child_index
                    ))
                })?;

                Ok(child.kind.get_symbol())
            }

            // Default case: no symbol, but it's not an error
            _ => Ok(None),
        }
    }
}

impl fmt::Display for AnnotatedSyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AstEntry {{ kind: {:?}, span: {}, children: {:?} }}",
            self.kind, self.span, self.children
        )
    }
}
