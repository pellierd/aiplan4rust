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
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::{passes, AnalyzerResult};
use crate::aiplan4rust::semantic::{SemanticContext, SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::{Ast, AstKind};
use std::collections::{HashMap, HashSet};

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
    pub fn perform_analysis(&mut self, ast: &mut Ast) -> Result<AnalyzerResult, SemanticError> {
        // 1. On récupère la racine pour l'aiguillage
        let root_ref = ast.syntax_tree().try_root_node_ref()?;

        // 2. Dispatch vers l'analyse spécifique qui créera le SemanticContext
        // Note : On ajoute le ";" à la fin du match pour l'assignation
        let context = match root_ref.node().kind() {
            AstKind::Domain => self.perform_domain_analysis(ast)?,
            AstKind::Problem => self.perform_problem_analysis(ast)?,
            found => {
                return Err(SemanticError::unexpected_node_kind(
                    root_ref.id(),
                    vec![AstKind::Domain, AstKind::Problem],
                    found,
                ));
            }
        };

        // 3. Détermination du succès/échec via les diagnostics accumulés
        let has_errors = self
            .diagnostic_manager
            .has_diagnostics_of_severity(Severity::Error);

        // 4. Construction du résultat final
        if !has_errors {
            Ok(AnalyzerResult::success(
                context,
                std::mem::take(&mut self.diagnostic_manager),
            ))
        } else {
            // En cas d'échec, on extrait l'interner du contexte pour le rendre au moteur
            Ok(AnalyzerResult::failure(
                std::mem::take(&mut self.diagnostic_manager),
                ast.take_interner(),
            ))
        }
    }

    /// Exécute le pipeline complet d'analyse sémantique pour un Domaine.
    ///
    /// Le flux suit une progression stratégique :
    /// 1. Validation de base (noms, types déclarés).
    /// 2. Optimisation de la SymbolTable et synchronisation de l'AST (Simplification).
    /// 3. Validation avancée sur l'état final optimisé (Signatures, Expressions).
    /// Exécute le pipeline complet d'analyse sémantique pour un Domaine.
    ///
    /// Le flux suit une progression stratégique :
    /// 1. Validation de la hiérarchie et des déclarations.
    /// 2. Résolution des symboles (vissage) et extraction des requirements.
    /// 3. Optimisation de la SymbolTable.
    /// 4. Validation avancée (expressions typées, signatures).
    pub fn perform_domain_analysis(
        &mut self,
        ast: &mut Ast,
    ) -> Result<SemanticContext, SemanticError> {
        // =========================================================================
        // 0. CONTEXT INITIALIZATION
        // =========================================================================
        // We create the PassContext early as a unified tool provider for all passes.
        let pass_ctx = PassContext::new(
            ast.syntax_tree(),
            ast.interner(),
            ast.source_id(),
            Provider::Analyzer,
        );

        // =========================================================================
        // 1. INITIAL EXTRACTION
        // =========================================================================
        // Extract the base symbol table and identify requirements declared by the user.
        let mut symbol_table = passes::extract_symbol_table(&pass_ctx)?;
        let declared_reqs = passes::extract_declared_requirements(&pass_ctx)?;

        // CheckContext is initialized with declared requirements to validate permissions.
        let check_ctx = CheckContext::new(
            ast.syntax_tree(),
            ast.interner(),
            ast.source_id(),
            Provider::Analyzer,
            &declared_reqs,
        );

        // =========================================================================
        // 2. STRUCTURAL VALIDATION (Fail-Fast)
        // =========================================================================
        // Validate the type hierarchy and symbol declarations before complex resolution.
        let can_continue = semantic::checks::check_type_hierarchy(
            &check_ctx,
            &mut symbol_table,
            &mut self.diagnostic_manager,
        )? && semantic::checks::check_symbol_declarations(
            &check_ctx,
            &mut symbol_table,
            &mut self.diagnostic_manager,
        )?;

        // Containers for data discovered during deep analysis.
        let mut inferred_required = HashSet::new();
        let mut requirement_triggers = HashMap::new();

        // =========================================================================
        // 3. RESOLUTION & DEEP SEMANTIC ANALYSIS
        // =========================================================================
        if can_continue {
            let type_hierarchy = symbol_table.to_type_hierarchy();
            let type_checker = TypeChecker::new(&type_hierarchy);

            // --- BINDING PASSES ---
            // Resolve symbols and derived predicates using the unified PassContext.
            passes::resolve_symbols(&pass_ctx, &mut symbol_table, Some(&type_checker), None)?;
            passes::resolve_derived_predicates(
                &pass_ctx,
                &mut symbol_table,
                Some(&type_checker),
                None,
            )?;

            // --- POST-RESOLUTION CHECKS ---
            // Verify usage, types, and ordering constraints on resolved symbols.
            semantic::checks::check_symbol_usage(
                &check_ctx,
                &symbol_table,
                &[],
                &mut self.diagnostic_manager,
            )?;
            semantic::checks::check_typed_expressions(
                &check_ctx,
                &mut symbol_table,
                &type_checker,
                &mut self.diagnostic_manager,
            )?;
            semantic::checks::check_task_ordering(&check_ctx, &mut self.diagnostic_manager)?;

            // --- REQUIREMENT INFERENCE ---
            // Detect which PDDL features are actually used in the domain.
            inferred_required = passes::extract_required_requirements(
                &pass_ctx,
                &symbol_table,
                &mut requirement_triggers,
            )?;

            // Validate that used features match the declared requirements.
            semantic::checks::check_requirements(
                &check_ctx,
                &requirement_triggers,
                &mut self.diagnostic_manager,
            )?;
        }

        // =========================================================================
        // 4. FINAL CONTEXT PACKING
        // =========================================================================
        // Transfer ownership of the SyntaxTree and Interner to the SemanticContext.
        // We return a context even if can_continue was false to provide IDE feedback.
        let mut context = SemanticContext::new(
            ast.take_syntax_tree(),
            ast.source_id(),
            symbol_table,
            ast.take_interner(),
            std::time::SystemTime::now(),
        )?;

        context.set_required_requirements(inferred_required);
        context.set_requirement_triggers(requirement_triggers);

        Ok(context)
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
    pub fn perform_problem_analysis(
        &mut self,
        ast: &mut Ast,
    ) -> Result<SemanticContext, SemanticError> {
        // =========================================================================
        // 0. CONTEXT INITIALIZATION
        // =========================================================================
        // Create the PassContext as the unified tool provider for the problem file.
        let pass_ctx = PassContext::new(
            ast.syntax_tree(),
            ast.interner(),
            ast.source_id(),
            Provider::Analyzer,
        );

        // =========================================================================
        // 1. INITIAL EXTRACTION & CONTEXT SETUP
        // =========================================================================
        // Extract the initial symbol table (objects, initial state, goal, etc.).
        let mut symbol_table = passes::extract_symbol_table(&pass_ctx)?;

        // Problems inherit requirements from the Domain, so we use an empty set here
        // to maintain CheckContext consistency without forcing local declarations.
        let declared_reqs = HashSet::new();

        let check_ctx = CheckContext::new(
            ast.syntax_tree(),
            ast.interner(),
            ast.source_id(),
            Provider::Analyzer,
            &declared_reqs,
        );

        // =========================================================================
        // 2. RESOLUTION & BINDING
        // =========================================================================
        // Resolve local symbols (objects and problem-specific variables).
        // Note: Linking with Domain types/constants is a separate later phase.
        passes::resolve_symbols(&pass_ctx, &mut symbol_table, None, None)?;

        // =========================================================================
        // 3. SEMANTIC VALIDATIONS
        // =========================================================================
        // Initial structural checks on problem-level declarations.
        let mut can_continue = semantic::checks::check_symbol_declarations(
            &check_ctx,
            &mut symbol_table,
            &mut self.diagnostic_manager,
        )?;

        // Symbols expected to be in the Domain; we skip "undefined" errors for these
        // during the standalone problem analysis phase.
        let external_symbols = &[
            SymbolKind::PrimitiveType,
            SymbolKind::Constant,
            SymbolKind::Predicate,
            SymbolKind::Function,
            SymbolKind::Task,
        ];

        // Check symbol usage, ignoring those defined externally in the Domain.
        can_continue &= semantic::checks::check_symbol_usage(
            &check_ctx,
            &symbol_table,
            external_symbols,
            &mut self.diagnostic_manager,
        )?;

        // Detect objects that are declared but never referenced in Init or Goal.
        can_continue &= semantic::checks::check_unused_symbols(
            &check_ctx,
            &mut symbol_table,
            &[],
            &mut self.diagnostic_manager,
        )?;

        // Perform specific task ordering validations (e.g., for HTN problems).
        can_continue &=
            semantic::checks::check_task_ordering(&check_ctx, &mut self.diagnostic_manager)?;

        // =========================================================================
        // 4. FINAL PACKING
        // =========================================================================
        // Metadata containers, usually populated during the Domain-Problem linking.
        let inferred_required = HashSet::new();
        let requirement_triggers = HashMap::new();

        // Finalize by moving the AST and Interner into the SemanticContext.
        let mut context = SemanticContext::new(
            ast.take_syntax_tree(),
            ast.source_id(),
            symbol_table,
            ast.take_interner(),
            std::time::SystemTime::now(),
        )?;

        context.set_required_requirements(inferred_required);
        context.set_requirement_triggers(requirement_triggers);

        Ok(context)
    }
}
