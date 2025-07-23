use std::collections::HashMap;
use std::fmt::{self, Formatter};
use std::ops::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{Ident, Requirement};
use crate::aiplan4rust::semantic::symbol::{SymbolKind, SymbolRef};
use crate::aiplan4rust::syntax::ast::{renderer, AstContent, AstError, AstKind};
use crate::aiplan4rust::syntax::tree::{SyntaxBaseNode, SyntaxNode, SyntaxTree, NodeId};
use crate::aiplan4rust::syntax::Span;
use crate::aiplan4rust::AiplanError;
use crate::aiplan4rust::syntax::tree::error::SyntaxTreeError;

/// Represents a node in the Abstract Syntax Tree (AST).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AstNode {
    inner: SyntaxBaseNode<AstKind, AstContent>,
    span: Span,
}

impl AstNode {
    pub fn new(
        kind: AstKind,
        content: AstContent,
        children: Vec<NodeId>,
        span: Span,
        parent: Option<NodeId>,
    ) -> Self {
        let inner = SyntaxBaseNode::new(kind, content, children, parent);
        AstNode { inner, span }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn span_mut(&mut self) -> &mut Span {
        &mut self.span
    }

    pub fn as_requirement(&self) -> Option<Requirement> {
        self.content().as_requirement()
    }

    pub fn try_requirement(&self) -> Result<Requirement, AiplanError> {
        self.content().try_requirement()
    }
}

// Delegates everything to `inner`
impl Deref for AstNode {
    type Target = SyntaxBaseNode<AstKind, AstContent>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AstNode {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl fmt::Display for AstNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderer::default::render(self, f)
    }
}

impl ArenaNode for AstNode {
    fn parent(&self) -> Option<NodeId> {
        self.inner.parent()
    }

    fn set_parent(&mut self, parent: Option<NodeId>) {
        self.inner.set_parent(parent);
    }

    fn children(&self) -> &[NodeId] {
        self.inner.children()
    }

    fn set_children(&mut self, children: Vec<NodeId>) {
        self.inner.set_children(children);
    }

    fn add_child(&mut self, child: NodeId) {
        self.inner.add_child(child);
    }
}

impl SyntaxNode for AstNode {
    type Kind = AstKind;
    type Content = AstContent;

    fn kind(&self) -> Self::Kind {
        self.inner.kind()
    }

    fn set_kind(&mut self, kind: Self::Kind) {
        self.inner.set_kind(kind);
    }

    fn content(&self) -> &Self::Content {
        self.inner.content()
    }

    fn content_mut(&mut self) -> &mut Self::Content {
        self.inner.content_mut()
    }

    fn as_symbol_ref(&self) -> Result<Option<SymbolRef>, SyntaxTreeError> {
        let symbol_kind = match self.kind() {
            AstKind::DomainName => SymbolKind::DomainName,
            AstKind::PrimitiveType => SymbolKind::PrimitiveType,
            AstKind::ProblemName => SymbolKind::ProblemName,
            AstKind::Constant => SymbolKind::Constant,
            AstKind::Variable => SymbolKind::Variable,
            AstKind::FunctionSymbol => SymbolKind::Function,
            AstKind::Predicate => SymbolKind::Predicate,
            AstKind::ActionSymbol => SymbolKind::Action,
            AstKind::DASymbol => SymbolKind::DASymbol,
            AstKind::MethodSymbol => SymbolKind::Method,
            AstKind::TaskSymbol => SymbolKind::Task,
            AstKind::TaskID => SymbolKind::TaskID,
            _ => return Ok(None),
        };

        let ident = self.try_ident()?;
        Ok(Some(SymbolRef::new(ident, symbol_kind)))
    }

    fn remap_idents(&mut self, map: &HashMap<Ident, Ident>) {
        if let AstContent::Ident(id) = self.content_mut() {
            if let Some(&new_id) = map.get(id) {
                *id = new_id;
            }
        }
    }

    fn fmt_with_interner(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
    ) -> fmt::Result {
        renderer::tree::render(self, f, arena, interner)
    }

    fn fmt_syntax_with_indent(
        &self,
        f: &mut Formatter<'_>,
        arena: &SyntaxTree<Self>,
        interner: &StringInterner,
        indent: usize,
    ) -> fmt::Result {
        renderer::syntax::render_with_indent(self, f, arena, interner, indent)
    }
}
