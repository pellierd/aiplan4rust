use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::{DisplayWithInterner, StringInterner};
use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind, ExprNode};
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::FromAst;
use crate::aiplan4rust::syntax::DisplaySyntax;
use crate::aiplan4rust::tree::TreeArena;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};

/// Wrapper struct around `TreeArena<ExprNode>` representing an expression tree.
///
/// This struct provides a dedicated type for expressions, allowing
/// to add custom methods on top of the underlying `TreeArena`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Expr {
    tree: TreeArena<ExprNode>,
}

impl Expr {
    /// Creates a new, empty expression tree.
    ///
    /// # Examples
    ///
    /// ```
    /// let expr = Expr::new();
    /// assert!(expr.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            tree: TreeArena::<ExprNode>::new(),
        }
    }

    pub fn empty_or() -> Self {
        let mut empty_or = Expr::new();
        let root = ExprNode::new(ExprKind::Or, ExprContent::None, None);
        empty_or.add(root);
        empty_or
    }

    pub fn empty_and() -> Self {
        let mut empty_and = Expr::new();
        let root = ExprNode::new(ExprKind::And, ExprContent::None, None);
        empty_and.add(root);
        empty_and
    }
}

/// Implements the `FromAst` trait for `Expr`.
///
/// This allows constructing an `Expr` (expression tree) from an AST node subtree.
///
/// # Expected AST structure
///
/// The `node` is expected to be the root of an AST subtree representing a logical expression.
/// All descendants will be recursively converted into `ExprNode`s and stored in the `Expr` arena.
///
/// # Errors
///
/// Returns a `ParserInternalError` if:
/// - The `AstKind` cannot be converted into an `ExprKind`.
/// - The `AstContent` cannot be converted into an `ExprContent`.
/// - Any node lookup in the `TreeArena` fails.
///
/// # Example
///
/// ```rust
/// let expr = Expr::from_ast(ast_root_node, &arena)?;
/// ```
impl FromAst for Expr {
    fn from_ast(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Self, ParserInternalError> {
        wrap(node, ast)
    }
}

/// Converts an AST node and its subtree into an Expr iteratively.
///
/// This function builds the entire Expr arena starting from the given node reference,
/// avoiding recursion by using an explicit stack.
fn wrap(node: &AstArenaNode, ast: &TreeArena<AstArenaNode>) -> Result<Expr, ParserInternalError> {
    let mut expr = Expr::default();

    // Stack entries keep track of (AST node, ExprNodeId parent, index of next child to process)
    // For root node, parent is None.
    let mut stack = Vec::new();

    // Push root node with no parent, and child index 0
    stack.push((node, None, 0));

    while let Some((current_ast_node, parent_expr_id_opt, mut child_idx)) = stack.pop() {
        // Create the ExprNode for current AST node
        let kind = ExprKind::try_from(current_ast_node.kind())?;
        let content = ExprContent::try_from(current_ast_node.content())?;
        let expr_node = ExprNode::new(kind, content, parent_expr_id_opt);
        let expr_node_id = expr.add(expr_node);

        // Attach to parent if exists
        if let Some(parent_id) = parent_expr_id_opt {
            expr.try_node_mut(parent_id)?.add_child(expr_node_id);
        }

        let children = current_ast_node.children();

        // If this node has children, push it back with incremented child index
        // and push first child to process next
        if !children.is_empty() {
            if child_idx < children.len() {
                // Push current node back with next child index
                stack.push((current_ast_node, parent_expr_id_opt, child_idx + 1));

                // Push child node to process
                let child_node = ast.try_node(children[child_idx])?;
                stack.push((child_node, Some(expr_node_id), 0));
            }
        }
    }

    Ok(expr)
}

impl Deref for Expr {
    type Target = TreeArena<ExprNode>;

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

impl DisplayWithInterner for Expr {
    fn fmt_with(&self, f: &mut fmt::Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.tree.fmt_with(f, interner)
    }
}

impl DisplaySyntax for Expr {
    fn fmt_syntax(&self, f: &mut Formatter<'_>, interner: &StringInterner) -> fmt::Result {
        self.tree.fmt_with(f, interner)
    }
}
