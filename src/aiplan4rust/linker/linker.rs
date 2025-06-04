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

        self.check_domain_name_declarations(&domain, &problem)?;

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

    /// Checks for consistency between the domain name declared in the domain AST
    /// and the domain name referenced in the problem AST.
    ///
    /// This function performs the following steps:
    /// 1. Resolves the domain name declared in the domain file.
    /// 2. Resolves the domain name referenced in the problem file.
    /// 3. Compares the two names:
    ///     - If they match, nothing happens.
    ///     - If they differ, it emits a diagnostic warning indicating the mismatch.
    /// 4. If any expected declaration or AST entry is missing, a `ParserInternalError` is returned.
    ///
    /// # Arguments
    /// * `domain` - The annotated syntax tree representing the domain file.
    /// * `problem` - The annotated syntax tree representing the problem file.
    ///
    /// # Returns
    /// * `Ok(true)` if the check completes successfully (whether or not names match).
    /// * `Err(ParserInternalError)` if a domain name declaration or AST entry is missing.
    ///
    /// # Diagnostics
    /// Emits a `DomainProblemNameMismatch` warning if the domain names differ.
    fn check_domain_name_declarations(
        &mut self,
        domain: &AnnotatedSyntaxTree,
        problem: &AnnotatedSyntaxTree,
    ) -> Result<bool, ParserInternalError> {

        // --- 1. Resolve the domain name declared in the domain AST ---
        // Tries to extract the domain name from the domain's symbol table.
        // If not found, returns an internal parser error.
        let declared_domain_name = match domain.symbol_table().resolve_domain_name_declaration()? {
            Some(name) => name,
            None => {
                return Err(ParserInternalError::new(
                    "Domain name declaration not found in domain AST".to_string(),
                ));
            }
        };

        // --- 2. Resolve the domain name referenced in the problem AST ---
        // Tries to extract the expected domain name from the problem file.
        // If not found, returns an internal parser error.
        let referenced_domain_name = match problem.symbol_table().resolve_domain_name_declaration()? {
            Some(name) => name,
            None => {
                return Err(ParserInternalError::new(
                    "Domain name declaration not found in problem AST".to_string(),
                ));
            }
        };

        // --- 3. Compare both domain names ---
        // If the names don't match, emit a diagnostic warning.
        if declared_domain_name.name() != referenced_domain_name.name() {

            // --- 4. Locate the AST node for the referenced domain name ---
            // Try to find the declaration in the problem's symbol table.
            match problem.symbol_table().resolve_declaration(
                referenced_domain_name.name(),
                &SymbolKind::DomainName,
                &Scope::root(),
            )? {
                Some(domain_name_declaration) => {

                    // --- 5. Retrieve the corresponding AST entry ---
                    // Needed to determine the span (location) for the warning.
                    match problem.get_entry(domain_name_declaration.ast()) {
                        Some(ast) => {

                            // --- 6. Emit a warning about the mismatch ---
                            // Includes both names in the diagnostic message.
                            let warning = Diagnostic::new(
                                DiagnosticKind::DomainProblemNameMismatch {
                                    domain_name: declared_domain_name.name().clone(),
                                    problem_name: referenced_domain_name.name().clone(),
                                },
                                DiagnosticSource::Linker,
                                problem.filename().clone(),
                                ast.span().clone(),
                            );
                            self.diagnostic_manager.add_diagnostic(warning);
                        }
                        None => {
                            // AST entry is missing for the declaration — this should not happen
                            return Err(ParserInternalError::new(
                                "AST entry for domain name declaration not found in problem AST.".to_string(),
                            ));
                        }
                    }
                }
                None => {
                    // No declaration found for the domain name in the problem's symbol table
                    return Err(ParserInternalError::new(
                        "Domain name declaration not found in problem symbol table".to_string(),
                    ));
                }
            }
        }

        // --- 7. Names match or warning has been emitted; return success ---
        Ok(true)
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

                for usage in symbol.usages() {

                    let domain_declaration_option = domain_symbol_table.resolve_declaration(
                        symbol.name(),
                        usage.kind(),
                        &Scope::root(),
                    )?;  // ici on propage l'erreur éventuelle

                    if let Some(domain_declaration) = domain_declaration_option {
                        let mut domain_declaration = domain_declaration.clone();
                        domain_declaration.set_source(SymbolOrigin::Domain);
                        updates.push((symbol.name().to_string(), domain_declaration));
                    } else {
                        undeclared.push((symbol.name().to_string(), usage.clone()));
                        checked &= false;
                    }
                }
            }
        }
        Ok(checked)
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
