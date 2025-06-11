use crate::aiplan4rust::diagnostic::{DiagnosticManager, Severity, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::syntax::ast::Ast;
use crate::aiplan4rust::semantic::TypeChecker;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::AnalyzerResult;
use crate::aiplan4rust::semantic::hir::HirTree;

use std::mem;
use crate::aiplan4rust::semantic::normalization::{normalize_type_declarations, normalize_typed_list};
use crate::aiplan4rust::semantic;

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

    /// Analyzes the given `SyntaxTree` and performs semantic checks.
    ///
    /// This function takes a `SyntaxTree` as input and processes it to produce an
    /// `AnnotatedSyntaxTree`, which contains additional semantic information.
    /// It also performs error detection and collects any encountered issues.
    ///
    /// # Arguments
    ///
    /// * `ast` - A reference to the `SyntaxTree` to be analyzed.
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
    /// - If the `ast` cannot be converted into an `AnnotatedSyntaxTree`.
    /// - If the AST type is not recognized (`AstKind::Domain` or `AstKind::Problem` expected).
    ///
    /// # Example
    ///
    /// ```rust
    /// let ast = SyntaxTree::new();
    /// let mut analyzer = Analyzer::new();
    ///
    /// match analyzer.analyze(&ast) {
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
    /// - The function assumes that the `ast` has already been parsed
    ///   and that it contains a valid AST representation.
    pub fn analyze(
        &mut self,
        syntax_tree: &Ast,
    ) -> Result<AnalyzerResult, ParserInternalError> {
        // Create the `AnnotatedSyntaxTree` using the dedicated `from` function
        let mut annotated_syntax_tree = HirTree::from(syntax_tree)?;

        // Determine the AST kind and perform the appropriate checks
        match syntax_tree.root().kind() {
            AstKind::Domain => {
                Self::normalize_domain(&mut annotated_syntax_tree, &mut self.diagnostic_manager)?;
                Self::check_domain(&annotated_syntax_tree, &mut self.diagnostic_manager)?
            },
            AstKind::Problem => {
                Self::normalize_problem(&mut annotated_syntax_tree, &mut self.diagnostic_manager)?;
                Self::check_problem(&annotated_syntax_tree, &mut self.diagnostic_manager)?
            },
            _ => {
                return Err(ParserInternalError::new(format!(
                    "Unexpected AST node kind found: {}",
                    syntax_tree.root().kind()
                )));
            }
        };

        // If no parsing errors occurred, return the annotated syntax tree
        if !self.diagnostic_manager.has_diagnotics_of_severity(Severity::Error) {
            Ok(AnalyzerResult::new(
                Some(annotated_syntax_tree),
                mem::take(&mut self.diagnostic_manager),
            ))
        } else {
            // Otherwise, return an empty result with collected errors
            Ok(AnalyzerResult::new(
                None,
                mem::take(&mut self.diagnostic_manager),
            ))
        }
    }

    /// Normalizes the domain by applying various normalization passes on the syntax tree.
    ///
    /// This function orchestrates domain normalization by sequentially invoking specific normalization
    /// routines such as merging duplicated type declarations and normalizing typed lists. It ensures
    /// that the syntax tree is updated accordingly and emits any relevant diagnostics via the
    /// provided `DiagnosticManager`.
    ///
    /// # Parameters
    /// - `ast`: A mutable reference to the `AnnotatedSyntaxTree` representing the domain
    ///   to be normalized. This tree may be mutated during normalization.
    /// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` used to collect
    ///   and report any diagnostics generated during normalization.
    ///
    /// # Returns
    /// Returns `Ok(true)` if any normalization step modified the syntax tree (i.e., the domain
    /// was changed). Returns `Ok(false)` if no changes were made. Returns an error if any
    /// internal error occurs during normalization.
    ///
    /// # Errors
    /// Propagates errors from underlying normalization functions, typically
    /// [`ParserInternalError`] if unexpected conditions arise.
    ///
    /// # Behavior
    /// 1. Normalizes type declarations by merging duplicates and emitting warnings.
    /// 2. Normalizes typed lists and emits related diagnostics.
    /// 3. Combines the results of both steps to indicate if any changes were made.
    ///
    /// # See Also
    /// - [`normalize_type_declarations`]
    /// - [`normalize_typed_list`]
    fn normalize_domain(
        syntax_tree: &mut HirTree,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, ParserInternalError> {
        // Normalize primitive type declarations and merge duplicates; track if changed
        let mut changed = normalize_type_declarations(syntax_tree, diagnostic_manager)?;

        // Normalize typed lists and combine with previous changed flag
        changed &= normalize_typed_list(syntax_tree, diagnostic_manager)?;

        // Return true if any normalization was performed, false otherwise
        Ok(changed)
    }

    /// Normalizes the problem domain by applying normalization passes excluding type hierarchy normalization.
    ///
    /// This function performs normalization on components of the domain such as typed lists, while
    /// explicitly skipping normalization of primitive type declarations and their hierarchy. This is useful
    /// when you want to normalize certain parts of the domain without merging or modifying the type inheritance.
    ///
    /// # Parameters
    /// - `ast`: A mutable reference to the `AnnotatedSyntaxTree` representing the problem domain.
    /// - `diagnostic_manager`: A mutable reference to the `DiagnosticManager` to collect and report diagnostics.
    ///
    /// # Returns
    /// Returns:
    /// - `Ok(true)` if the normalization resulted in any changes to the syntax tree (typed lists modified).
    /// - `Ok(false)` if no changes were needed.
    /// - `Err(ParserInternalError)` if an internal error occurs during normalization.
    ///
    /// # Behavior
    /// - Only normalizes typed lists and emits diagnostics if needed.
    /// - Does NOT modify or merge primitive type declarations or the type hierarchy.
    ///
    /// # See Also
    /// - [`normalize_typed_list`]: for normalization of typed lists.
    /// - [`normalize_type_declarations`]: for full normalization including type hierarchy.
    pub fn normalize_problem(
        syntax_tree: &mut HirTree,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, ParserInternalError> {
        // Apply normalization on typed lists only, returning whether any change occurred.
        normalize_typed_list(syntax_tree, diagnostic_manager)
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
        annotated_syntax_tree: &HirTree,
        diagnostic_manager: &mut DiagnosticManager
    ) -> Result<bool, ParserInternalError> {
        // Skip unused symbols of kind Constant during the checks
        let skip_symbols_unused = &[SymbolKind::Constant];

        // Perform the first symbol check (declared symbols check)
        let mut checked= Self::check_symbols(
            annotated_syntax_tree,
            &[],                 // No symbols to skip for declared symbols check
            skip_symbols_unused, // Skip symbols of type Constant for unused symbol check
            diagnostic_manager,
        )?;

        checked &= checked && semantic::checks::check_type_hierarchy(
            annotated_syntax_tree,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // If the symbol check passes without errors, proceed with further checks
        if checked {


            // Create a type checker using the symbol table from the annotated syntax tree
            let type_checker = TypeChecker::new(annotated_syntax_tree.symbol_table());

            // Check atomic formulas in the domain using the type checker
            checked &= semantic::checks::check_declared_symbol_signatures(
                annotated_syntax_tree,
                &type_checker,
                diagnostic_manager,
            )?;

            // Check functional expressions in the domain using the type checker
            checked &= semantic::checks::check_typed_expressions(
                annotated_syntax_tree,
                &type_checker,
                Provider::Analyzer,
                diagnostic_manager,
            )?;

            checked &= semantic::checks::check_task_ordering(
                annotated_syntax_tree,
                Provider::Analyzer,
                diagnostic_manager
            )?;

            semantic::checks::check_requirement_violations(
                annotated_syntax_tree,
                annotated_syntax_tree.requirements(),
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
        annotated_syntax_tree: &HirTree,
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
            annotated_syntax_tree,
            skip_types_undeclared,
            &[],
            diagnostic_manager,
        )?;

        checked &= semantic::checks::check_task_ordering(
            annotated_syntax_tree,
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
        annotated_syntax_tree: &HirTree,
        skip_types_undeclared: &[SymbolKind], // Types of symbols to ignore during undeclared symbol checking
        skip_symbols_unused: &[SymbolKind],   // Symbols to ignore during unused symbol checking
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, ParserInternalError> {
        let mut checked = true;

        // Check declared symbols in the annotated syntax tree
        // This check ensures that declared symbols follow the correct syntax and declarations
        checked &= semantic::checks::check_declared_symbols(&annotated_syntax_tree, diagnostic_manager)?;

        // Check for undeclared symbols, skipping specific types of symbols
        // This ensures that all symbols used in the tree are declared, except for those types in
        // `skip_types_undeclared`
        checked &= semantic::checks::check_undeclared_symbols(
            annotated_syntax_tree,
            skip_types_undeclared, // Skip certain symbol types for undeclared checking
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // Check for unused symbols, skipping specific symbols
        // This ensures that no declared symbols are unused, except for those in `skip_symbols_unused`
        checked &= semantic::checks::check_unused_symbols(
            annotated_syntax_tree,
            skip_symbols_unused, // Skip certain symbols for unused checking
            Provider::Analyzer,
            diagnostic_manager,

        )?;

        // Return the result indicating whether all checks passed
        Ok(checked)
    }
}
