use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lang::Ident;
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::{ExprContent, ExprKind};
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::arena::{BaseNode, NodeId, Arena, ArenaNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::ops::{Deref, DerefMut};
use crate::aiplan4rust::syntax::core::{SyntaxNode, SyntaxTree};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ExprNode {
    data: BaseNode<ExprKind, ExprContent>,
}

impl ExprNode {
    pub fn new(kind: ExprKind, content: ExprContent, parent: Option<NodeId>) -> Self {
        ExprNode {
            data: BaseNode::new(kind, content, Vec::new(), parent),
        }
    }
}

impl Deref for ExprNode {
    type Target = BaseNode<ExprKind, ExprContent>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for ExprNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl fmt::Display for ExprNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children = self
            .children()
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let parent = self
            .parent()
            .map_or("none".to_string(), |idx| idx.to_string());

        write!(
            f,
            "[kind={}, content={}, parent={}, children=[{}]]",
            self.kind(),
            self.content(),
            parent,
            children,
        )
    }
}

impl ArenaNode for ExprNode {
    type Kind = ExprKind;
    type Content = ExprContent;

    fn kind(&self) -> Self::Kind {
        self.data.kind()
    }

    fn set_kind(&mut self, kind: Self::Kind) {
        self.data.set_kind(kind);
    }

    fn content(&self) -> &Self::Content {
        &self.data.content()
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.data.content_mut()
    }

    fn parent(&self) -> Option<NodeId> {
        self.data.parent()
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.data.set_parent(parent)
    }

    fn children(&self) -> &[NodeId] {
        self.data.children()
    }

    fn set_children(&mut self, children: Vec<NodeId>) {
        self.data.set_children(children)
    }

    fn add_child(&mut self, child: NodeId) {
        self.data.add_child(child)
    }

}

impl SyntaxNode for ExprNode {
    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        self.data.remap_idents(map)
    }

    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, AiplanError> {
        let kind = self.kind();
        let symbol_kind = match kind {
            ExprKind::PrimitiveType => SymbolKind::PrimitiveType,
            ExprKind::Constant => SymbolKind::Constant,
            ExprKind::Variable => SymbolKind::Variable,
            ExprKind::FunctionSymbol => SymbolKind::Function,
            ExprKind::Predicate => SymbolKind::Predicate,
            ExprKind::TaskSymbol => SymbolKind::Task,
            ExprKind::TaskID => SymbolKind::TaskID,
            _ => return Ok(None),
        };

        let ident = self.try_ident()?;
        Ok(Some(SymbolRef::new(ident, symbol_kind)))
    }
    /// Recursively pretty-prints this syntax and its children as a arena.
    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        fn fmt_node(
            node: &ExprNode,
            f: &mut Formatter<'_>,
            arena: &SyntaxTree<ExprNode>,
            interner: &StringInterner,
            prefix: &str,
            last: bool,
        ) -> fmt::Result {
            let branch = if last { "└─" } else { "├─" };

            let content_str = match node.content() {
                Content::None => String::new(),
                Content::Ident(id) => format!(" [{}]", id.to_string_with_interner(interner)),
                other => format!(" [{}]", other),
            };

            let children = node.children();
            let len = children.len();

            // Affiche ligne courante (pas de saut de ligne ici)
            write!(f, "{}{}{}{}", prefix, branch, node.kind(), content_str)?;

            if len > 0 {
                // Si on a des enfants, on ajoute un saut de ligne pour commencer leur indentation
                write!(f, "\n")?;
            }

            let new_prefix = if last {
                format!("{}   ", prefix)
            } else {
                format!("{}│  ", prefix)
            };

            for (i, child_idx) in children.iter().enumerate() {
                let child = arena
                    .get_node(*child_idx)
                    .expect("Child not found in arena");

                fmt_node(child, f, arena, interner, &new_prefix, i == len - 1)?;

                // Saut de ligne entre enfants (sauf après le dernier)
                if i < len - 1 {
                    write!(f, "\n")?;
                }
            }

            Ok(())
        }

        fmt_node(self, f, arena, interner, "", true)
    }

    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result
    where
        Self: Sized,
    {
        self.fmt_with_interner(f, arena, interner)?;
        Ok(())
    }
}
