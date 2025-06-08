
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSeverity, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::linker::LiftedPlanningTask;
use crate::aiplan4rust::linker::LinkerResult;
use crate::aiplan4rust::parser::SymbolOrigin;
use crate::aiplan4rust::semantic_analyser::checkers::task_ordering_checker;
use crate::aiplan4rust::semantic_analyser::checkers::{
    functional_expression_checker, requirement_checker, TypeChecker,
};
use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, Scope, SymbolKind};
use crate::aiplan4rust::semantic_analyser::symbol::Usage;
use crate::aiplan4rust::semantic_analyser::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::LiftedDomain;
use crate::aiplan4rust::semantic_analyser::LiftedProblem;
use crate::aiplan4rust::semantic_analyser::SymbolTable;

use std::mem;
use std::mem::take;

use crate::aiplan4rust::linker::checkers::domain_name_checker;
use crate::aiplan4rust::semantic_checks;
use crate::aiplan4rust::semantic_checks::checker_context::CheckerContext;

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

        domain_name_checker::check(&domain, &problem, &mut self.diagnostic_manager)?;

        Self::update_problem_symbols_table_from_domain(&mut problem, domain.symbol_table())?;

        println!("{}", problem.symbol_table());

        //let mut problem = problem.clone();
        // Si le nom de domaine est déclaré, vérifier les symboles non déclarés
        //if self.check_undeclared_symbols(&problem) {
        /*if SemanticAnalyzer::check_symbols(
            &problem,
            &vec![],
            &vec![],
            &mut self.diagnostic_manager)? {*/
        if semantic_checks::check_undeclared_symbols(&problem, &[], &mut self.diagnostic_manager, CheckerContext::Linker)?
            && semantic_checks::check_cross_duplicate_symbol_declarations(&domain, &problem, &mut self.diagnostic_manager, CheckerContext::Linker)? {

            let type_checker = TypeChecker::new(&domain.symbol_table());
            semantic_checks::check(&problem, &type_checker, &mut self.diagnostic_manager)?;

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


    /// Updates the problem's symbol table by adding declarations found in the domain's symbol table.
    ///
    /// For each symbol in the problem's symbol table that has no declarations,
    /// this function attempts to find matching declarations from the domain symbol table
    /// based on the symbol's name and the usage kind. Matching declarations are cloned,
    /// their origin is marked as coming from the domain, and then added to the problem's symbol.
    ///
    /// This function does not directly handle undeclared symbols diagnostics; instead,
    /// it relies on the helper `collect_declared_and_undeclared_symbols` to gather
    /// necessary updates and undeclared symbols.
    ///
    /// # Arguments
    ///
    /// * `problem` - A mutable reference to the `AnnotatedSyntaxTree` representing the problem.
    /// * `domain_symbol_table` - A reference to the domain's `SymbolTable`.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or propagates any `ParserInternalError` encountered
    /// during resolution.
    ///
    /// # Example
    ///
    /// ```ignore
    /// update_problem_symbols_table_from_domain(&mut problem_ast, &domain_symbol_table)?;
    /// ```
    ///
    fn update_problem_symbols_table_from_domain<'a>(
        problem: &'a mut AnnotatedSyntaxTree,
        domain_symbol_table: &'a SymbolTable,
    ) -> Result<(), ParserInternalError> {
        let mut declared = Vec::new();
        let mut undeclared = Vec::new();

        // Collect symbol declarations to add and gather undeclared symbols.
        Self::collect_declared_and_undeclared_symbols(problem, domain_symbol_table, &mut declared, &mut undeclared)?;

        // Get mutable access to problem's symbol table.
        let problem_symbol_table = problem.symbol_table_mut();

        // Apply collected declarations to symbols in the problem's symbol table.
        for (symbol_name, declaration) in declared {
            if let Some(symbol) = problem_symbol_table.get_symbol_mut(&symbol_name) {
                symbol.add_declaration(declaration);
            }
        }

        Ok(())
    }


    /// Collects symbol declarations from the domain symbol table for symbols in the problem
    /// that currently lack declarations, and gathers undeclared symbols.
    ///
    /// This function performs no mutation on the problem or domain symbol tables.
    /// Instead, it collects:
    /// - `updates`: a vector of `(symbol_name, declaration)` pairs to be added to the problem.
    /// - `undeclared`: a vector of references to symbols and their usages that could not be
    ///   resolved.
    ///
    /// The function returns `Ok(true)` if all symbols were resolved, or `Ok(false)` if some
    /// remain undeclared. It propagates any internal errors encountered during resolution.
    ///
    /// # Arguments
    ///
    /// * `problem` - A reference to the problem's annotated syntax tree.
    /// * `domain_symbol_table` - A reference to the domain's symbol table.
    /// * `updates` - A mutable vector to collect declarations to add.
    /// * `undeclared` - A mutable vector to collect undeclared symbols.
    ///
    /// # Returns
    ///
    /// A `Result<bool, ParserInternalError>`. The boolean indicates whether all symbols were
    /// resolved (`true`) or not (`false`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut updates = Vec::new();
    /// let mut undeclared = Vec::new();
    /// let all_resolved = collect_declared_and_undeclared_symbols(
    ///     &problem,
    ///     &domain_symbol_table,
    ///     &mut updates,
    ///     &mut undeclared
    /// )?;
    /// if !all_resolved {
    ///     // Handle undeclared symbols diagnostics...
    /// }
    /// ```
    ///
    fn collect_declared_and_undeclared_symbols<'a>(
        problem: &'a AnnotatedSyntaxTree,
        domain_symbol_table: &'a SymbolTable,
        declared: &mut Vec<(String, Declaration)>,
        undeclared: &mut Vec<(&'a String, &'a Usage)>,
    ) -> Result<bool, ParserInternalError> {
        let problem_symbol_table = problem.symbol_table();
        let mut all_resolved = true;

        // Iterate over all symbols in the problem symbol table.
        for symbol in problem_symbol_table.values() {
            // Only process symbols with no declarations.
            if symbol.declarations().is_empty() {

                // For each usage of the symbol, try to resolve a matching declaration in the domain.
                for usage in symbol.usages() {

                    // Attempt to resolve declaration from domain by symbol name and usage kind.
                    // Propagate error if resolution fails.
                    let domain_declaration_option = domain_symbol_table.resolve_declaration(
                        symbol.name(),
                        usage.kind(),
                        &Scope::root(),
                    )?;

                    if let Some(domain_declaration) = domain_declaration_option {
                        // Clone the declaration and mark it as originating from the domain.
                        let mut domain_declaration = domain_declaration.clone();
                        domain_declaration.set_source(SymbolOrigin::Domain);

                        // Queue the declaration to be added to the problem's symbol table.
                        declared.push((symbol.name().to_string(), domain_declaration));
                    } else {
                        // No matching declaration found in domain: record the undeclared symbol.
                        undeclared.push((symbol.name(), usage));

                        // Mark that not all symbols could be resolved.
                        all_resolved = false;
                    }
                }
            }
        }

        Ok(all_resolved)
    }
}
