//! Serialization utilities for the Abstract Syntax Tree (AST).
//!
//! This module defines flat, serializable representations of `Ast` and `AstNode`
//! structures, enabling lossless serialization and deserialization of ASTs
//! (e.g., to/from JSON).
//!
//! It introduces two main types:
//!
//! - [`SerializableAst`] — a serializable version of the full `Ast`.
//! - [`SerializableNode`] — a serializable version of a single `AstNode`.
//!
//! # Design Considerations
//!
//! - **Serialization clones the AST:** The `From<&Ast>` and `From<&AstNode>`
//!   implementations perform deep cloning of the AST. This ensures that you
//!   can serialize an immutable reference without consuming the original AST.
//!
//! - **Deserialization avoids cloning:** When deserializing, the data is directly
//!   converted into owned `Ast` and `AstNode` instances without intermediate clones.
//!
//! This design makes serialization ergonomic and deserialization efficient.

use serde::{Serialize, Deserialize};
use crate::aiplan4rust::syntax::ast::{Ast, AstNode, AstKind};
use crate::aiplan4rust::syntax::Span;

/// A flat, serializable representation of a single AST node (`AstNode`).
///
/// This type is designed for recursive serialization of AST trees. Each node
/// stores its unique ID, kind, span, and child nodes.
///
/// See [`From<&AstNode>`] for how conversion is handled.
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableNode {
    /// Unique identifier for the node.
    pub id: usize,

    /// The kind of AST node (e.g., identifier, literal, expression, etc.).
    pub kind: AstKind,

    /// Span information (e.g., source position) for the node.
    pub span: Span,

    /// Child nodes of this AST node.
    pub children: Vec<SerializableNode>,
}

/// A serializable representation of the entire AST (`Ast`).
///
/// This includes the root node, the source name from which the AST was parsed,
/// and the generation timestamp.
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableAst {
    /// The root node of the AST.
    pub root: SerializableNode,

    /// Name of the original source file (if any).
    pub source_name: String,

    /// Time of AST generation, as a UNIX timestamp in seconds.
    pub generated_at: u64,
}

// === Conversion: &Ast -> SerializableAst ===

/// Creates a serializable version of an `Ast` from an immutable reference.
///
/// This implementation clones all AST contents. This allows you to serialize
/// an AST without consuming it.
impl From<&Ast> for SerializableAst {
    fn from(ast: &Ast) -> Self {
        SerializableAst {
            root: SerializableNode::from(ast.root().as_ref()),
            source_name: ast.source_name().clone(),
            generated_at: ast
                .generated_at()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

/// Creates a `SerializableNode` from an `&AstNode`.
///
/// This clones the node's contents recursively (kind, span, children).
impl From<&AstNode> for SerializableNode {
    fn from(node: &AstNode) -> Self {
        SerializableNode {
            id: *node.id(),
            kind: node.kind().clone(),
            span: node.span().clone(),
            children: node
                .children()
                .iter()
                .map(|child| SerializableNode::from(child.as_ref()))
                .collect(),
        }
    }
}

// === Back-conversion: SerializableAst -> Ast ===

impl SerializableAst {
    /// Converts this serializable AST back into a full `Ast` structure.
    ///
    /// This is the inverse of the `From<&Ast>` conversion. It does not clone
    /// or share memory with any other structure.
    pub fn into_ast(self) -> Ast {
        Ast::new(
            Box::new(self.root.into_ast_node()),
            self.source_name,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.generated_at),
        )
    }
}

impl SerializableNode {
    /// Converts this serializable node into a full `AstNode`.
    ///
    /// This is the inverse of the `From<&AstNode>` conversion.
    /// It recursively reconstructs the AST hierarchy.
    pub fn into_ast_node(self) -> AstNode {
        let children = self.children
            .into_iter()
            .map(|child| Box::new(child.into_ast_node()))
            .collect();

        let mut node = AstNode::new_with_span(self.kind, children, self.span);
        node.set_id(self.id);
        node
    }
}
