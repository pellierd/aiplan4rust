//! # Semantic Analyzer Module
//!
//! This module provides the `Analyzer` struct and related functionality to perform semantic analysis
//! on abstract syntax trees (ASTs) generated from parsing AI syntax domain and problem specifications.
//!
//! ## Overview
//!
//! The `Analyzer` is responsible for:
//! - Validating the semantic correctness of the input AST.
//! - Detecting and reporting semantic errors such as undeclared symbols, unused symbols, typing mismatches,
//!   and task ordering violations.
//! - Managing diagnostics (errors, warnings, and informational messages) during analysis.
//!
//! The analysis supports two primary root AST kinds:
//! - `Domain`: checks related to domain specifications (e.g., symbol declarations, typing hierarchies, logic).
//! - `Problem`: checks related to problem instances within a domain (e.g., symbol usage, task ordering).
//!
//! ## Main Types
//!
//! - [`Analyzer`]: The main struct that performs semantic analysis and collects diagnostics.
//! - [`DiagnosticManager`]: Manages diagnostics such as errors and warnings.
//! - [`SemanticContext`]: Represents the annotated semantic information for an AST.
//! - [`Result`]: Contains the outcome of the analysis including semantic context and diagnostics.
//!
//! ## Usage Example
//!
//! ```rust
//! use crate::aiplan4rust::analyzer::Analyzer;
//! use crate::aiplan4rust::syntax::ast::Ast;
//!
//! let mut ast = Ast::parse("...domain and problem source...")?;
//! let mut analyzer = Analyzer::new();
//!
//! match analyzer.analyze(&mut ast) {
//!     Ok(result) => {
//!         if let Some(semantic_ctx) = result.semantic_context() {
//!             println!("Semantic analysis succeeded.");
//!         } else {
//!             println!("Semantic errors were found.");
//!         }
//!         // Diagnostics can be inspected via result.diagnostics()
//!     }
//!     Err(err) => eprintln!("Failed to analyze AST: {}", err),
//! }
//! ```
//!
//! ## Notes
//!
//! - The analyzer may mutate the AST during analysis, consuming internal resources such as string interners
//!   to improve efficiency.
//! - Diagnostic information is collected throughout the analysis and can be retrieved after analysis completes.
//! - Custom diagnostic managers can be injected to collect or customize diagnostic handling.
//!
//! ## Error Handling
//!
//! Semantic errors are returned as variants of [`SemanticError`]. These may include unexpected AST node kinds,
//! typing errors, symbol resolution errors, and other domain-specific semantic validation failures.

use crate::aiplan4rust::diagnostic::{DiagnosticManager, Provider, Severity};
use crate::aiplan4rust::normalization::NormalizerResult;
use crate::aiplan4rust::semantic;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::type_checker::TypeHierarchy;
use crate::aiplan4rust::semantic::{finalization, AnalyzerResult};
use crate::aiplan4rust::semantic::{SemanticContext, SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};

/// The `Analyzer` struct is responsible for performing semantic analysis on a `SyntaxTree`.
///
/// It manages the detection and collection of errors encountered during the analysis process.
/// The `diagnostic_manager` field stores all diagnostics, including errors, warnings,
/// and informational messages that occur during semantic analysis.
///
/// # Example
/// ```rust
/// let mut analyzer = Analyzer::new();
/// let mut ast = ...; // obtain or build the AST
/// let result = analyzer.analyze(&mut ast);
/// match result {
///     Ok(analyzer_result) => {
///         if analyzer_result.is_some() {
///             println!("Semantic analysis succeeded.");
///         } else {
///             println!("Semantic errors were found.");
///         }
///     }
///     Err(err) => eprintln!("Failed to analyze: {}", err),
/// }
/// ```
#[derive(Debug)]
pub struct Analyzer {
    /// Manages and tracks parsing and semantic diagnostics encountered during analysis.
    diagnostic_manager: DiagnosticManager,
}

impl Analyzer {
    /// Creates a new instance of `Analyzer`.
    ///
    /// Initializes a fresh `DiagnosticManager` to collect diagnostics during analysis.
    ///
    /// # Returns
    ///
    /// An `Analyzer` ready to perform semantic checks.
    ///
    /// # Example
    ///
    /// ```rust
    /// let analyzer = Analyzer::new();
    /// ```
    pub fn new() -> Self {
        Self {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the internal `DiagnosticManager`.
    ///
    /// This allows inspection of collected diagnostics (errors, warnings, infos) after analysis.
    ///
    /// # Returns
    ///
    /// A reference to the `DiagnosticManager`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let analyzer = Analyzer::new();
    /// let diagnostics = analyzer.diagnostic_manager();
    /// for diagnostic in diagnostics.diagnostics() {
    ///     println!("{}", diagnostic);
    /// }
    /// ```
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Performs semantic analysis on the AST contained within a `NormalizerResult`.
    ///
    /// This method consumes or borrows parts of the `NormalizerResult` (e.g., normalized AST,
    /// diagnostic manager, interner) as needed for efficient semantic analysis.
    ///
    /// # Parameters
    /// - `normalizer_result`: The result of logic containing the AST and diagnostics.
    ///
    /// # Returns
    ///
    /// On success, returns an `AnalyzerResult` encapsulating the semantic context and diagnostics.
    /// On failure, returns a `SemanticError`.
    ///
    /// # Note
    ///
    /// The AST inside the `NormalizerResult` may be mutated during analysis, and internal structures may be consumed.
    pub fn analyze(
        &mut self,
        normalizer_result: &mut NormalizerResult,
    ) -> Result<AnalyzerResult, SemanticError> {
        // Match on the normalized AST to decide how to continue.
        match normalizer_result.take_ast() {
            Some(mut ast) => {
                // Take diagnostics accumulated during logic.
                let diagnostic_manager = normalizer_result.take_diagnostic_manager();
                self.diagnostic_manager
                    .add_diagnostic_from(diagnostic_manager);

                // Analyze the normalized AST.
                let analysis_result = self.perform_analysis(&mut ast)?;

                // Return the final analysis result.
                Ok(analysis_result)
            }
            None => {
                // If no AST, propagate diagnostics and interner to AnalyzerResult.
                let diagnostic_manager = normalizer_result.take_diagnostic_manager();
                let interner = normalizer_result.take_interner();
                Ok(AnalyzerResult::failure(diagnostic_manager, interner))
            }
        }
    }

    /// Internal method to perform semantic analysis on the AST.
    ///
    /// Converts the AST into a `SemanticContext` and dispatches checks depending on
    /// whether the root node represents a `Domain` or a `Problem`.
    ///
    /// # Parameters
    /// - `ast`: Mutable reference to the AST.
    ///
    /// # Returns
    ///
    /// Returns an `AnalyzerResult` with the semantic context if no errors, or none if errors are present.
    ///
    /// # Errors
    ///
    /// Returns a `SemanticError` if the root node kind is not supported.
    fn perform_analysis(&mut self, ast: &mut Ast) -> Result<AnalyzerResult, SemanticError> {
        // Build semantic context from AST
        let mut context = SemanticContext::try_from(ast)?;

        // Determine root kind and run appropriate checks
        let root_ref = context.syntax_tree().try_root_node_ref()?;
        match root_ref.node().kind() {
            AstKind::Domain => {
                Self::check_domain(&mut context, &mut self.diagnostic_manager)?;
                finalization::finalize(&mut context)?;
            }
            AstKind::Problem => {
                let check_ctx = CheckContext::from(&context);
                Self::check_problem(&check_ctx, &mut self.diagnostic_manager)?;
            }
            found => {
                return Err(SemanticError::unexpected_node_kind(
                    root_ref.id(),
                    vec![AstKind::Domain, AstKind::Problem],
                    found,
                ));
            }
        }

        // Build the AnalyzerResult based on presence of errors
        if !self
            .diagnostic_manager
            .has_diagnostics_of_severity(Severity::Error)
        {
            Ok(AnalyzerResult::success(
                context,
                std::mem::take(&mut self.diagnostic_manager),
            ))
        } else {
            Ok(AnalyzerResult::failure(
                std::mem::take(&mut self.diagnostic_manager),
                context.take_interner(),
            ))
        }
    }

    /// Performs a multi-phase semantic validation of the domain within the syntax arena.
    ///
    /// This function orchestrates the domain validation process by splitting it into three
    /// distinct logical phases to comply with Rust's borrowing rules while allowing
    /// for symbol table optimization.
    ///
    /// ### Validation Phases:
    /// 0. **Extraction**: A [`TypeHierarchy`] is generated once from the initial
    ///    [`SymbolTable`]. This immutable index is shared across all subsequent steps
    ///    to ensure consistency and $O(1)$ lookup performance.
    /// 1. **Base Validation**: Initial integrity checks (declarations, requirements, etc.)
    ///    using the pre-computed hierarchy and an immutable snapshot of the context.
    /// 2. **Type Simplification (Mutation)**: If base checks pass, the `SymbolTable` is
    ///    mutated to simplify type unions (e.g., removing redundant ancestors)
    ///    using the [`TypeChecker`] and the existing hierarchy.
    /// 3. **Advanced Validation**: Final semantic checks (atomic formulas, task ordering, etc.)
    ///    performed on the optimized symbol table.
    ///
    /// # Parameters
    /// - `context`: The [`SemanticContext`] providing access to the symbol table and domain data.
    /// - `diagnostic_manager`: A mutable reference to collect and report semantic errors or warnings.
    ///
    /// # Returns
    /// - `Ok(true)` if all validation phases complete successfully.
    /// - `Ok(false)` if any semantic violation is detected (diagnostics will be populated).
    /// - `Err(SemanticError)` if an unrecoverable internal error occurs during processing.
    ///
    /// # Technical Note: Borrowing Strategy
    /// To allow the mutation of the `SymbolTable` between two validation steps, the [`CheckContext`]
    /// is scoped within a block in Step 1. This ensures that any immutable borrows of the context
    /// are dropped before calling `context.symbol_table_mut()` in Step 2. The [`TypeHierarchy`]
    /// remains valid throughout as the simplification process does not alter the structural
    /// relationships between types.
    fn check_domain(
        context: &mut SemanticContext,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        // --- ÉTAPE 0 : EXTRACTION ---
        // Generate the type hierarchy once. This becomes the static reference for all checks.
        let type_hierarchy = context.symbol_table().to_type_hierarchy();

        // --- STEP 1: BASE CHECK ---
        // Scoped block to ensure CheckContext (immutable borrow) is dropped before mutation.
        let mut checked = {
            let check_ctx = CheckContext::from(&*context);
            Self::check_domain_base(&check_ctx, &type_hierarchy, diagnostic_manager)?
        };

        if checked {
            // --- STEP 2: SYMBOL TABLE OPTIMIZATION ---
            // Accessing mutable symbol table is now safe as check_ctx is out of scope.
            let table = context.symbol_table_mut();

            // Perform type union simplification using the pre-extracted hierarchy.
            let type_checker = TypeChecker::new(&type_hierarchy);

            type_checker.simplify_symbol_table(table)?;

            // --- STEP 3: ADVANCED CHECK ---
            // Re-create a fresh CheckContext to reflect the simplified symbol table.
            let check_ctx = CheckContext::from(&*context);
            checked &= Self::check_domain_advanced(&check_ctx, &type_checker, diagnostic_manager)?;
        }

        Ok(checked)
    }

    /// Performs the initial fundamental semantic checks on the domain.
    ///
    /// This function acts as the "first pass" of the validation process. It focuses on
    /// ensuring that the core components of the domain—symbols, types, and their
    /// immediate relationships—are well-defined and consistent.
    ///
    /// By receiving a pre-computed [`TypeHierarchy`], this function can perform type
    /// existence checks and structural validations without redundant table scans.
    ///
    /// ### Checks Performed:
    /// 1. **Symbol Declaration**: Validates that domain symbols (specifically constants)
    ///    are correctly declared and do not violate naming or scoping rules.
    /// 2. **Type Existence**: Ensures that every type referenced (by variables, constants,
    ///    or functions) has a corresponding declaration, using the hierarchy for $O(1)$ lookups.
    /// 3. **Type Hierarchy Integrity**: Verifies the structural validity of the type
    ///    hierarchy (e.g., checking for cycles or invalid parent-child relationships)
    ///    using the `Analyzer` provider.
    ///
    /// # Parameters
    /// - `context`: The [`CheckContext`] containing the immutable snapshot of the current domain state.
    /// - `type_hierarchy`: The pre-computed [`TypeHierarchy`] used to validate type references.
    /// - `diagnostic_manager`: A mutable reference used to record any detected semantic violations.
    ///
    /// # Returns
    /// - `Ok(true)` if all base checks pass.
    /// - `Ok(false)` if any fundamental error is found (e.g., an undefined type or a cyclic hierarchy).
    /// - `Err(SemanticError)` if an unexpected internal error occurs during validation.
    fn check_domain_base(
        context: &CheckContext,
        type_hierarchy: &TypeHierarchy,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        // 1. Validate basic symbol declarations (starting with Constants).
        let mut checked =
            Self::check_symbols(context, &[], &[SymbolKind::Constant], diagnostic_manager)?;

        // 2. Verify that ALL types used across the domain are properly declared.
        // This catches "ghost types" by comparing references against the established hierarchy.
        checked &=
            semantic::checks::check_symbol_types(context, type_hierarchy, diagnostic_manager)?;

        // 3. Structural validation of the type tree/graph.
        checked &= semantic::checks::check_type_hierarchy(
            context,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }

    /// Performs advanced semantic validation using the optimized type hierarchy.
    ///
    /// This function executes the "second pass" of the validation process. It relies on the
    /// [`TypeChecker`] and the simplified [`SymbolTable`] to perform complex analysis
    /// on expressions, signatures, and domain logic that require a finalized type system.
    ///
    /// ### Checks Performed:
    /// 1. **Symbol Signatures**: Validates that predicates and functions are used with
    ///    arguments that match their declared type constraints (variance/covariance).
    /// 2. **Typed Expressions**: Deep analysis of the expression tree (Arena-based)
    ///    to ensure that nested terms and logical operators are type-consistent.
    /// 3. **Task Ordering**: Checks the validity of hierarchical or sequential constraints
    ///    within actions and tasks (e.g., in HTN or temporal PDDL).
    /// 4. **Requirement Violations**: Verifies that the features used in the domain
    ///    (e.g., `:typing`, `:fluents`) are explicitly declared in the `:requirements` section.
    ///
    /// # Parameters
    /// - `context`: The [`CheckContext`] reflecting the optimized state of the domain.
    /// - `type_checker`: The [`TypeChecker`] instance used for subtyping and closure lookups.
    /// - `diagnostic_manager`: A mutable reference to record semantic errors or warnings.
    ///
    /// # Returns
    /// - `Ok(true)` if all advanced semantic checks pass.
    /// - `Ok(false)` if any violation is detected (e.g., type mismatch in a predicate call).
    /// - `Err(SemanticError)` if an internal error occurs during the analysis.
    ///
    /// # Note
    /// This function should only be called after [`check_domain_base`] and the type
    /// simplification phase have completed successfully.
    fn check_domain_advanced(
        context: &CheckContext,
        type_checker: &TypeChecker,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        let mut checked = true;

        // 1. Verify that all calls to predicates/functions respect their type signatures.
        checked &=
            semantic::checks::check_symbol_signatures(context, type_checker, diagnostic_manager)?;

        // 2. Perform deep type checking on the expression Arena (AST).
        checked &= semantic::checks::check_typed_expressions(
            context,
            &type_checker,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // 3. Validate structural ordering and task dependencies.
        checked &=
            semantic::checks::check_task_ordering(context, Provider::Analyzer, diagnostic_manager)?;

        // 4. Ensure no undeclared PDDL requirements are being used.
        // This is a post-check that doesn't necessarily block 'checked' but reports errors.
        semantic::checks::check_requirement_violations(
            context,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }

    /// Checks the problem part of the syntax arena with problem-specific semantic validations.
    ///
    /// It primarily checks symbols excluding some types and verifies task ordering.
    ///
    /// # Parameters
    /// - `context`: The `CheckContext` derived from the semantic context.
    /// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` to collect diagnostics.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if checks pass, `Ok(false)` if errors are found,
    /// or `Err(SemanticError)` if internal errors occur.
    fn check_problem(
        context: &CheckContext,
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        // Skip these kinds during undeclared symbol check in problems
        let skip_types_undeclared = &[
            SymbolKind::PrimitiveType,
            SymbolKind::Constant,
            SymbolKind::Predicate,
            SymbolKind::Function,
            SymbolKind::Task,
        ];

        let mut checked =
            Self::check_symbols(context, skip_types_undeclared, &[], diagnostic_manager)?;

        checked &=
            semantic::checks::check_task_ordering(context, Provider::Analyzer, diagnostic_manager)?;

        Ok(checked)
    }

    /// Performs general symbol checks: declared, undeclared, and unused symbols.
    ///
    /// This method verifies that:
    /// - All declared symbols are valid.
    /// - No undeclared symbols are used (except those specified to skip).
    /// - No symbols are unused (except those specified to skip).
    ///
    /// # Parameters
    /// - `context`: The `CheckContext` representing the semantic context.
    /// - `skip_types_undeclared`: Symbol kinds to ignore during undeclared symbol checking.
    /// - `skip_symbols_unused`: Symbol kinds to ignore during unused symbol checking.
    /// - `diagnostic_manager`: Mutable reference to the `DiagnosticManager` for diagnostics.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if all checks succeed without errors,
    /// `Ok(false)` if some checks fail,
    /// or `Err(SemanticError)` if an internal error occurs.
    ///
    /// # Example
    /// ```rust
    /// let mut diagnostic_manager = DiagnosticManager::new();
    /// let result = Analyzer::check_symbols(&check_ctx, &[], &[], &mut diagnostic_manager);
    /// ```
    pub fn check_symbols(
        context: &CheckContext,
        skip_types_undeclared: &[SymbolKind],
        skip_symbols_unused: &[SymbolKind],
        diagnostic_manager: &mut DiagnosticManager,
    ) -> Result<bool, SemanticError> {
        let mut checked = true;

        // Verify declared symbols correctness
        checked &= semantic::checks::check_symbol_declarations(context, diagnostic_manager)?;

        // Check undeclared symbols, skipping specified types
        checked &= semantic::checks::check_undeclared_symbols(
            context,
            skip_types_undeclared,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        // Check for unused symbols, skipping specified symbols
        checked &= semantic::checks::check_unused_symbols(
            context,
            skip_symbols_unused,
            Provider::Analyzer,
            diagnostic_manager,
        )?;

        Ok(checked)
    }
}
