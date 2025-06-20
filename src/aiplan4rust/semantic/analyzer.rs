use crate::aiplan4rust::diagnostic::{DiagnosticManager, Severity, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::{SemanticContext, TypeChecker};
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::semantic;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};

/// The `Analyzer` struct is responsible for performing semantic analysis on a `SyntaxTree`.
///
/// It manages the detection and collection of errors encountered during the analysis process.
/// The `error_manager` field stores any errors found while analyzing the syntax tree.
#[derive(Debug)]
pub struct Analyzer {
    /// Manages and tracks parsing and semantic errors encountered during analysis.
    diagnostic_manager: DiagnosticManager
}

impl Analyzer {
    /// Creates a new instance of `Analyzer`.
    ///
    /// # Returns
    ///
    /// Returns an `Analyzer` instance with an initialized `ErrorManager`.
    ///
    /// # Example
    ///
    /// ```
    /// let analyzer = Analyzer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the `ErrorManager` for accessing collected errors.
    ///
    /// # Returns
    ///
    /// Returns a reference to the internal `ErrorManager` instance.
    ///
    /// # Example
    ///
    /// ```
    /// let analyzer = Analyzer::new();
    /// let errors = analyzer.error_manager();
    /// ```
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    pub fn analyze(&mut self, ast: &Ast) -> Result<AnalyzerResult, ParserInternalError> {
        self.perform_analysis(ast)
    }

    pub fn analyze_with_diagnostic_manager(
        &mut self,
        ast: &Ast,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        self.diagnostic_manager = diagnostic_manager;
        self.perform_analysis(ast)
    }

    fn perform_analysis(
        &mut self,
        ast: &Ast,
    ) -> Result<AnalyzerResult, ParserInternalError> {

        let context = SemanticContext::from(ast)?;

        // Step 2: Determine kind and apply semantic checks
        match ast.root().kind() {
            AstKind::Domain => {
                Self::check_domain(&context, &mut self.diagnostic_manager)?;
            }
            AstKind::Problem => {
                Self::check_problem(&context, &mut self.diagnostic_manager)?;
            }
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST node kind found: {}",
                    ast.root().kind()
                )));
            }
        }

        // Step 3: Build the result depending on errors
        if !self.diagnostic_manager.has_diagnotics_of_severity(Severity::Error) {
            Ok(AnalyzerResult::new(
                Some(context),
                std::mem::take(&mut self.diagnostic_manager),
            ))
        } else {
            Ok(AnalyzerResult::new(
                None,
                std::mem::take(&mut self.diagnostic_manager),
            ))
        }
    }

    /// Checks the domain-related syntax tree and performs the relevant checks.
    ///
    /// This function performs several checks specific to the domain section of the syntax tree:
    /// 1. It first checks the declared symbols against the given exclusions for unused symbols.
    /// 2. If the symbol check passes, it proceeds to check for atomic formula correctness.
    /// 3. Then, it checks the functional expression correctness using the type information from the
    ///   symbol table.
    ///
    /// # Parameters
    /// - `annotated_syntax_tree`: The annotated syntax tree that contains the domain-related AST.
    ///    This tree includes the structure of the domain and the associated symbol table.
    ///
    /// # Returns
    /// Returns a `Result<bool, ParserInternalError>`.
    /// - `Ok(true)` if all checks pass without errors.
    /// - `Ok(false)` if any check fails.
    /// - `Err(ParserInternalError)` if there is an internal error during the checking process.
    ///
    /// # Example
    /// ```rust
    /// let result = my_analyzer.check_domain(&annotated_syntax_tree);
    /// match result {
    ///     Ok(true) => println!("Domain is valid."),
    ///     Ok(false) => println!("Domain check failed."),
    ///     Err(e) => eprintln!("Error during domain check: {}", e),
    /// }
    /// ```
    fn check_domain(
        context: &SemanticContext,
        diagnostic_manager: &mut DiagnosticManager
    ) -> Result<bool, ParserInternalError> {
        // Skip unused symbols of kind Constant during the checks
        let skip_symbols_unused = &[SymbolKind::Constant];

        // Perform the first symbol check (declared symbols check)
        let mut checked= Self::check_symbols(
            context,
            &[],                 // No symbols to skip for declared symbols check
            skip_symbols_unused, // Skip symbols of type Constant for unused symbol check
            diagnostic_manager,
        )?;

        checked &= checked && semantic::checks::check_type_hierarchy(
            context,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // If the symbol check passes without errors, proceed with further checks
        if checked {


            // Create a type checker using the symbol table from the annotated syntax tree
            let type_checker = TypeChecker::new(context.symbol_table());

            // Check atomic formulas in the domain using the type checker
            checked &= semantic::checks::check_declared_symbol_signatures(
                context,
                &type_checker,
                diagnostic_manager,
            )?;

            // Check functional expressions in the domain using the type checker
            checked &= semantic::checks::check_typed_expressions(
                context,
                &type_checker,
                Provider::Analyzer,
                diagnostic_manager,
            )?;

            checked &= semantic::checks::check_task_ordering(
                context,
                Provider::Analyzer,
                diagnostic_manager
            )?;

            semantic::checks::check_requirement_violations(
                context,
                context.requirements(),
                Provider::Analyzer,
                diagnostic_manager,
            )?;
        }

        // Return the result of the checks (true if all checks passed, false otherwise)
        Ok(checked)
    }

    /// Analyzes the problem-related syntax tree and performs relevant checks.
    ///
    /// This method checks for any symbol-related issues in the provided annotated syntax tree,
    /// excluding certain types of symbols (like primitive types, constants, predicates, functions, and tasks).
    /// Errors encountered during the check are recorded in the `error_manager`.
    ///
    /// # Arguments
    ///
    /// * `annotated_syntax_tree` - A reference to the `AnnotatedSyntaxTree` that will be checked.
    ///
    /// # Returns
    ///
    /// This method returns a `Result<bool, ParserInternalError>`.
    /// * `Ok(true)` indicates that no errors were found during the check.
    /// * `Ok(false)` indicates that errors were found.
    /// * `Err(ParserInternalError)` indicates an internal error occurred.
    fn check_problem(
        context: &SemanticContext,
        diagnostic_manager: &mut DiagnosticManager
    ) -> Result<bool, ParserInternalError> {
        let skip_types_undeclared = &[
            SymbolKind::PrimitiveType,
            SymbolKind::Constant,
            SymbolKind::Predicate,
            SymbolKind::Function,
            SymbolKind::Task, // Add for HDDL
        ];

        let mut checked = Self::check_symbols(
            context,
            skip_types_undeclared,
            &[],
            diagnostic_manager,
        )?;

        checked &= semantic::checks::check_task_ordering(
            context,
            Provider::Analyzer,
            diagnostic_manager
        )?;

        Ok(checked)
    }

    /// Checks the symbols in the given annotated syntax tree for various types of symbol-related
    /// errors.
    ///
    /// This function performs the following checks:
    /// - Verifies declared symbols.
    /// - Verifies undeclared symbols, skipping specific types.
    /// - Verifies unused symbols, skipping specific ones.
    ///
    /// # Arguments
    ///
    /// * `annotated_syntax_tree` - An `AnnotatedSyntaxTree` representing the parsed code that needs
    ///   to be checked.
    /// * `skip_types_undeclared` - A slice of `SymbolKind` specifying the types of symbols to skip
    ///   during undeclared symbol checking.
    /// * `skip_symbols_unused` - A slice of `SymbolKind` specifying the symbols to skip during
    ///   unused symbol checking.
    /// * `error_manager` - A mutable reference to an `ErrorManager` where any errors found during
    ///  the checks will be stored.
    ///
    /// # Returns
    ///
    /// This function returns a `Result<bool, ParserInternalError>`.
    /// - `Ok(true)` if no errors are found (i.e., the checks passed).
    /// - `Ok(false)` if errors are found during any of the checks.
    /// - `Err(ParserInternalError)` if an internal error occurs during the process.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut error_manager = ErrorManager::new();
    /// let result = check_symbols(&annotated_syntax_tree, &skip_types, &skip_symbols, &mut error_manager);
    /// match result {
    ///     Ok(true) => println!("All checks passed."),
    ///     Ok(false) => println!("Some checks failed."),
    ///     Err(err) => println!("An internal error occurred: {}", err),
    /// }
    /// ```
    pub fn check_symbols(
        context: &SemanticContext,
        skip_types_undeclared: &[SymbolKind], // Types of symbols to ignore during undeclared symbol checking
        skip_symbols_unused: &[SymbolKind],   // Symbols to ignore during unused symbol checking
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, ParserInternalError> {
        let mut checked = true;



        // Check declared symbols in the annotated syntax tree
        // This check ensures that declared symbols follow the correct syntax and declarations
        checked &= semantic::checks::check_declared_symbols(&context, diagnostic_manager)?;

        // Check for undeclared symbols, skipping specific types of symbols
        // This ensures that all symbols used in the tree are declared, except for those types in
        // `skip_types_undeclared`
        checked &= semantic::checks::check_undeclared_symbols(
            context,
            skip_types_undeclared, // Skip certain symbol types for undeclared checking
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // Check for unused symbols, skipping specific symbols
        // This ensures that no declared symbols are unused, except for those in `skip_symbols_unused`
        checked &= semantic::checks::check_unused_symbols(
            context,
            skip_symbols_unused, // Skip certain symbols for unused checking
            Provider::Analyzer,
            diagnostic_manager,

        )?;

        // Return the result indicating whether all checks passed
        Ok(checked)
    }
}
