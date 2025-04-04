use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::ParserErrorKind::ParseError;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::analyser_result::AnalyzerResult;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::checkers::symbol_declaration_checker;
use crate::aiplan4rust::semantics::checkers::undeclared_symbol_checker;
use crate::aiplan4rust::semantics::checkers::TypeChecker;
use crate::aiplan4rust::semantics::checkers::{
    atomic_formula_checker, functional_expression_checker,
};
use crate::aiplan4rust::semantics::symbol::SymbolKind;
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::syntax_tree::SyntaxTree;
use std::mem;
//const PDDL_BUILTIN_SYMBOLS: [&str; 4] = [OBJECT_TYPE, NUMBER_TYPE, TOTAL_TIME, DURATION_VARIABLE];

#[derive(Debug)]
pub struct Analyzer {
    error_manager: ErrorManager,
}

impl Analyzer {
    pub fn new() -> Self {
        Self {
            error_manager: ErrorManager::new(),
        }
    }

    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    pub fn analyze(
        &mut self,
        syntax_tree: &SyntaxTree,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        // Vérification si l'AST existe dans le syntax_tree
        let ast = syntax_tree.ast();

        // Convertir l'AST en hash map
        let ast_table = AstTable::from(&ast)?;
        println!("{}", ast_table);

        // Créer le SymbolTable avec l'AST et le Bimap
        let mut symbol_table = SymbolTable::new();
        symbol_table.initialize_from_ast(0, &ast_table)?;

        // Print the symbol table for debugging
        println!("{}", symbol_table);

        let annotated_syntax_tree = AnnotatedSyntaxTree::new(
            ast_table,
            symbol_table,
            syntax_tree.filename().unwrap().clone(),
            std::time::SystemTime::now(),
        );

        match ast.kind() {
            AstKind::Domain => {
                if symbol_declaration_checker::check(
                    &annotated_syntax_tree,
                    &mut self.error_manager,
                )? && undeclared_symbol_checker::check(
                    &annotated_syntax_tree,
                    &[],
                    &mut self.error_manager,
                )? {
                    let type_checker = TypeChecker::new(annotated_syntax_tree.symbol_table());
                    atomic_formula_checker::check(
                        &annotated_syntax_tree,
                        &type_checker,
                        &mut self.error_manager,
                    )?;

                    functional_expression_checker::check(
                        &annotated_syntax_tree,
                        &type_checker,
                        &mut self.error_manager,
                    )?;
                }

                // Vérification des symboles inutilisés, indépendamment des précédentes vérifications
                let skip_symbols = &[SymbolKind::Constant];
                //self.check_symbol_usage(&symbol_table, &ast_table, skip_symbols)?;
                undeclared_symbol_checker::check(
                    &annotated_syntax_tree,
                    skip_symbols,
                    &mut self.error_manager,
                )?;
            }
            AstKind::Problem => {
                symbol_declaration_checker::check(&annotated_syntax_tree, &mut self.error_manager)?;

                // Vérification des symboles non déclarés
                let skip_symbols = &[
                    SymbolKind::PrimitiveType,
                    SymbolKind::Constant,
                    SymbolKind::Predicate,
                    SymbolKind::Function,
                    SymbolKind::Task, // Add for HDDL
                ];
                undeclared_symbol_checker::check(
                    &annotated_syntax_tree,
                    skip_symbols,
                    &mut self.error_manager,
                )?;

                // Vérification des symboles inutilisés, indépendamment des précédentes vérifications
                undeclared_symbol_checker::check(
                    &annotated_syntax_tree,
                    &[],
                    &mut self.error_manager,
                )?;
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST node kind found: {}",
                    ast.kind()
                )));
            }
        }

        if !self.error_manager.has_errors_of_kind(ParseError) {
            Ok(AnalyzerResult::new(
                Some(annotated_syntax_tree),
                mem::take(&mut self.error_manager),
            ))
        } else {
            Ok(AnalyzerResult::new(
                None,
                mem::take(&mut self.error_manager),
            ))
        }
    }
}
