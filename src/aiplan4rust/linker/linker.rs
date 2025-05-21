use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSeverity, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::linker::LiftedPlanningTask;
use crate::aiplan4rust::linker::LinkerResult;
use crate::aiplan4rust::parser::SymbolOrigin;
use crate::aiplan4rust::semantic_analyser::checkers::{
    atomic_formula_checker, task_ordering_checker,
};
use crate::aiplan4rust::semantic_analyser::checkers::{
    functional_expression_checker, requirement_checker, TypeChecker,
};
use crate::aiplan4rust::semantic_analyser::symbol::Declaration;
use crate::aiplan4rust::semantic_analyser::symbol::SymbolKind;
use crate::aiplan4rust::semantic_analyser::symbol::Usage;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::LiftedDomain;
use crate::aiplan4rust::semantic_analyser::LiftedProblem;
use crate::aiplan4rust::semantic_analyser::SymbolTable;

use std::mem;
use std::mem::take;

#[derive(Debug)]
pub struct Linker {
    diagnostic_manager: DiagnosticManager,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    pub fn link(
        &mut self,
        domain: &LiftedDomain,
        problem: &LiftedProblem,
    ) -> Result<LinkerResult, ParserInternalError> {

        // TO DO: Vérifier la consistence des requirements déclarés dans le problem et dans le domaine
        // et vérifier ici qu''ils ne sont pas contradictoires
        self.check_domain_name_declaration(domain, problem)?;

        let mut problem = problem.clone();
        // Si le nom de domaine est déclaré, vérifier les symboles non déclarés
        if self.check_undeclared_problem_symbols(domain, &mut problem)? {

            let type_checker = TypeChecker::new(&domain.symbol_table());
            atomic_formula_checker::check(&problem, &type_checker, &mut self.diagnostic_manager)?;

            // Check functional expressions in the domain using the type checker
            functional_expression_checker::check(&problem, &type_checker, &mut self.diagnostic_manager)?;

            task_ordering_checker::check(&problem, &mut self.diagnostic_manager)?;

            let mut requirements = domain.requirements().clone();
            requirements.extend(problem.requirements().clone());
            requirement_checker::check(&problem, &requirements, &mut self.diagnostic_manager)?;
        }

        // Vérifier si des erreurs de type ParseError existent dans le gestionnaire d'erreurs
        if self
            .diagnostic_manager()
            .has_diagnotics_of_severity(DiagnosticSeverity::Error)
        {
            // Si des erreurs existent, renvoyer LinkerResult sans LiftedPlanningTask
            Ok(LinkerResult::new(None, take(&mut self.diagnostic_manager)))
        } else {
            // Sinon, créer un LiftedPlanningTask à partir des domaines et problèmes déplaçés
            let mut domain = domain.clone();
            let lifted_planning_task =
                LiftedPlanningTask::new(take(&mut domain), take(&mut problem));

            // Retourner LinkerResult avec LiftedPlanningTask et l'ErrorManager mis à jour
            Ok(LinkerResult::new(
                Some(lifted_planning_task),
                mem::take(&mut self.diagnostic_manager),
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
            let ast_entry = problem.get_entry(domain_name_declaration.ast()).unwrap();
            let warning = Diagnostic::new(
                DiagnosticKind::DomainProblemNameMismatch {
                    domain_name : domain_name_symbols[0].name().clone(),
                    problem_name : problem_name_symbols[0].name().clone(),
                },
                DiagnosticSource::Linker,
                problem.filename().clone(),
                ast_entry.span().clone(),
            );
            self.diagnostic_manager.add_diagnostic(warning);
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
                        domain_declaration.set_source(SymbolOrigin::Domain);
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
                //let kind = domain_declaration.kind().clone();
                symbol.add_declaration(domain_declaration);
                //println!("{} '{}' is declared in domain", kind, symbol_name);
            }
        }
    }

    fn process_undeclared_symbols(
        &mut self,
        problem: &mut AnnotatedSyntaxTree,
        undeclared: Vec<(String, Usage)>,
    ) {
        for (symbol_name, usage) in undeclared {
            let ast_entry = problem.get_entry(usage.ast()).unwrap();
            let error = Diagnostic::new(
                DiagnosticKind::UndeclaredSymbol {
                    symbol: symbol_name.clone(),
                    kind: usage.kind().clone(),
                },
                DiagnosticSource::SemanticAnalyzer,
                problem.filename().clone(),
                ast_entry.span().clone(),
            );

            self.diagnostic_manager.add_diagnostic(error);
        }
    }
}
