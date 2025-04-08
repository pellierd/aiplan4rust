use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxNodeKind};
use crate::aiplan4rust::syntax::Span;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeapSyntaxNode {
    kind: SyntaxNodeKind,
    span: Span,
    children: Vec<usize>,
}

impl HeapSyntaxNode {
    pub fn new(kind: SyntaxNodeKind, span: Span, children: Vec<usize>) -> Self {
        HeapSyntaxNode {
            kind,
            span,
            children,
        }
    }

    pub fn from(
        ast: &SyntaxNode,
        index_table: &HashMap<&SyntaxNode, usize>,
    ) -> Result<HeapSyntaxNode, ParserInternalError> {
        let mut children = Vec::new();

        for child in ast.children() {
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

        Ok(HeapSyntaxNode::new(
            ast.kind().clone(),
            ast.span().clone(),
            children,
        ))
    }

    // Accesseur pour obtenir le `kind` d'un noeud
    pub fn kind(&self) -> &SyntaxNodeKind {
        &self.kind
    }

    // Accesseur pour obtenir les enfants du noeud
    pub fn children(&self) -> &Vec<usize> {
        &self.children
    }

    // Accesseur pour obtenir la portée (span) du noeud
    pub fn span(&self) -> &Span {
        &self.span
    }

    // Méthode pour vérifier si un noeud a des enfants
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
    pub fn get_key(&self, ast_table: &HeapSyntaxTree) -> Result<String, ParserInternalError> {
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

impl fmt::Display for HeapSyntaxNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AstEntry {{ kind: {:?}, span: {}, children: {:?} }}",
            self.kind, self.span, self.children
        )
    }
}
