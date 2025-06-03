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
use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, Scope};
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
        domain: LiftedDomain,
        mut problem: LiftedProblem,
    ) -> Result<LinkerResult, ParserInternalError> {

        // TO DO: Vérifier la consistence des requirements déclarés dans le problem et dans le domaine
        // et vérifier ici qu''ils ne sont pas contradictoires

        self.check_domain_name_declaration(&domain, &problem)?;

        //let mut problem = problem.clone();
        // Si le nom de domaine est déclaré, vérifier les symboles non déclarés
        if self.check_undeclared_problem_symbols(&domain, &mut problem)? {

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
            //let mut domain = domain.clone();
            let lifted_planning_task =
                LiftedPlanningTask::new(domain, problem);

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

        // Vérifier les symboles non déclarés
        let no_error = self.check_symbols_without_declarations(
            problem,
            &domain_symbol_table,
            &mut declared,
            &mut undeclared,
        )?;

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
    ) -> Result<bool, ParserInternalError> {
        let problem_symbol_table = problem.symbol_table();
        let mut checked = true;

        for symbol in problem_symbol_table.values() {
            if symbol.declarations().is_empty() {
                if symbol.name() == "move_vehicle_no_traincar" {

                    let a = domain_symbol_table.get_symbol(symbol.name());
                    println!("++++++++++++++++++{:?}\n{}", a, domain_symbol_table);
                }

                for usage in symbol.usages() {

                    let domain_declaration_option = Self::get_declaration_by_filter(
                        domain_symbol_table,
                        symbol.name(),
                        usage.kind(),
                        &Scope::root(),
                    )?;  // ici on propage l'erreur éventuelle

                    if let Some(domain_declaration) = domain_declaration_option {
                        let mut domain_declaration = domain_declaration.clone();
                        domain_declaration.set_source(SymbolOrigin::Domain);
                        updates.push((symbol.name().to_string(), domain_declaration));
                    } else {

                        println!("{} '{}' not declared", usage.kind(), symbol.name());
                        undeclared.push((symbol.name().to_string(), usage.clone()));
                        checked &= false;

                        println!("*********************************•\n{}", problem_symbol_table);
                    }
                }
            }
        }
        Ok(checked)
    }

    fn get_declaration_by_filter<'a>(
        symbol_table: &'a SymbolTable,
        symbol_name: &str,
        usage_kind: &SymbolKind,
        scope: &Scope,
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        let fetch_valid = |kind: SymbolKind| {
            let decls = symbol_table.get_declarations_by_filter(
                Some(symbol_name),
                Some(&kind),
                Some(scope),
            );
            Self::find_valid_declaration(symbol_name, usage_kind, &decls)
        };

        match usage_kind {
            SymbolKind::Task => match fetch_valid(SymbolKind::Task)? {
                Some(decl) => Ok(Some(decl)),
                None => fetch_valid(SymbolKind::Action),
            },
            _ => fetch_valid(*usage_kind),
        }
    }


    fn find_valid_declaration<'a>(
        symbol_name: &str,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        match usage_kind {
            SymbolKind::PrimitiveType | SymbolKind::Predicate => {
                Self::validate_declarations_for_type_predicate(symbol_name, &usage_kind, declarations)
            }
            SymbolKind::Task => {
                Self::validate_declarations_for_task(symbol_name, declarations)
            }
            _ => match declarations.len() {
                0 => Ok(None),
                1 => Ok(Some(declarations[0])),
                _ => Err(Self::multiple_declarations_error(symbol_name, &usage_kind, declarations.len())),
            },
        }
    }

    fn is_kind_allowed_for_usage(usage_kind: &SymbolKind, decl_kind: &SymbolKind) -> bool {
        match usage_kind {
            SymbolKind::PrimitiveType => *decl_kind == SymbolKind::PrimitiveType || *decl_kind == SymbolKind::Predicate,
            SymbolKind::Predicate => *decl_kind == SymbolKind::Predicate || *decl_kind == SymbolKind::PrimitiveType,
            SymbolKind::Task => *decl_kind == SymbolKind::Task || *decl_kind == SymbolKind::Action,
            _ => false,
        }
    }


    fn validate_declarations_for_type_predicate<'a>(
        symbol_name: &str,
        usage_kind: &SymbolKind,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        let matching: Vec<_> = declarations
            .iter()
            .filter(|d| d.kind() == usage_kind)
            .collect();

        match matching.len() {
            0 => Ok(None),
            1 => match declarations.len() {
                1 => Ok(Some(matching[0])),
                2 => match declarations.iter().find(|d| d.kind() != usage_kind) {
                    Some(other) if Self::is_kind_allowed_for_usage(&usage_kind, other.kind()) => {
                        Ok(Some(matching[0]))
                    }
                    _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, declarations.len())),
                },
                _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, declarations.len())),
            },
            _ => Err(Self::multiple_declarations_error(symbol_name, usage_kind, matching.len())),
        }
    }

    fn validate_declarations_for_task<'a>(
        symbol_name: &str,
        declarations: &[&'a Declaration],
    ) -> Result<Option<&'a Declaration>, ParserInternalError> {
        if declarations.len() > 1 {
            return Err(Self::multiple_declarations_error(symbol_name, &SymbolKind::Task, declarations.len()));
        }

        match declarations.first() {
            Some(decl) => match decl.kind() {
                SymbolKind::Task | SymbolKind::Action => {
                    Ok(Some(decl))
                },
                _ => {
                    Ok(None)
                },
            },
            None => Ok(None),
        }
    }

    fn multiple_declarations_error(
        symbol_name: &str,
        usage_kind: &SymbolKind,
        count: usize,
    ) -> ParserInternalError {
        ParserInternalError::new(format!(
            "Symbol '{}' with kind '{:?}' has {} declarations, which is invalid.",
            symbol_name, usage_kind, count
        ))
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
