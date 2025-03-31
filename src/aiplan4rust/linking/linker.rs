use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::linking::lifted_planning_task::LiftedPlanningTask;
use crate::aiplan4rust::linking::linker_result::LinkerResult;
use crate::aiplan4rust::semantics::annotated_syntax_tree::{
    AnnotatedSyntaxTree, LiftedDomain, LiftedProblem,
};
use crate::aiplan4rust::semantics::ast_table::AstTable;
use crate::aiplan4rust::semantics::symbol::{Declaration, Source, SymbolKind, Usage};
use crate::aiplan4rust::semantics::symbol_table::SymbolTable;
use crate::aiplan4rust::semantics::type_checker::TypeChecker;
use std::mem;
use std::mem::take;

#[derive(Debug)]
pub struct Linker {
    error_manager: ErrorManager,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            error_manager: ErrorManager::new(),
        }
    }

    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    pub fn link(
        &mut self,
        domain: &LiftedDomain,
        problem: &LiftedProblem,
    ) -> Result<LinkerResult, ParserInternalError> {
        println!("Domain and problem linking");

        // TO DO: Vérifier la consistence des requirements déclarés dans le problem et dans le domaine
        // et vérifier ici qu''ils ne sont pas contradictoires
        self.check_domain_name_declaration(domain, problem)?;

        let mut problem = problem.clone();
        // Si le nom de domaine est déclaré, vérifier les symboles non déclarés
        if self.check_undeclared_problem_symbols(domain, &mut problem)? {
            println!(
                "Table des symboles de problème liée \n: {}",
                problem.symbol_table()
            );

            self.check_atomic_formula_usages(
                problem.symbol_table(),
                domain.symbol_table(),
                problem.ast(),
                problem.filename(),
            )?;
        }

        // Vérifier si des erreurs de type ParseError existent dans le gestionnaire d'erreurs
        if self
            .error_manager()
            .has_errors_of_kind(ParserErrorKind::ParseError)
        {
            // Si des erreurs existent, renvoyer LinkerResult sans LiftedPlanningTask
            Ok(LinkerResult::new(None, take(&mut self.error_manager)))
        } else {
            // Sinon, créer un LiftedPlanningTask à partir des domaines et problèmes déplaçés
            let mut domain = domain.clone();
            let lifted_planning_task =
                LiftedPlanningTask::new(take(&mut domain), take(&mut problem));

            // Retourner LinkerResult avec LiftedPlanningTask et l'ErrorManager mis à jour
            Ok(LinkerResult::new(
                Some(lifted_planning_task),
                mem::take(&mut self.error_manager),
            ))
        }
    }

    fn check_domain_name_declaration(
        &mut self,
        domain: &AnnotatedSyntaxTree,
        problem: &AnnotatedSyntaxTree,
    ) -> Result<bool, ParserInternalError> {
        // Récupérer les symboles de domaine pour le nom de domaine dans les deux arbres
        let domain_name_symbols = domain
            .symbol_table()
            .get_symbols_with_declaration_by_filter(None, Some(&SymbolKind::DomainName), None);

        // Vérification du nombre de symboles dans domain_name_symbols
        if domain_name_symbols.len() != 1 {
            return Err(ParserInternalError::new(format!(
                "Malformed Annotated Syntax Tree: multiple declaration of domain name symbol in domain: {:?}",
                domain_name_symbols,
            )));
        }

        let problem_name_symbols = problem
            .symbol_table()
            .get_symbols_with_declaration_by_filter(None, Some(&SymbolKind::DomainName), None);

        // Vérification du nombre de symboles dans problem_name_symbols
        if problem_name_symbols.len() != 1 {
            return Err(ParserInternalError::new(
                "Malformed Annotated Syntax Tree: multiple declaration of domain name symbol in problem"
                    .to_string(),
            ));
        }

        // Vérification que les noms des symboles sont identiques
        if domain_name_symbols[0].name() != problem_name_symbols[0].name() {
            let domain_name_declaration = problem.symbol_table().get_declarations_by_filter(
                Some(problem_name_symbols[0].name().as_str()),
                Some(&SymbolKind::DomainName),
                None,
            )[0];
            let ast_entry = problem
                .ast()
                .get_entry(domain_name_declaration.ast())
                .unwrap();
            let (line, column) = ast_entry.span().start_position();
            let error = ParsingError::new(
                ParserErrorKind::ParseWarning,
                Some(domain.filename().clone()),
                line,
                column,
                format!(
                    "Domain and Problem names do not match: '{}' != '{}'",
                    domain_name_symbols[0].name(),
                    problem_name_symbols[0].name()
                ),
            );
            self.error_manager.add_error(error);
        }

        Ok(true) // Tout est correct
    }

    fn check_undeclared_problem_symbols(
        &mut self,
        domain: &AnnotatedSyntaxTree,
        problem: &mut AnnotatedSyntaxTree,
    ) -> Result<bool, ParserInternalError> {
        let domain_symbol_table = domain.symbol_table();

        let mut declared = Vec::new();
        let mut undeclared = Vec::new();
        let mut no_error = true;

        // Vérifier les symboles non déclarés
        self.check_symbols_without_declarations(
            problem,
            &domain_symbol_table,
            &mut declared,
            &mut undeclared,
            &mut no_error,
        );

        // Appliquer les mises à jour après l'itération
        self.update_symbol_declaration(problem, declared);

        // Traiter les erreurs pour les symboles non déclarés
        self.process_undeclared_symbols(problem, undeclared);

        Ok(no_error)
    }

    fn check_symbols_without_declarations(
        &self,
        problem: &AnnotatedSyntaxTree,
        domain_symbol_table: &SymbolTable,
        updates: &mut Vec<(String, Declaration)>,
        undeclared: &mut Vec<(String, Usage)>,
        no_error: &mut bool,
    ) {
        let problem_symbol_table = problem.symbol_table();

        for symbol in problem_symbol_table.values() {
            if symbol.declarations().is_empty() {
                for usage in symbol.usages() {
                    if let Some(domain_declaration) = domain_symbol_table
                        .get_declarations_by_filter(
                            Some(symbol.name().as_str()),
                            Some(usage.kind()),
                            None,
                        )
                        .into_iter()
                        .next()
                    {
                        let mut domain_declaration = domain_declaration.clone();
                        domain_declaration.set_source(Source::Domain);
                        updates.push((symbol.name().to_string(), domain_declaration));
                    } else {
                        println!("{} '{}' not declared", usage.kind(), symbol.name());
                        undeclared.push((symbol.name().to_string(), usage.clone()));
                        *no_error = false;
                    }
                }
            }
        }
    }

    fn update_symbol_declaration(
        &self,
        problem: &mut AnnotatedSyntaxTree,
        updates: Vec<(String, Declaration)>,
    ) {
        let problem_symbol_table = problem.symbol_table_mut();

        for (symbol_name, domain_declaration) in updates {
            if let Some(symbol) = problem_symbol_table.get_symbol_mut(&symbol_name) {
                let kind = domain_declaration.kind().clone();
                symbol.add_declaration(domain_declaration);
                println!("{} '{}' is declared in domain", kind, symbol_name);
            }
        }
    }

    fn process_undeclared_symbols(
        &mut self,
        problem: &mut AnnotatedSyntaxTree,
        undeclared: Vec<(String, Usage)>,
    ) {
        for (symbol_name, usage) in undeclared {
            let ast_entry = problem.ast().get_entry(usage.ast()).unwrap();
            let (line, column) = ast_entry.span().start_position();
            let content = format!("{} '{}' not declared in domain", usage.kind(), symbol_name);
            let error = ParsingError::new(
                ParserErrorKind::ParseError,
                Some(problem.filename().clone()),
                line,
                column,
                content,
            );
            self.error_manager.add_error(error);
        }
    }

    // TO DO: Il faudrait mutualiser avec celle de l'analyser à
    // check_problem_atomic_expression
    pub fn check_atomic_formula_usages(
        &mut self,
        symbol_table: &SymbolTable,
        domain_symbol_table: &SymbolTable,
        ast_table: &AstTable,
        filename: &String,
    ) -> Result<bool, ParserInternalError> {
        let mut no_error = true;
        // Iterate over each symbol in the symbol table.
        println!("{}", ast_table);
        let atomic_expression_checker = TypeChecker::new(domain_symbol_table);

        for symbol in symbol_table.values() {
            for declaration in symbol.declarations() {
                if !matches!(
                    declaration.kind(),
                    SymbolKind::Predicate | SymbolKind::Function | SymbolKind::Task // Add to check compound task in HTN
                        | SymbolKind::Action // Add to check primitive task in HTN
                ) {
                    continue;
                }
                for usage in symbol.usages() {
                    if !atomic_expression_checker.match_declaration_with_usage(
                        declaration,
                        usage,
                        symbol_table,
                        ast_table,
                    )? {
                        no_error = false;
                        let entry = ast_table.get_entry(usage.ast()).unwrap();
                        let (line, column) = entry.span().start_position();
                        let content = format!(
                            "{} '{}' does not match any declaration.",
                            usage.kind(),
                            symbol.name()
                        );
                        let error = ParsingError::new(
                            ParserErrorKind::ParseError, // Use a different error kind if needed
                            Some(filename.clone()),
                            line,
                            column,
                            content,
                        );
                        self.error_manager.add_error(error);
                    }
                }
            }
        }
        Ok(no_error)
    }
}
