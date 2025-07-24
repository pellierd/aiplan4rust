use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind, ExprNode};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::lang::Optimization;
use crate::aiplan4rust::syntax::tree::{SyntaxSubtree, SyntaxTree};

/// Wrapper struct around `TreeArena<ExprNode>` representing an expression arena.
///
/// This struct provides a dedicated type for expressions, allowing
/// to add custom methods on top of the underlying `TreeArena`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Expr {
    tree: SyntaxTree<ExprNode>,
}

impl Expr {
    /// Creates a new, empty expression arena.
    ///
    /// # Examples
    ///
    /// ```
    /// let expr = Expr::new();
    /// assert!(expr.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            tree: SyntaxTree::<ExprNode>::new(),
        }
    }

    pub fn empty_or() -> Self {
        let mut empty_or = Expr::new();
        let root = ExprNode::new(ExprKind::Or, ExprContent::None, None);
        empty_or.alloc(root);
        empty_or
    }

    pub fn empty_and() -> Self {
        let mut empty_and = Expr::new();
        let root = ExprNode::new(ExprKind::And, ExprContent::None, None);
        empty_and.alloc(root);
        empty_and
    }

    pub fn metric_none() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Metric,
            ExprContent::Optimization(Optimization::None),
            None,
        );
        expr.alloc(root);
        expr
    }

    pub fn empty_length_spec() -> Self {
        let mut expr = Expr::new();
        let root = ExprNode::new(
            ExprKind::Length,
            ExprContent::None,
            None,
        );
        expr.alloc(root);
        expr
    }
}

/// Attempts to construct an [`Expr`] from a given [`SyntaxSubtree`] referencing an AST node and its syntax tree.
///
/// This builds the entire expression arena iteratively from the AST subtree,
/// converting each AST node into an `ExprNode`, preserving the tree structure.
///
/// # Errors
///
/// Returns [`AiplanError`] if:
/// - Conversion of AST kinds or contents to Expr kinds/contents fails.
/// - Syntax node lookups fail.
///
/// # Example
///
/// ```rust,ignore
/// let subtree: &SyntaxSubtree<AstNode> = ...;
/// let expr = Expr::try_from(subtree)?;
/// ```
impl TryFrom<&SyntaxSubtree<'_, AstNode>> for Expr {
    type Error = AiplanError;

    fn try_from(subtree: &SyntaxSubtree<'_, AstNode>) -> Result<Self, Self::Error> {
        let mut expr = Expr::new();
        let mut stack = Vec::new();

        // Stack elements are (AST node, Option<parent ExprNodeId>)
        stack.push((subtree.node(), None));

        while let Some((current_ast_node, parent_expr_id_opt)) = stack.pop() {
            // Convert AST kind and content into Expr kind and content
            let kind = ExprKind::try_from(current_ast_node.kind())?;
            let content = ExprContent::try_from(current_ast_node.content())?;

            // Create ExprNode and allocate in arena
            let expr_node = ExprNode::new(kind, content, parent_expr_id_opt);
            let expr_node_id = expr.alloc(expr_node);

            // Link to parent if applicable
            if let Some(parent_id) = parent_expr_id_opt {
                expr.try_node_mut(parent_id)?.add_child(expr_node_id);
            }

            // Push children in reverse order to preserve left-to-right traversal
            for &child_id in current_ast_node.children().iter().rev() {
                let child_node = subtree.tree().try_node(child_id)?;
                stack.push((child_node, Some(expr_node_id)));
            }
        }

        Ok(expr)
    }
}


impl Deref for Expr {
    type Target = SyntaxTree<ExprNode>;

    /// Dereferences the `Expr` to the underlying `TreeArena<ExprNode>`.
    ///
    /// This allows transparent access to all methods of `TreeArena`.
    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for Expr {
    /// Mutable dereference to the underlying `TreeArena<ExprNode>`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Délègue au Display de TreeArena<ExprNode>
        fmt::Display::fmt(&self.tree, f)
    }
}

impl InternerDisplay for Expr {
    fn fmt_with_interner(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.tree.fmt_with_interner(f, interner)
    }
}

impl SyntaxDisplay for Expr {
    fn fmt_syntax_with_indent(&self, f: &mut Formatter<'_>, interner: &StringInterner, indent: usize) -> fmt::Result {
        let indent_str = Self::make_indent(indent);
        f.write_str(&indent_str)?;
        self.tree.fmt_with_interner(f, interner)
    }
}
