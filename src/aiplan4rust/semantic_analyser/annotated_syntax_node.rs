use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNode;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::parser::Span;

use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
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
    pub fn from(
        node: &SyntaxNode,
        index_table: &HashMap<&SyntaxNode, usize>,
    ) -> Result<AnnotatedSyntaxNode, ParserInternalError> {
        let mut children = Vec::new();

        for child in node.children() {
            match index_table.get(child.as_ref()) {
                Some(&index) => children.push(index),
                None => {
                    return Err(ParserInternalError::new(format!(
                        "Node not found in index table: {:?}",
                        child
                    )))
                }
            }
        }

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

    /// Retrieves the key for the given AST node based on its type.
    /// This function is used to extract the key for symbols used in the symbol table.
    /// If the node is a constant, variable, or one of the predefined symbols, the key is the
    /// symbol's name. For `FunctionTerm` and `AtomicFormula`, it returns a key formatted as
    /// `name/arity` based on their first child.
    ///
    /// The key is used to store and look up symbols in the symbol table. This function ensures that
    /// each symbol has a unique identifier based on its structure, which is useful for semantics
    /// analysis and symbol resolution.
    ///
    /// # Returns
    /// - Ok(String): The key derived from the node.
    /// - Err(ParserInternalError): An error if the node cannot be processed or if it does not meet
    ///   the expected structure.
    ///
    /// # Errors
    /// - If the node has no children or its first child is not a `FunctionSymbol` or
    ///   `PredicateSymbol`.
    /// - If the node is of an unexpected kind.
    pub fn get_key(&self, ast_table: &AnnotatedSyntaxTree) -> Result<String, ParserInternalError> {
        match &self.kind {
            // For symbols like constants, variables, action symbols, etc., return the symbol's
            // name directly.
            SyntaxNodeKind::Constant(name)
            | SyntaxNodeKind::Variable(name)
            | SyntaxNodeKind::PrimitiveType(name)
            | SyntaxNodeKind::DomainName(name)
            | SyntaxNodeKind::ProblemName(name)
            | SyntaxNodeKind::ActionSymbol(name)
            | SyntaxNodeKind::DASymbol(name)
            | SyntaxNodeKind::PrefName(name) => Ok(name.to_string()),
            // For `FunctionTerm` and `AtomicFormula`, derive the key from their first child
            SyntaxNodeKind::FunctionTerm | SyntaxNodeKind::AtomicFormula => {
                // Check if the node has children
                if let Some(child_index) = self.children.first() {
                    let child = ast_table.get_entry(*child_index).unwrap();
                    // Check the type of the first child (it should be either a FunctionSymbol or
                    // PredicateSymbol)
                    match &child.kind {
                        SyntaxNodeKind::FunctionSymbol(name) => Ok(name.to_string()),
                        SyntaxNodeKind::Predicate(name) => Ok(name.to_string()),
                        _ => {
                            // If the first child is neither a FunctionSymbol nor a PredicateSymbol, return an error
                            Err(ParserInternalError::new(
                                format!(
                                    "First child must be a 'FunctionSymbol' or 'PredicateSymbol', but found: {:?}.",
                                    child.kind
                                )
                            ))
                        }
                    }
                } else {
                    // If there are no children, return an error
                    Err(ParserInternalError::new(
                        "No children found for 'FunctionTerm' or 'AtomicFormula'.".to_string(),
                    ))
                }
            }
            // Handle unexpected AST node kinds
            _ => {
                // Return an error if the AST node kind is not recognized
                Err(ParserInternalError::new(format!(
                    "Unexpected AST kind: {:?}",
                    self.kind
                )))
            }
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
