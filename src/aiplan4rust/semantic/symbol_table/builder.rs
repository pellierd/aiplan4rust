use std::collections::VecDeque;
use std::mem;

use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::arena::{ArenaAst, ArenaAstNode};
use crate::aiplan4rust::semantic::arena::arena::Arena;
use crate::aiplan4rust::semantic::symbol::SymbolSource;
use crate::aiplan4rust::syntax::ast_old::{AstKindOld, AstNodeOld, AstOld};
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, Symbol, SymbolKind, TypedSymbol, Usage};
use crate::aiplan4rust::semantic::symbol::kind::Kind;
use crate::aiplan4rust::semantic::SymbolTable;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::lexer::token::TOTAL_TIME;

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

        let root = ast.get_node(0).unwrap();

        match root.kind() {
            AstKindOld::Domain => {
                self.table_mut().set_source(SymbolSource::Domain);
            }
            AstKindOld::Problem => {
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
        let mut stack: VecDeque<(usize, Scope)> = VecDeque::new();

        stack.push_back((0, Scope::new(0, None)));

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














    fn add_declaration_symbol(
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
    }



}
