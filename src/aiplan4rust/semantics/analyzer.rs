use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::ParserErrorKind::ParseError;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::analyser_result::AnalyzerResult;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::checkers::undeclared_symbol_checker;
use crate::aiplan4rust::semantics::checkers::TypeChecker;
use crate::aiplan4rust::semantics::checkers::{
    atomic_formula_checker, functional_expression_checker,
};
use crate::aiplan4rust::semantics::checkers::{symbol_declaration_checker, unused_symbol_checker};
use crate::aiplan4rust::semantics::symbol::SymbolKind;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::syntax_tree::SyntaxTree;
use std::mem;

/// The `Analyzer` struct is responsible for performing semantic analysis on a `SyntaxTree`.
///
/// It manages the detection and collection of errors encountered during the analysis process.
/// The `error_manager` field stores any errors found while analyzing the syntax tree.
#[derive(Debug)]
pub struct Analyzer {
    /// Manages and tracks parsing and semantic errors encountered during analysis.
    error_manager: ErrorManager,
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
            error_manager: ErrorManager::new(),
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
    pub fn error_manager(&self) -> &ErrorManager {
        &self.error_manager
    }

    /// Analyzes the given `SyntaxTree` and performs semantic checks.
    ///
    /// This function takes a `SyntaxTree` as input and processes it to produce an
    /// `AnnotatedSyntaxTree`, which contains additional semantic information.
    /// It also performs error detection and collects any encountered issues.
    ///
    /// # Arguments
    ///
    /// * `syntax_tree` - A reference to the `SyntaxTree` to be analyzed.
    ///
    /// # Returns
    ///
    /// Returns a `Result<AnalyzerResult, ParserInternalError>`, where:
    /// - `Ok(AnalyzerResult)` contains the annotated syntax tree if no critical errors were found.
    /// - `Err(ParserInternalError)` is returned if an internal error occurs during analysis.
    ///
    /// # Errors
    ///
    /// This function can return a `ParserInternalError` in the following cases:
    /// - If the `syntax_tree` cannot be converted into an `AnnotatedSyntaxTree`.
    /// - If the AST type is not recognized (`AstKind::Domain` or `AstKind::Problem` expected).
    ///
    /// # Example
    ///
    /// ```rust
    /// let syntax_tree = SyntaxTree::new();
    /// let mut analyzer = Analyzer::new();
    ///
    /// match analyzer.analyze(&syntax_tree) {
    ///     Ok(result) => {
    ///         if let Some(annotated_tree) = result.annotated_tree() {
    ///             println!("Analysis successful: {:?}", annotated_tree);
    ///         } else {
    ///             println!("Analysis completed with warnings.");
    ///         }
    ///     }
    ///     Err(error) => eprintln!("Error during analysis: {}", error),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - If semantic errors are found but they do not prevent further processing,
    ///   the function will return an `AnalyzerResult` with `None` for the annotated tree,
    ///   but still provide the collected errors.
    /// - The function assumes that the `syntax_tree` has already been parsed
    ///   and that it contains a valid AST representation.
    pub fn analyze(
        &mut self,
        syntax_tree: &SyntaxTree,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        // Create the `AnnotatedSyntaxTree` using the dedicated `from` function
        let annotated_syntax_tree = AnnotatedSyntaxTree::from(syntax_tree)?;

        // Determine the AST kind and perform the appropriate checks
        match syntax_tree.ast().kind() {
            AstKind::Domain => self.check_domain(&annotated_syntax_tree)?,
            AstKind::Problem => self.check_problem(&annotated_syntax_tree)?,
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST node kind found: {}",
                    syntax_tree.ast().kind()
                )));
            }
        };

        // If no parsing errors occurred, return the annotated syntax tree
        if !self.error_manager.has_errors_of_kind(ParseError) {
            Ok(AnalyzerResult::new(
                Some(annotated_syntax_tree),
                mem::take(&mut self.error_manager),
            ))
        } else {
            // Otherwise, return an empty result with collected errors
            Ok(AnalyzerResult::new(
                None,
                mem::take(&mut self.error_manager),
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
        &mut self,
        annotated_syntax_tree: &AnnotatedSyntaxTree,
    ) -> Result<bool, ParserInternalError> {
        // Skip unused symbols of kind Constant during the checks
        let skip_symbols_unused = &[SymbolKind::Constant];

        // Perform the first symbol check (declared symbols check)
        let mut checked = Self::check_symbols(
            annotated_syntax_tree,
            &[],                 // No symbols to skip for declared symbols check
            skip_symbols_unused, // Skip symbols of type Constant for unused symbol check
            &mut self.error_manager,
        )?;

        // If the symbol check passes without errors, proceed with further checks
        if checked {
            // Create a type checker using the symbol table from the annotated syntax tree
            let type_checker = TypeChecker::new(annotated_syntax_tree.symbol_table());

            // Check atomic formulas in the domain using the type checker
            checked &= atomic_formula_checker::check(
                annotated_syntax_tree,
                &type_checker,
                &mut self.error_manager,
            )?;

            // Check functional expressions in the domain using the type checker
            checked &= functional_expression_checker::check(
                annotated_syntax_tree,
                &type_checker,
                &mut self.error_manager,
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
        &mut self,
        annotated_syntax_tree: &AnnotatedSyntaxTree,
    ) -> Result<bool, ParserInternalError> {
        let skip_types_undeclared = &[
            SymbolKind::PrimitiveType,
            SymbolKind::Constant,
            SymbolKind::Predicate,
            SymbolKind::Function,
            SymbolKind::Task, // Add for HDDL
        ];

        Ok(Self::check_symbols(
            annotated_syntax_tree,
            skip_types_undeclared,
            &[],
            &mut self.error_manager,
        )?)
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
    fn check_symbols(
        annotated_syntax_tree: &AnnotatedSyntaxTree,
        skip_types_undeclared: &[SymbolKind], // Types of symbols to ignore during undeclared symbol checking
        skip_symbols_unused: &[SymbolKind],   // Symbols to ignore during unused symbol checking
        error_manager: &mut ErrorManager,
    ) -> Result<bool, ParserInternalError> {
        let mut checked = true;

        // Check declared symbols in the annotated syntax tree
        // This check ensures that declared symbols follow the correct syntax and declarations
        checked &= symbol_declaration_checker::check(&annotated_syntax_tree, error_manager)?;

        // Check for undeclared symbols, skipping specific types of symbols
        // This ensures that all symbols used in the tree are declared, except for those types in
        // `skip_types_undeclared`
        checked &= undeclared_symbol_checker::check(
            annotated_syntax_tree,
            skip_types_undeclared, // Skip certain symbol types for undeclared checking
            error_manager,
        )?;

        // Check for unused symbols, skipping specific symbols
        // This ensures that no declared symbols are unused, except for those in `skip_symbols_unused`
        checked &= unused_symbol_checker::check(
            annotated_syntax_tree,
            skip_symbols_unused, // Skip certain symbols for unused checking
            error_manager,
        )?;

        // Return the result indicating whether all checks passed
        Ok(checked)
    }
}
