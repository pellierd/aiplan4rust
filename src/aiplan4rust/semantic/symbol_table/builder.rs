use std::collections::VecDeque;
use std::mem;

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::arena::{ArenaAst, NodeId};
use crate::aiplan4rust::semantic::symbol::SymbolSource;
use crate::aiplan4rust::semantic::symbol::{Scope};

use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstKind;

pub struct SymbolTableBuilderFromArena<'a> {
    ast: &'a ArenaAst,
    table: SymbolTable,
}

impl<'a> SymbolTableBuilderFromArena<'a> {
    /// Creates a new `SymbolTableBuilder` instance.
    pub fn new(ast: &'a ArenaAst) -> Self {
        SymbolTableBuilderFromArena {
            ast,
            table: SymbolTable::new(SymbolSource::Unknown),
        }
    }

    fn table(&self) -> &SymbolTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut SymbolTable {
        &mut self.table
    }

    pub fn build_from_arena(
        &mut self,
        ast: &'a ArenaAst,
    ) -> Result<SymbolTable, ParserInternalError> {
        self.ast = ast;

        let root = ast.get_node(NodeId::ROOT_NODE_ID).unwrap();

        match root.kind() {
            AstKind::Domain => {
                self.table_mut().set_source(SymbolSource::Domain);
            }
            AstKind::Problem => {
                self.table_mut().set_source(SymbolSource::Problem);
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Invalid AST: root node is not a Domain or Problem, found: {:?}",
                    root.kind()
                )));
            }
        }

        self.initialize_from_arena()?;

        Ok(mem::take(&mut self.table))
    }

    pub fn initialize_from_arena(
        &mut self,
    ) -> Result<(), ParserInternalError> {
        let mut stack: VecDeque<(NodeId, &Scope)> = VecDeque::new();

        stack.push_back((NodeId::ROOT_NODE_ID, Scope::root()));

        while let Some((node_id, scope)) = stack.pop_back() {
            let node = self.ast.get_node(node_id).unwrap();

            /*match node.kind() {
                AstKind::DomainName(_)
                | AstKind::ProblemName(_)
                | AstKind::Requirement(_) => {
                    self.add_declaration_symbol(node_id, node, scope.clone(), None, None)?;
                }
            }*/
        }

        Ok(())
    }














    /*fn add_declaration_symbol(
        &mut self,
        node_id: usize,
        node: &ArenaAstNode,
        scope: Scope,
        types: Option<Vec<String>>,
        arguments: Option<Vec<TypedSymbol<String>>>,
    ) -> Result<(), ParserInternalError> {

        // Extract the symbol information from the AST
        let symbol_ref = self.ast.get_symbol_ref(node_id).unwrap();

        // Check if the symbol is already in the symbol table and add a declaration
        let source = self.table().source().clone();
        if let Some(symbol) = self.table_mut().get_symbol_mut(symbol_ref.name()) {
            let declaration =
                Declaration::new(
                    symbol_ref.name().to_string(),
                    symbol_ref.kind(),
                    scope,
                    source,
                    types,
                    arguments,
                    node.span().clone(),
                    node_id
                );
            symbol.add_declaration(declaration);
        } else {
            // Create a new symbol and add the declaration to it
            let mut symbol = Symbol::new(symbol_ref.name());
            let declaration =
                Declaration::new(
                    symbol_ref.name().to_string(),
                    symbol_ref.kind(),
                    scope,
                    source,
                    types,
                    arguments,
                    node.span().clone(),
                    node_id
                );
            symbol.add_declaration(declaration);
            self.table_mut().insert_symbol(symbol_ref.name().to_string(), symbol); // Insert the new symbol into the table
        }

        Ok(())
    }*/



}
